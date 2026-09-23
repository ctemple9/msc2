import Darwin
import Foundation

private let maxServiceRequestBytes = 64 * 1024

private struct PrivilegedServiceConfiguration {
    let socketPath: String
    let allowedUID: uid_t
    let allowedGID: gid_t
    let approvedRoots: [String]

    init(arguments: [String]) throws {
        var socketPath: String?
        var allowedUID: uid_t?
        var allowedGID: gid_t?
        var roots: [String] = []
        var index = 0

        while index < arguments.count {
            switch arguments[index] {
            case "--service":
                // This is a mode flag, so the loop must inspect the next
                // option instead of skipping it as though it had a value.
                break
            case "--socket-path":
                index += 1
                guard index < arguments.count else { throw ServiceConfigurationError.missingValue("--socket-path") }
                socketPath = arguments[index]
            case "--allowed-uid":
                index += 1
                guard index < arguments.count, let value = UInt32(arguments[index]) else {
                    throw ServiceConfigurationError.invalidValue("--allowed-uid")
                }
                allowedUID = uid_t(value)
            case "--allowed-gid":
                index += 1
                guard index < arguments.count, let value = UInt32(arguments[index]) else {
                    throw ServiceConfigurationError.invalidValue("--allowed-gid")
                }
                allowedGID = gid_t(value)
            case "--approved-root":
                index += 1
                guard index < arguments.count else { throw ServiceConfigurationError.missingValue("--approved-root") }
                roots.append(arguments[index])
            default:
                throw ServiceConfigurationError.unknownArgument(arguments[index])
            }
            index += 1
        }

        guard let socketPath, socketPath.hasPrefix("/") else {
            throw ServiceConfigurationError.missingValue("--socket-path")
        }
        guard let allowedUID else {
            throw ServiceConfigurationError.missingValue("--allowed-uid")
        }
        guard let allowedGID else {
            throw ServiceConfigurationError.missingValue("--allowed-gid")
        }
        guard !roots.isEmpty else {
            throw ServiceConfigurationError.missingValue("--approved-root")
        }

        self.socketPath = socketPath
        self.allowedUID = allowedUID
        self.allowedGID = allowedGID
        self.approvedRoots = try roots.map(Self.canonicalApprovedRoot)
        for (index, root) in approvedRoots.enumerated() {
            if root == "/" {
                throw ServiceConfigurationError.invalidValue("--approved-root cannot be the filesystem root")
            }
            for other in approvedRoots[(index + 1)...] where root == other || root.hasPrefix(other + "/") || other.hasPrefix(root + "/") {
                throw ServiceConfigurationError.invalidValue("approved roots overlap: \(root) and \(other)")
            }
        }
    }

    private static func canonicalApprovedRoot(_ path: String) throws -> String {
        guard path.hasPrefix("/") else {
            throw ServiceConfigurationError.invalidValue("--approved-root must be absolute")
        }
        guard let canonical = canonicalExistingPath(path) else {
            throw ServiceConfigurationError.invalidValue("approved root is missing: \(path)")
        }
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: canonical, isDirectory: &isDirectory), isDirectory.boolValue else {
            throw ServiceConfigurationError.invalidValue("approved root is not a directory: \(path)")
        }
        return canonical
    }

    func validateServerDirectory(_ path: String) -> Result<String, ServerDirectoryValidationError> {
        guard path.hasPrefix("/") else { return .failure(.init("server-directory-must-be-absolute")) }
        guard let canonical = canonicalExistingPath(path) else {
            return .failure(.init("server-directory-path-invalid-or-missing"))
        }
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: canonical, isDirectory: &isDirectory), isDirectory.boolValue else {
            return .failure(.init("server-directory-is-not-a-directory"))
        }
        guard approvedRoots.contains(where: { canonical.hasPrefix($0 + "/") }) else {
            return .failure(.init("server-directory-outside-approved-bedrock-root"))
        }
        var info = stat()
        guard lstat(canonical, &info) == 0,
              info.st_uid == allowedUID,
              info.st_gid == allowedGID else {
            return .failure(.init("server-directory-owner-mismatch"))
        }
        return .success(canonical)
    }
}

private struct ServerDirectoryValidationError: Error {
    let reason: String

    init(_ reason: String) { self.reason = reason }
}

private enum ServiceConfigurationError: LocalizedError {
    case missingValue(String)
    case invalidValue(String)
    case unknownArgument(String)

    var errorDescription: String? {
        switch self {
        case .missingValue(let option): return "missing or invalid value for \(option)"
        case .invalidValue(let reason): return reason
        case .unknownArgument(let argument): return "unknown service argument: \(argument)"
        }
    }
}

private func canonicalExistingPath(_ path: String) -> String? {
    path.withCString { pointer in
        guard let resolved = realpath(pointer, nil) else { return nil }
        defer { free(resolved) }
        return String(cString: resolved)
    }
}

private final class UnixSocketListener: @unchecked Sendable {
    let path: String
    private(set) var descriptor: Int32

    init(path: String, owner: uid_t, group: gid_t) throws {
        self.path = path
        self.descriptor = -1
        try Self.prepareSocketPath(path)

        let descriptor = socket(AF_UNIX, SOCK_STREAM, 0)
        guard descriptor >= 0 else { throw Self.posixError("create helper socket") }
        self.descriptor = descriptor

        do {
            var address = try Self.address(for: path)
            let result = withUnsafePointer(to: &address) { pointer in
                pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    bind(descriptor, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
                }
            }
            guard result == 0 else { throw Self.posixError("bind helper socket") }
            guard listen(descriptor, 1) == 0 else { throw Self.posixError("listen on helper socket") }
            let flags = fcntl(descriptor, F_GETFL, 0)
            guard flags >= 0, fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) == 0 else {
                throw Self.posixError("make helper socket nonblocking")
            }
            guard chmod(path, mode_t(0o600)) == 0 else { throw Self.posixError("restrict helper socket") }
            guard chown(path, owner, group) == 0 else { throw Self.posixError("assign helper socket owner") }
        } catch {
            Darwin.close(descriptor)
            self.descriptor = -1
            try? FileManager.default.removeItem(atPath: path)
            throw error
        }
    }

    func accept() -> Int32? {
        let client = Darwin.accept(descriptor, nil, nil)
        guard client >= 0 else { return nil }
        // The listener must be nonblocking so the service can notice shutdown,
        // but a session's FileHandle read is intentionally blocking. Darwin
        // can carry O_NONBLOCK onto the accepted descriptor; leaving it set
        // makes an idle, newly connected agent look like a closed protocol
        // session before it has sent its first request.
        let flags = fcntl(client, F_GETFL, 0)
        guard flags >= 0, fcntl(client, F_SETFL, flags & ~O_NONBLOCK) == 0 else {
            Darwin.close(client)
            return nil
        }
        return client
    }

    func close() {
        guard descriptor >= 0 else { return }
        Darwin.shutdown(descriptor, SHUT_RDWR)
        Darwin.close(descriptor)
        descriptor = -1
        try? FileManager.default.removeItem(atPath: path)
    }

    deinit { close() }

    private static func prepareSocketPath(_ path: String) throws {
        var info = stat()
        if lstat(path, &info) == 0 {
            guard (info.st_mode & S_IFMT) == S_IFSOCK else {
                throw posixError("helper socket path is not a socket")
            }
            let probe = socket(AF_UNIX, SOCK_STREAM, 0)
            guard probe >= 0 else { throw posixError("probe helper socket") }
            defer { Darwin.close(probe) }
            var address = try address(for: path)
            let result = withUnsafePointer(to: &address) { pointer in
                pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    Darwin.connect(probe, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
                }
            }
            if result == 0 || (errno != ECONNREFUSED && errno != ENOENT) {
                throw posixError("existing helper socket is active")
            }
            try FileManager.default.removeItem(atPath: path)
        } else if errno != ENOENT {
            throw posixError("inspect helper socket path")
        }
    }

    private static func address(for path: String) throws -> sockaddr_un {
        var address = sockaddr_un()
        address.sun_family = sa_family_t(AF_UNIX)
        let bytes = Array(path.utf8)
        let capacity = MemoryLayout.size(ofValue: address.sun_path)
        guard bytes.count < capacity else { throw ServiceConfigurationError.invalidValue("helper socket path is too long") }
        withUnsafeMutableBytes(of: &address.sun_path) { buffer in
            buffer.initializeMemory(as: UInt8.self, repeating: 0)
            buffer.copyBytes(from: bytes)
        }
        return address
    }

    private static func posixError(_ operation: String) -> NSError {
        NSError(domain: NSPOSIXErrorDomain, code: Int(errno), userInfo: [NSLocalizedDescriptionKey: "\(operation): \(String(cString: strerror(errno)) )"])
    }
}

private final class ServiceSession: @unchecked Sendable {
    private let handle: FileHandle
    private let writer: SocketResponseWriter
    private let controller: BedrockSidecarController
    private let policy: PrivilegedServiceConfiguration
    private let closed = DispatchSemaphore(value: 0)
    private let stateLock = NSLock()
    private var shutdownRequested = false
    var onClosed: (() -> Void)?

    init(descriptor: Int32, policy: PrivilegedServiceConfiguration) {
        self.handle = FileHandle(fileDescriptor: descriptor, closeOnDealloc: true)
        self.writer = SocketResponseWriter(handle: self.handle)
        self.policy = policy
        self.controller = BedrockSidecarController(
            guestFileOwner: BedrockGuestFileOwner(
                uid: policy.allowedUID,
                gid: policy.allowedGID,
                identity: "root-helper"),
            responseHandler: { [writer] response in
            writer.write(response)
        })
    }

    func isAuthorized() -> Bool {
        var effectiveUID: uid_t = 0
        var effectiveGID: gid_t = 0
        return getpeereid(handle.fileDescriptor, &effectiveUID, &effectiveGID) == 0 && effectiveUID == policy.allowedUID
    }

    func start() {
        DispatchQueue.global(qos: .userInitiated).async { [self] in
            readRequests()
        }
    }

    func requestShutdown() {
        stateLock.lock()
        guard !shutdownRequested else {
            stateLock.unlock()
            return
        }
        shutdownRequested = true
        stateLock.unlock()
        DispatchQueue.main.async { [self] in
            controller.shutdown { [self] in
                handle.closeFile()
                closed.signal()
                onClosed?()
            }
        }
    }

    func waitUntilClosed() {
        closed.wait()
    }

    private func readRequests() {
        var pending = Data()
        do {
            var bytes = [UInt8](repeating: 0, count: 4096)
            while true {
                let count = Darwin.read(handle.fileDescriptor, &bytes, bytes.count)
                if count == 0 { break }
                if count < 0 {
                    if errno == EINTR { continue }
                    throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
                }
                pending.append(bytes, count: count)
                while let newline = pending.firstIndex(of: 0x0A) {
                    var line = Data(pending[..<newline])
                    pending.removeSubrange(...newline)
                    if line.last == 0x0D { line.removeLast() }
                    guard line.count <= maxServiceRequestBytes else {
                        throw ServiceProtocolError.requestTooLarge
                    }
                    try handleRequest(line)
                }
                guard pending.count <= maxServiceRequestBytes else {
                    throw ServiceProtocolError.requestTooLarge
                }
            }
            guard pending.isEmpty else { throw ServiceProtocolError.truncatedRequest }
        } catch {
            FileHandle.standardError.write(Data("bedrock helper protocol closed: \(error.localizedDescription)\n".utf8))
        }
        requestShutdown()
    }

    private func handleRequest(_ line: Data) throws {
        let request = try JSONDecoder().decode(SidecarRequest.self, from: line)
        let gatedRequest: SidecarRequest
        if case .provision(let serverDirectory, let version) = request {
            switch policy.validateServerDirectory(serverDirectory) {
            case .success(let canonical):
                gatedRequest = .provision(serverDir: canonical, version: version)
            case .failure(let error):
                writer.write(.provisioned(ok: false, reason: error.reason))
                return
            }
        } else {
            gatedRequest = request
        }
        DispatchQueue.main.sync { [self] in
            controller.handle(gatedRequest).forEach(writer.write)
        }
    }
}

private enum ServiceProtocolError: LocalizedError {
    case requestTooLarge
    case truncatedRequest

    var errorDescription: String? {
        switch self {
        case .requestTooLarge: return "request exceeds 65536 bytes"
        case .truncatedRequest: return "connection ended in the middle of a request"
        }
    }
}

private final class SocketResponseWriter: @unchecked Sendable {
    private let handle: FileHandle
    private let lock = NSLock()

    init(handle: FileHandle) { self.handle = handle }

    func write(_ response: SidecarResponse) {
        lock.lock()
        writeSidecarResponse(response, to: handle)
        lock.unlock()
    }
}

private final class PrivilegedService: @unchecked Sendable {
    private let configuration: PrivilegedServiceConfiguration
    private let listener: UnixSocketListener
    private let stateLock = NSLock()
    private var stopping = false
    private var activeSession: ServiceSession?

    init(configuration: PrivilegedServiceConfiguration) throws {
        self.configuration = configuration
        self.listener = try UnixSocketListener(
            path: configuration.socketPath,
            owner: configuration.allowedUID,
            group: configuration.allowedGID)
    }

    func start() {
        DispatchQueue.global(qos: .userInitiated).async { [self] in
            acceptSessions()
        }
    }

    func stop() {
        stateLock.lock()
        stopping = true
        let session = activeSession
        stateLock.unlock()
        listener.close()
        session?.requestShutdown()
    }

    private func acceptSessions() {
        while true {
            stateLock.lock()
            let shouldStop = stopping
            stateLock.unlock()
            if shouldStop { break }
            if activeSessionExists {
                if let descriptor = listener.accept() {
                    close(descriptor)
                } else {
                    Thread.sleep(forTimeInterval: 0.05)
                }
                continue
            }
            guard let descriptor = listener.accept() else {
                Thread.sleep(forTimeInterval: 0.05)
                continue
            }
            let session = ServiceSession(descriptor: descriptor, policy: configuration)
            guard session.isAuthorized() else {
                close(descriptor)
                continue
            }
            session.onClosed = { [weak self] in
                self?.clearActiveSession()
            }
            stateLock.lock()
            activeSession = session
            stateLock.unlock()
            session.start()
        }
        while activeSessionExists {
            Thread.sleep(forTimeInterval: 0.05)
        }
        DispatchQueue.main.async {
            CFRunLoopStop(CFRunLoopGetMain())
        }
    }

    private var activeSessionExists: Bool {
        stateLock.lock()
        defer { stateLock.unlock() }
        return activeSession != nil
    }

    private func clearActiveSession() {
        stateLock.lock()
        activeSession = nil
        stateLock.unlock()
    }
}

@main
private struct BedrockSidecarMain {
    static func main() {
        let arguments = Array(CommandLine.arguments.dropFirst())
        if arguments.contains("--service") {
            do {
                let service = try PrivilegedService(configuration: PrivilegedServiceConfiguration(arguments: arguments))
                let signalSource = DispatchSource.makeSignalSource(signal: SIGTERM, queue: .main)
                // A client can disconnect while a response is being written.
                // Ignore SIGPIPE so FileHandle.write reports that broken
                // connection as an error instead of launchd killing the
                // privileged helper and leaving the agent without a runtime.
                signal(SIGPIPE, SIG_IGN)
                signal(SIGTERM, SIG_IGN)
                signalSource.setEventHandler { service.stop() }
                signalSource.resume()
                service.start()
                RunLoop.main.run()
            } catch {
                FileHandle.standardError.write(Data("bedrock helper failed to start: \(error.localizedDescription)\n".utf8))
                exit(EXIT_FAILURE)
            }
        } else {
            runSidecar()
        }
    }
}
