import Darwin
import Foundation
import Network
import Virtualization

private let protocolOutputLock = NSLock()

private enum NonblockingRead {
    case data(Data)
    case pending
    case endOfFile
    case failed
}

private func makeNonblocking(_ handle: FileHandle) throws {
    let descriptor = handle.fileDescriptor
    let flags = fcntl(descriptor, F_GETFL)
    guard flags >= 0, fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) == 0 else {
        throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
    }
}

private func readNonblocking(_ handle: FileHandle) -> NonblockingRead {
    var bytes = [UInt8](repeating: 0, count: 64 * 1024)
    let count = Darwin.read(handle.fileDescriptor, &bytes, bytes.count)
    if count > 0 { return .data(Data(bytes.prefix(count))) }
    if count == 0 { return .endOfFile }
    if errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR { return .pending }
    return .failed
}

private func sendDatagram(_ data: Data, to handle: FileHandle) -> Bool {
    data.withUnsafeBytes { buffer in
        guard let baseAddress = buffer.baseAddress else { return true }
        return Darwin.send(handle.fileDescriptor, baseAddress, data.count, 0) == data.count
    }
}

func writeSidecarResponse(_ response: SidecarResponse, to handle: FileHandle = .standardOutput) {
    do {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        var data = try encoder.encode(response)
        data.append(0x0A)
        protocolOutputLock.lock()
        defer { protocolOutputLock.unlock() }
        var written = 0
        while written < data.count {
            let result = data.withUnsafeBytes { buffer -> Int in
                guard let baseAddress = buffer.baseAddress else { return 0 }
                return Darwin.write(
                    handle.fileDescriptor,
                    baseAddress.advanced(by: written),
                    data.count - written)
            }
            if result > 0 {
                written += result
            } else if result < 0, errno == EINTR {
                continue
            } else {
                // The agent can close the authenticated socket while the
                // VM's asynchronous stop callback is still finishing. A raw
                // POSIX write reports that teardown as an errno instead of
                // allowing FileHandle to raise an Objective-C exception.
                let message = result == 0 ? "write made no progress" : String(cString: strerror(errno))
                FileHandle.standardError.write(Data("sidecar response write failed: \(message)\n".utf8))
                break
            }
        }
    } catch {
        FileHandle.standardError.write(Data("sidecar response encoding failed: \(error)\n".utf8))
    }
}

private struct CodingKeyName: CodingKey {
    var stringValue: String
    var intValue: Int? { nil }

    init(_ stringValue: String) { self.stringValue = stringValue }
    init?(stringValue: String) { self.stringValue = stringValue }
    init?(intValue: Int) { return nil }
}

private enum SidecarProtocolError: LocalizedError {
    case missingType
    case unknownType(String)
    case missingField(String)
    case wrongType(String)
    case unexpectedField(String)

    var errorDescription: String? {
        switch self {
        case .missingType: return "missing-type"
        case .unknownType(let type): return "unknown-type:\(type)"
        case .missingField(let field): return "missing-field:\(field)"
        case .wrongType(let field): return "\(field)-has-wrong-type"
        case .unexpectedField(let field): return "unexpected-field:\(field)"
        }
    }
}

enum SidecarRequest: Decodable, Equatable {
    case provision(serverDir: String, version: String)
    case start(memoryGB: UInt32, bedrockPort: UInt16, transport: String)
    case stop
    case forceStop
    case command(String)

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeyName.self)
        guard let type = try container.decodeIfPresent(String.self, forKey: CodingKeyName("type")) else {
            throw SidecarProtocolError.missingType
        }

        func require<T: Decodable>(_ name: String, as type: T.Type) throws -> T {
            guard container.contains(CodingKeyName(name)) else {
                throw SidecarProtocolError.missingField(name)
            }
            do {
                return try container.decode(type, forKey: CodingKeyName(name))
            } catch {
                throw SidecarProtocolError.wrongType(name)
            }
        }

        func rejectUnexpected(_ allowed: Set<String>) throws {
            for key in container.allKeys where !allowed.contains(key.stringValue) {
                throw SidecarProtocolError.unexpectedField(key.stringValue)
            }
        }

        switch type {
        case "provision":
            try rejectUnexpected(["type", "server_dir", "version"])
            self = .provision(
                serverDir: try require("server_dir", as: String.self),
                version: try require("version", as: String.self))
        case "start":
            try rejectUnexpected(["type", "memory_gb", "bedrock_port", "transport"])
            self = .start(
                memoryGB: try require("memory_gb", as: UInt32.self),
                bedrockPort: try require("bedrock_port", as: UInt16.self),
                transport: try require("transport", as: String.self))
        case "stop":
            try rejectUnexpected(["type"])
            self = .stop
        case "force-stop":
            try rejectUnexpected(["type"])
            self = .forceStop
        case "command":
            try rejectUnexpected(["type", "command"])
            self = .command(try require("command", as: String.self))
        default:
            throw SidecarProtocolError.unknownType(type)
        }
    }
}

enum SidecarResponse: Codable, Equatable {
    case provisioned(ok: Bool, reason: String?)
    case started(accepted: Bool, reason: String?)
    case ready(guestIP: String, port: UInt16, relayUp: Bool)
    case commandResult(ok: Bool, reason: String?)
    case consoleLine(String)
    case metrics(cpuPercent: Double?, ramUsedMB: Double?, ramMaxMB: Double?)
    case diagnostics(
        identity: String,
        protocolVersion: String,
        ownerUID: UInt32,
        ownerGID: UInt32,
        pathOwnership: String,
        lastTeardownReason: String?)
    case terminated(String)

    private enum CodingKeys: String, CodingKey {
        case type, ok, reason, accepted, guestIP = "guest_ip", port, relayUp = "relay_up", command, line
        case cpuPercent = "cpu_percent", ramUsedMB = "ram_used_mb", ramMaxMB = "ram_max_mb"
        case identity, protocolVersion = "protocol", ownerUID = "owner_uid", ownerGID = "owner_gid"
        case pathOwnership = "path_ownership", lastTeardownReason = "last_teardown_reason"
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        switch try c.decode(String.self, forKey: .type) {
        case "provisioned": self = .provisioned(ok: try c.decode(Bool.self, forKey: .ok), reason: try c.decodeIfPresent(String.self, forKey: .reason))
        case "started": self = .started(accepted: try c.decode(Bool.self, forKey: .accepted), reason: try c.decodeIfPresent(String.self, forKey: .reason))
        case "ready": self = .ready(guestIP: try c.decode(String.self, forKey: .guestIP), port: try c.decode(UInt16.self, forKey: .port), relayUp: try c.decode(Bool.self, forKey: .relayUp))
        case "command-result": self = .commandResult(ok: try c.decode(Bool.self, forKey: .ok), reason: try c.decodeIfPresent(String.self, forKey: .reason))
        case "console-line": self = .consoleLine(try c.decode(String.self, forKey: .line))
        case "metrics": self = .metrics(cpuPercent: try c.decodeIfPresent(Double.self, forKey: .cpuPercent), ramUsedMB: try c.decodeIfPresent(Double.self, forKey: .ramUsedMB), ramMaxMB: try c.decodeIfPresent(Double.self, forKey: .ramMaxMB))
        case "diagnostics": self = .diagnostics(identity: try c.decode(String.self, forKey: .identity), protocolVersion: try c.decode(String.self, forKey: .protocolVersion), ownerUID: try c.decode(UInt32.self, forKey: .ownerUID), ownerGID: try c.decode(UInt32.self, forKey: .ownerGID), pathOwnership: try c.decode(String.self, forKey: .pathOwnership), lastTeardownReason: try c.decodeIfPresent(String.self, forKey: .lastTeardownReason))
        case "terminated": self = .terminated(try c.decode(String.self, forKey: .reason))
        default: throw SidecarProtocolError.unknownType("worker-response")
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .provisioned(let ok, let reason):
            try container.encode("provisioned", forKey: .type)
            try container.encode(ok, forKey: .ok)
            try container.encodeIfPresent(reason, forKey: .reason)
        case .started(let accepted, let reason):
            try container.encode("started", forKey: .type)
            try container.encode(accepted, forKey: .accepted)
            try container.encodeIfPresent(reason, forKey: .reason)
        case .ready(let guestIP, let port, let relayUp):
            try container.encode("ready", forKey: .type)
            try container.encode(guestIP, forKey: .guestIP)
            try container.encode(port, forKey: .port)
            try container.encode(relayUp, forKey: .relayUp)
        case .commandResult(let ok, let reason):
            try container.encode("command-result", forKey: .type)
            try container.encode(ok, forKey: .ok)
            try container.encodeIfPresent(reason, forKey: .reason)
        case .consoleLine(let line):
            try container.encode("console-line", forKey: .type)
            try container.encode(line, forKey: .line)
        case .metrics(let cpuPercent, let ramUsedMB, let ramMaxMB):
            try container.encode("metrics", forKey: .type)
            try container.encodeIfPresent(cpuPercent, forKey: .cpuPercent)
            try container.encodeIfPresent(ramUsedMB, forKey: .ramUsedMB)
            try container.encodeIfPresent(ramMaxMB, forKey: .ramMaxMB)
        case .diagnostics(
            let identity,
            let protocolVersion,
            let ownerUID,
            let ownerGID,
            let pathOwnership,
            let lastTeardownReason):
            try container.encode("diagnostics", forKey: .type)
            try container.encode(identity, forKey: .identity)
            try container.encode(protocolVersion, forKey: .protocolVersion)
            try container.encode(ownerUID, forKey: .ownerUID)
            try container.encode(ownerGID, forKey: .ownerGID)
            try container.encode(pathOwnership, forKey: .pathOwnership)
            try container.encodeIfPresent(lastTeardownReason, forKey: .lastTeardownReason)
        case .terminated(let reason):
            try container.encode("terminated", forKey: .type)
            try container.encode(reason, forKey: .reason)
        }
    }
}

struct BedrockGuestMetrics: Equatable {
    let cpuPercent: Double?
    let ramUsedMB: Double?
    let ramMaxMB: Double?
}

protocol ApplianceResourceProvider {
    var kernelURL: URL? { get }
    var initramfsURL: URL? { get }
}

struct BundleApplianceResources: ApplianceResourceProvider {
    var kernelURL: URL? {
        Bundle.main.url(forResource: "vmlinuz-kata", withExtension: nil)
            ?? Bundle.main.url(forResource: "vmlinuz-kata-6.18.35", withExtension: nil)
    }

    var initramfsURL: URL? {
        Bundle.main.url(forResource: "appliance-initramfs", withExtension: "gz")
    }
}

private enum ControllerState {
    case new
    case provisioned(serverDirectory: URL, version: String)
    case starting
    case running
    case stopping
    case terminated
}

struct BedrockGuestFileOwner: Equatable, Sendable {
    let uid: uid_t
    let gid: gid_t
    let identity: String

    static func currentProcess(identity: String = "development-process") -> Self {
        Self(uid: getuid(), gid: getgid(), identity: identity)
    }
}

/// The only component in MSC 2 that knows about Virtualization.framework.
/// It intentionally exposes no management API: the foreground diagnostic
/// process and privileged service mode both carry this narrow protocol, while
/// `serverDirectory` is the only persistent state.
final class BedrockSidecarController: NSObject, @unchecked Sendable {
    private let resources: ApplianceResourceProvider
    private let responseHandler: (SidecarResponse) -> Void
    private let stateLock = NSLock()
    private var state: ControllerState = .new
    private var vm: VZVirtualMachine?
    private var guestOutput: Pipe?
    private var guestInput: Pipe?
    private var pendingOutput = Data()
    private var bedrockPort: UInt16 = 19132
    private var guestIP: String?
    private var relayReady = false
    private var relayPathReady = false
    private var relayPathCheckInFlight = false
    private var relayStartsRemaining = 0
    private var connectionTransport = "raknet"
    private var tcpRelay: TCPRelay?
    private var udpRelay: UDPRelay?
    private var udpRelays: [UDPRelay] = []
    private var bedrockReady = false
    private var gracefulStopWorkItem: DispatchWorkItem?
    private var didTerminate = false
    private var shutdownCompletions: [() -> Void] = []
    private let guestFileOwner: BedrockGuestFileOwner
    private let supervisesWorker: Bool
    private let hostRelaysEnabled: Bool
    private var worker: BedrockVMWorker?
    private var pendingWorkerTeardownReason: String?
    private var diagnosedServerDirectory: URL?
    private var lastTeardownReason: String?

    init(
        resources: ApplianceResourceProvider = BundleApplianceResources(),
        guestFileOwner: BedrockGuestFileOwner = .currentProcess(),
        supervisesWorker: Bool = false,
        hostRelaysEnabled: Bool = true,
        responseHandler: @escaping (SidecarResponse) -> Void = { response in
            writeSidecarResponse(response)
        }) {
        self.resources = resources
        self.guestFileOwner = guestFileOwner
        self.supervisesWorker = supervisesWorker
        self.hostRelaysEnabled = hostRelaysEnabled
        self.responseHandler = responseHandler
    }

    func handle(_ request: SidecarRequest) -> [SidecarResponse] {
        switch request {
        case .provision(let serverDir, let version):
            return provision(serverDir: serverDir, version: version)
        case .start(let memoryGB, let port, let transport):
            return start(memoryGB: memoryGB, bedrockPort: port, transport: transport)
        case .stop:
            stop()
            return []
        case .forceStop:
            forceStop()
            return []
        case .command(let command):
            return commandResult(command)
        }
    }

    private func provision(serverDir: String, version: String) -> [SidecarResponse] {
        switch state {
        case .new, .terminated:
            break
        default:
            return [.provisioned(ok: false, reason: "provision-already-completed")]
        }
        guard Self.hostArchitectureIsIntel else {
            return [.provisioned(ok: false, reason: "apple-silicon-unavailable-no-test-hardware")]
        }
        guard VZVirtualMachine.isSupported else {
            return [.provisioned(ok: false, reason: "virtualization-unavailable")]
        }
        guard let kernel = resources.kernelURL, FileManager.default.fileExists(atPath: kernel.path) else {
            return [.provisioned(ok: false, reason: "VM kernel is missing from the app")]
        }
        guard let initramfs = resources.initramfsURL, FileManager.default.fileExists(atPath: initramfs.path) else {
            return [.provisioned(ok: false, reason: "VM initramfs is missing from the app")]
        }
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: serverDir, isDirectory: &isDirectory), isDirectory.boolValue else {
            return [.provisioned(ok: false, reason: "Server folder not found: \(serverDir)")]
        }
        // The sidecar process stays alive between first-start attempts. A
        // terminated VM is a fresh run, so discard the previous run's guest
        // address and termination latch before binding the next one.
        pendingOutput.removeAll(keepingCapacity: false)
        guestIP = nil
        relayReady = false
        relayPathReady = false
        relayPathCheckInFlight = false
        relayStartsRemaining = 0
        tcpRelay = nil
        udpRelay = nil
        udpRelays.removeAll()
        bedrockReady = false
        didTerminate = false
        state = .provisioned(serverDirectory: URL(fileURLWithPath: serverDir, isDirectory: true), version: version)
        diagnosedServerDirectory = URL(fileURLWithPath: serverDir, isDirectory: true)
        if supervisesWorker {
            do {
                let workerID = UUID()
                let next = try BedrockVMWorker(id: workerID) { [weak self] response in
                    self?.receiveWorkerResponse(response)
                } closed: { [weak self] in
                    guard let self, self.worker?.id == workerID else { return }
                    self.worker = nil
                    let reason = self.pendingWorkerTeardownReason ?? "vm-worker-disconnected"
                    self.pendingWorkerTeardownReason = nil
                    self.finish(reason: reason)
                }
                worker = next
                try next.send(.provision(serverDir: serverDir, version: version))
                return []
            } catch {
                worker = nil
                state = .terminated
                return [.provisioned(ok: false, reason: "vm-worker-start-failed:\(error.localizedDescription)")]
            }
        }
        return [.provisioned(ok: true, reason: nil), diagnosticsResponse()]
    }

    private func start(
        memoryGB: UInt32,
        bedrockPort: UInt16,
        transport: String
    ) -> [SidecarResponse] {
        guard case .provisioned(let serverDirectory, _) = state else {
            return [.started(accepted: false, reason: "provision-required-first")]
        }
        guard Self.hostArchitectureIsIntel else {
            return [.started(accepted: false, reason: "apple-silicon-unavailable-no-test-hardware")]
        }
        guard transport == "raknet" || transport == "nethernet" else {
            return [.started(accepted: false, reason: "unsupported-transport:\(transport)")]
        }
        if supervisesWorker {
            self.bedrockPort = bedrockPort
            connectionTransport = transport
            state = .starting
            do {
                try worker?.send(.start(memoryGB: memoryGB, bedrockPort: bedrockPort, transport: transport))
                return []
            } catch {
                finish(reason: "vm-worker-send-failed:\(error.localizedDescription)")
                return [.started(accepted: false, reason: error.localizedDescription)]
            }
        }
        do {
            let configuration = try makeConfiguration(
                serverDirectory: serverDirectory,
                memoryGB: memoryGB)
            let machine = VZVirtualMachine(configuration: configuration, queue: DispatchQueue.main)
            machine.delegate = self
            vm = machine
            self.bedrockPort = bedrockPort
            connectionTransport = transport
            state = .starting
            machine.start { [weak self] result in
                if case .failure(let error) = result {
                    self?.finish(reason: "start-failed:\(error.localizedDescription)")
                }
            }
            return [.started(accepted: true, reason: nil)]
        } catch {
            return [.started(accepted: false, reason: error.localizedDescription)]
        }
    }

    private func makeConfiguration(serverDirectory: URL, memoryGB: UInt32) throws -> VZVirtualMachineConfiguration {
        guard let kernel = resources.kernelURL, let initramfs = resources.initramfsURL else {
            throw NSError(domain: "BedrockSidecar", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "VM appliance resources are unavailable"])
        }
        let configuration = VZVirtualMachineConfiguration()
        let bootLoader = VZLinuxBootLoader(kernelURL: kernel)
        bootLoader.initialRamdiskURL = initramfs
        bootLoader.commandLine = "console=hvc0 msc_uid=\(guestFileOwner.uid) msc_gid=\(guestFileOwner.gid)"
        configuration.bootLoader = bootLoader
        configuration.platform = VZGenericPlatformConfiguration()
        configuration.cpuCount = max(
            VZVirtualMachineConfiguration.minimumAllowedCPUCount,
            min(2, VZVirtualMachineConfiguration.maximumAllowedCPUCount))
        let requestedMemory = UInt64(max(memoryGB, 2)) * 1024 * 1024 * 1024
        configuration.memorySize = max(
            VZVirtualMachineConfiguration.minimumAllowedMemorySize,
            min(requestedMemory, VZVirtualMachineConfiguration.maximumAllowedMemorySize))

        let output = Pipe()
        let input = Pipe()
        guestOutput = output
        guestInput = input
        let serial = VZVirtioConsoleDeviceSerialPortConfiguration()
        serial.attachment = VZFileHandleSerialPortAttachment(
            fileHandleForReading: input.fileHandleForReading,
            fileHandleForWriting: output.fileHandleForWriting)
        configuration.serialPorts = [serial]
        try makeNonblocking(output.fileHandleForReading)
        output.fileHandleForReading.readabilityHandler = { [weak self] handle in
            switch readNonblocking(handle) {
            case .data(let data):
                self?.receiveGuestBytes(data)
            case .endOfFile, .failed:
                handle.readabilityHandler = nil
                self?.flushOutput()
            case .pending:
                break
            }
        }

        let network = VZVirtioNetworkDeviceConfiguration()
        network.attachment = VZNATNetworkDeviceAttachment()
        configuration.networkDevices = [network]

        try VZVirtioFileSystemDeviceConfiguration.validateTag("world")
        let share = VZVirtioFileSystemDeviceConfiguration(tag: "world")
        share.share = VZSingleDirectoryShare(directory: VZSharedDirectory(url: serverDirectory, readOnly: false))
        configuration.directorySharingDevices = [share]
        configuration.entropyDevices = [VZVirtioEntropyDeviceConfiguration()]
        configuration.memoryBalloonDevices = [VZVirtioTraditionalMemoryBalloonDeviceConfiguration()]
        try configuration.validate()
        return configuration
    }

    private func commandResult(_ command: String) -> [SidecarResponse] {
        if supervisesWorker {
            do { try worker?.send(.command(command)); return [] }
            catch { return [.commandResult(ok: false, reason: error.localizedDescription)] }
        }
        guard case .running = state, let input = guestInput else {
            return [.commandResult(ok: false, reason: "not-running")]
        }
        guard !command.isEmpty, !command.contains("\n"), !command.contains("\r") else {
            return [.commandResult(ok: false, reason: "invalid-command")]
        }
        let payload = command.hasSuffix("\n") ? command : command + "\n"
        guard let data = payload.data(using: .utf8) else {
            return [.commandResult(ok: false, reason: "encoding-failure")]
        }
        input.fileHandleForWriting.write(data)
        return [.commandResult(ok: true, reason: nil)]
    }

    private func stop() {
        guard stateIsStartingOrRunning else { return }
        state = .stopping
        if supervisesWorker {
            do { try worker?.send(.stop) }
            catch { finish(reason: "vm-worker-send-failed:\(error.localizedDescription)") }
            return
        }
        if let input = guestInput {
            input.fileHandleForWriting.write(Data("stop\n".utf8))
        }
        let work = DispatchWorkItem { [weak self] in self?.forceStop() }
        gracefulStopWorkItem = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 20, execute: work)
    }

    private var stateIsStartingOrRunning: Bool {
        if case .starting = state { return true }
        if case .running = state { return true }
        return false
    }

    private func forceStop() {
        gracefulStopWorkItem?.cancel()
        gracefulStopWorkItem = nil
        if supervisesWorker, let worker {
            do { try worker.send(.forceStop) }
            catch { finish(reason: "vm-worker-send-failed:\(error.localizedDescription)") }
            return
        }
        guard let machine = vm else {
            finish(reason: "clean")
            return
        }
        machine.stop { [weak self] _ in
            self?.finish(reason: "clean")
        }
    }

    /// Keep the controller alive until the VM and all host relays have been
    /// torn down. The service socket uses this when its client disconnects.
    func shutdown(completion: @escaping () -> Void) {
        guard !didTerminate else {
            completion()
            return
        }
        shutdownCompletions.append(completion)
        forceStop()
    }

    private func receiveGuestBytes(_ data: Data) {
        pendingOutput.append(data)
        while let newline = pendingOutput.firstIndex(of: 0x0A) {
            let line = pendingOutput.prefix(upTo: newline)
            pendingOutput.removeSubrange(...newline)
            processGuestLine(String(decoding: line, as: UTF8.self).trimmingCharacters(in: CharacterSet(charactersIn: "\r")))
        }
    }

    private func flushOutput() {
        guard !pendingOutput.isEmpty else { return }
        let line = String(decoding: pendingOutput, as: UTF8.self)
        pendingOutput.removeAll(keepingCapacity: false)
        processGuestLine(line)
    }

    private func processGuestLine(_ line: String) {
        if let metrics = Self.parseStats(line) {
            send(.metrics(
                cpuPercent: metrics.cpuPercent,
                ramUsedMB: metrics.ramUsedMB,
                ramMaxMB: metrics.ramMaxMB))
            return
        }
        if line.contains("[MSCSTATS]") { return }
        if guestIP == nil, line.contains("[appliance] dhcp:"), let ip = Self.parseGuestIP(line) {
            guestIP = ip
            if hostRelaysEnabled {
                do {
                    if connectionTransport == "raknet" {
                        let udpRelay = try UDPRelay(
                            listenPort: bedrockPort,
                            guestHost: ip,
                            guestPort: bedrockPort)
                        self.udpRelay = udpRelay
                        relayStartsRemaining = 1
                        udpRelay.start { [weak self] started in
                            DispatchQueue.main.async {
                                self?.relayDidStart(started, kind: "UDP RakNet")
                            }
                        }
                    } else {
                        let tcpRelay = try TCPRelay(
                            listenPort: bedrockPort,
                            guestHost: ip,
                            guestPort: bedrockPort,
                            diagnostic: { [weak self] message in
                                DispatchQueue.main.async {
                                    self?.send(.consoleLine("[TCP relay] \(message)"))
                                }
                            })
                        let udpRelays = try Self.netherNetUDPPorts(serverPort: bedrockPort).map { port in
                            try UDPRelay(listenPort: port, guestHost: ip, guestPort: port)
                        }
                        self.tcpRelay = tcpRelay
                        self.udpRelays = udpRelays
                        relayStartsRemaining = udpRelays.count + 1
                        tcpRelay.start { [weak self] started in
                            DispatchQueue.main.async {
                                self?.relayDidStart(started, kind: "TCP signaling")
                            }
                        }
                        for relay in udpRelays {
                            relay.start { [weak self] started in
                                DispatchQueue.main.async {
                                    self?.relayDidStart(started, kind: "UDP gameplay")
                                }
                            }
                        }
                    }
                } catch {
                    finish(reason: "start-failed:\(error.localizedDescription)")
                    return
                }
            }
        }
        send(.consoleLine(line))
        if Self.isBedrockServerReadyLine(line) {
            // launchd captures this one lifecycle milestone for the
            // unprivileged live inspector; the full console remains on the
            // authenticated agent stream.
            FileHandle.standardOutput.write(Data("bedrock guest: \(line)\n".utf8))
            bedrockReady = true
            emitReadyIfPossible()
        }
    }

    private func relayDidStart(_ started: Bool, kind: String) {
        guard case .starting = state else { return }
        guard started else {
            finish(reason: "start-failed:\(kind) relay could not bind")
            return
        }
        relayStartsRemaining -= 1
        if relayStartsRemaining == 0 {
            relayReady = true
            emitReadyIfPossible()
        }
    }

    private func emitReadyIfPossible() {
        if !hostRelaysEnabled {
            if bedrockReady { state = .running }
            return
        }
        guard relayReady, bedrockReady, let guestIP else { return }
        guard case .starting = state else { return }
        guard relayPathReady else {
            guard !relayPathCheckInFlight else { return }
            relayPathCheckInFlight = true
            let completion: @Sendable (Bool) -> Void = { [weak self] verified in
                DispatchQueue.main.async {
                    guard let self else { return }
                    self.relayPathCheckInFlight = false
                    guard verified else {
                        let boundary = self.connectionTransport == "raknet"
                            ? "UDP RakNet relay did not pass a Bedrock ping"
                            : "TCP signaling relay did not reach Bedrock"
                        self.finish(reason: "start-failed:\(boundary)")
                        return
                    }
                    self.relayPathReady = true
                    self.emitReadyIfPossible()
                }
            }
            if connectionTransport == "raknet" {
                guard let udpRelay else { return }
                udpRelay.verifyBedrockPath(completion: completion)
            } else {
                guard let tcpRelay else { return }
                tcpRelay.verifyBedrockPath(completion: completion)
            }
            return
        }
        send(.ready(guestIP: guestIP, port: bedrockPort, relayUp: true))
        state = .running
    }

    private func finish(reason: String) {
        if let worker {
            // EOF stops the VM; acknowledge teardown only after waitpid confirms
            // the worker is gone, including relay-start and protocol failures.
            pendingWorkerTeardownReason = pendingWorkerTeardownReason ?? reason
            state = .stopping
            worker.closeInput()
            return
        }
        stateLock.lock()
        guard !didTerminate else {
            stateLock.unlock()
            return
        }
        didTerminate = true
        state = .terminated
        lastTeardownReason = reason
        stateLock.unlock()
        gracefulStopWorkItem?.cancel()
        gracefulStopWorkItem = nil
        tcpRelay?.cancel()
        tcpRelay = nil
        udpRelay?.cancel()
        udpRelay = nil
        udpRelays.forEach { $0.cancel() }
        udpRelays.removeAll()
        guestOutput?.fileHandleForReading.readabilityHandler = nil
        guestOutput = nil
        guestInput = nil
        vm = nil
        worker?.closeInput()
        worker = nil
        send(diagnosticsResponse())
        send(.terminated(reason))
        let completions = shutdownCompletions
        shutdownCompletions.removeAll()
        completions.forEach { $0() }
    }

    private func send(_ response: SidecarResponse) {
        responseHandler(response)
    }

    private func receiveWorkerResponse(_ response: SidecarResponse) {
        switch response {
        case .consoleLine(let line): processGuestLine(line)
        case .terminated(let reason): finish(reason: reason)
        case .diagnostics, .ready: break
        default: send(response)
        }
    }

    private func diagnosticsResponse() -> SidecarResponse {
        .diagnostics(
            identity: guestFileOwner.identity,
            protocolVersion: "sidecar-v1",
            ownerUID: UInt32(guestFileOwner.uid),
            ownerGID: UInt32(guestFileOwner.gid),
            pathOwnership: pathOwnershipSummary(),
            lastTeardownReason: lastTeardownReason)
    }

    private func pathOwnershipSummary() -> String {
        guard let directory = diagnosedServerDirectory else { return "server-root=unavailable" }
        let paths: [(String, URL)] = [
            ("server-root", directory),
            ("server.properties", directory.appendingPathComponent("server.properties")),
            ("worlds", directory.appendingPathComponent("worlds", isDirectory: true)),
            ("logs", directory.appendingPathComponent("logs", isDirectory: true)),
            ("backups", directory.appendingPathComponent("backups", isDirectory: true)),
        ]
        return paths.map { label, path in
            var info = stat()
            guard lstat(path.path, &info) == 0 else { return "\(label)=missing" }
            guard info.st_uid == guestFileOwner.uid, info.st_gid == guestFileOwner.gid else {
                return "\(label)=mismatch:\(info.st_uid):\(info.st_gid)"
            }
            return "\(label)=verified:\(info.st_uid):\(info.st_gid)"
        }.joined(separator: ";")
    }

    static func parseGuestIP(_ line: String) -> String? {
        guard let range = line.range(of: #"\d{1,3}(\.\d{1,3}){3}"#, options: .regularExpression) else { return nil }
        return String(line[range])
    }

    static func netherNetUDPPorts(serverPort: UInt16) -> [UInt16] {
        let count: UInt16 = 16
        if serverPort <= UInt16.max - count {
            return Array((serverPort + 1) ... (serverPort + count))
        }
        guard serverPort > count else { return [] }
        return Array((serverPort - count) ... (serverPort - 1))
    }

    static func isBedrockServerReadyLine(_ line: String) -> Bool {
        line.range(of: "Server started", options: .caseInsensitive) != nil
    }

    static func parseStats(_ line: String) -> BedrockGuestMetrics? {
        let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.hasPrefix("[MSCSTATS]") else { return nil }

        var cpuPercent: Double?
        var ramUsedMB: Double?
        var ramMaxMB: Double?
        for field in trimmed.dropFirst("[MSCSTATS]".count).split(whereSeparator: \.isWhitespace) {
            guard let separator = field.firstIndex(of: "=") else { continue }
            let key = field[..<separator]
            let value = String(field[field.index(after: separator)...])
            switch key {
            case "cpu": cpuPercent = Double(value)
            case "memUsedMB": ramUsedMB = Double(value)
            case "memTotalMB": ramMaxMB = Double(value)
            default: break
            }
        }
        guard cpuPercent != nil || ramUsedMB != nil || ramMaxMB != nil else { return nil }
        return BedrockGuestMetrics(
            cpuPercent: cpuPercent,
            ramUsedMB: ramUsedMB,
            ramMaxMB: ramMaxMB)
    }

    #if arch(x86_64)
    static let hostArchitectureIsIntel = true
    #else
    static let hostArchitectureIsIntel = false
    #endif
}

extension BedrockSidecarController: VZVirtualMachineDelegate {
    func guestDidStop(_ virtualMachine: VZVirtualMachine) {
        flushOutput()
        finish(reason: "clean")
    }

    func virtualMachine(_ virtualMachine: VZVirtualMachine, didStopWithError error: Error) {
        flushOutput()
        finish(reason: "guest-error:\(error.localizedDescription)")
    }
}

private final class TCPRelay: @unchecked Sendable {
    private final class VerificationAttempt: @unchecked Sendable {
        var completed = false
    }

    private final class Session {
        let client: NWConnection
        let guest: NWConnection
        var lastActivity = Date()
        var clientReady = false
        var guestReady = false
        var pumpsStarted = false

        init(client: NWConnection, guest: NWConnection) {
            self.client = client
            self.guest = guest
        }
    }

    private let listener: NWListener
    private let guestHost: String
    private let guestPort: UInt16
    private let diagnostic: @Sendable (String) -> Void
    private let queue = DispatchQueue(label: "msc.bedrock.tcp-relay")
    private var clients: [ObjectIdentifier: Session] = [:]
    private var startCompletion: (@Sendable (Bool) -> Void)?

    init(
        listenPort: UInt16,
        guestHost: String,
        guestPort: UInt16,
        diagnostic: @escaping @Sendable (String) -> Void
    ) throws {
        guard let listen = NWEndpoint.Port(rawValue: listenPort),
              NWEndpoint.Port(rawValue: guestPort) != nil else {
            throw NSError(domain: "BedrockSidecar", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "invalid TCP port"])
        }
        let parameters = NWParameters.tcp
        parameters.allowLocalEndpointReuse = true
        parameters.requiredLocalEndpoint = .hostPort(
            host: NWEndpoint.Host("0.0.0.0"), port: listen)
        listener = try NWListener(using: parameters)
        self.guestHost = guestHost
        self.guestPort = guestPort
        self.diagnostic = diagnostic
    }

    func start(completion: @escaping @Sendable (Bool) -> Void) {
        startCompletion = completion
        listener.stateUpdateHandler = { [weak self] state in
            switch state {
            case .ready:
                self?.startCompletion?(true)
                self?.startCompletion = nil
            case .failed:
                self?.startCompletion?(false)
                self?.startCompletion = nil
            default:
                break
            }
        }
        listener.newConnectionHandler = { [weak self] client in self?.accept(client) }
        listener.start(queue: queue)
    }

    private func accept(_ client: NWConnection) {
        let key = ObjectIdentifier(client)
        let guest = NWConnection(
            host: NWEndpoint.Host(guestHost),
            port: NWEndpoint.Port(rawValue: guestPort)!,
            using: .tcp)
        clients[key] = Session(client: client, guest: guest)
        client.stateUpdateHandler = { [weak self] state in
            self?.handleState(state, key: key, guest: false)
        }
        guest.stateUpdateHandler = { [weak self] state in
            self?.handleState(state, key: key, guest: true)
        }
        client.start(queue: queue)
        guest.start(queue: queue)
    }

    private func startPumpsIfReady(key: ObjectIdentifier) {
        guard let session = clients[key], session.clientReady, session.guestReady,
              !session.pumpsStarted else { return }
        session.pumpsStarted = true
        pump(key: key, from: session.client, to: session.guest, direction: "client-to-guest")
        pump(key: key, from: session.guest, to: session.client, direction: "guest-to-client")
    }

    private func pump(
        key: ObjectIdentifier,
        from source: NWConnection,
        to destination: NWConnection,
        direction: String
    ) {
        guard let session = clients[key] else { return }
        source.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) {
            [weak self] data, _, complete, error in
            guard let self, self.clients[key] != nil else { return }
            if let data, !data.isEmpty {
                session.lastActivity = Date()
                destination.send(content: data, completion: .contentProcessed { [weak self] sendError in
                    guard let self else { return }
                    if let sendError {
                        self.log("\(direction) send failed: \(sendError.localizedDescription)")
                        self.closeClient(key)
                    } else if !complete {
                        self.pump(key: key, from: source, to: destination, direction: direction)
                    } else {
                        self.closeClient(key)
                    }
                })
            } else if !complete, error == nil {
                self.pump(key: key, from: source, to: destination, direction: direction)
            }
            if let error {
                self.log("\(direction) receive failed: \(error.localizedDescription)")
                self.closeClient(key)
            } else if complete {
                self.closeClient(key)
            }
        }
    }

    private func handleState(_ state: NWConnection.State, key: ObjectIdentifier, guest: Bool) {
        switch state {
        case .ready:
            if guest {
                clients[key]?.guestReady = true
            } else {
                clients[key]?.clientReady = true
            }
            startPumpsIfReady(key: key)
        case .waiting(let error):
            log("\(guest ? "guest" : "client") connection waiting: \(error.localizedDescription)")
        case .failed(let error):
            log("\(guest ? "guest" : "client") connection failed: \(error.localizedDescription)")
            closeClient(key)
        case .cancelled:
            closeClient(key)
        default:
            break
        }
    }

    private func closeClient(_ key: ObjectIdentifier) {
        guard let session = clients.removeValue(forKey: key) else { return }
        session.client.stateUpdateHandler = nil
        session.client.cancel()
        session.guest.stateUpdateHandler = nil
        session.guest.cancel()
    }

    func verifyBedrockPath(completion: @escaping @Sendable (Bool) -> Void) {
        queue.async { [weak self] in
            self?.verifyBedrockPath(
                deadline: Date().addingTimeInterval(30),
                completion: completion)
        }
    }

    private func verifyBedrockPath(
        deadline: Date,
        completion: @escaping @Sendable (Bool) -> Void
    ) {
        guard Date() < deadline else {
            completion(false)
            return
        }
        let probe = NWConnection(
            host: NWEndpoint.Host("127.0.0.1"),
            port: listener.port ?? NWEndpoint.Port(rawValue: guestPort)!,
            using: .tcp)
        let attempt = VerificationAttempt()
        let finish: @Sendable (Bool) -> Void = { [self] success in
            guard !attempt.completed else { return }
            attempt.completed = true
            probe.cancel()
            if success {
                completion(true)
            } else {
                queue.asyncAfter(deadline: .now() + 0.5) { [self] in
                    verifyBedrockPath(
                        deadline: deadline,
                        completion: completion)
                }
            }
        }
        probe.stateUpdateHandler = { [self] state in
            guard case .ready = state else {
                if case .failed(let error) = state {
                    log("relay verification connection failed: \(error.localizedDescription)")
                    finish(false)
                }
                return
            }
            let request = Data("GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n".utf8)
            probe.send(content: request, completion: .contentProcessed { [self] error in
                if let error {
                    self.log("relay verification send failed: \(error.localizedDescription)")
                    finish(false)
                    return
                }
                probe.receive(minimumIncompleteLength: 1, maximumLength: 4096) { data, _, _, _ in
                    finish(data?.isEmpty == false)
                }
            })
        }
        probe.start(queue: queue)
        queue.asyncAfter(deadline: .now() + 2) { finish(false) }
    }

    private func log(_ message: String) {
        FileHandle.standardError.write(Data("bedrock TCP relay: \(message)\n".utf8))
        diagnostic(message)
    }

    func cancel() {
        queue.sync {
            listener.cancel()
            for key in Array(clients.keys) { closeClient(key) }
        }
    }
}

private final class UDPRelay: @unchecked Sendable {
    private final class VerificationAttempt: @unchecked Sendable {
        var completed = false
    }

    private final class Session {
        let client: NWConnection
        let guest: FileHandle
        var lastActivity = Date()

        init(client: NWConnection, guest: FileHandle) {
            self.client = client
            self.guest = guest
        }
    }

    private let listener: NWListener
    private let guestHost: String
    private let guestPort: UInt16
    private let queue: DispatchQueue
    private var clients: [ObjectIdentifier: Session] = [:]
    private var cleanupTimer: DispatchSourceTimer?
    private var startCompletion: (@Sendable (Bool) -> Void)?

    fileprivate static let rakNetMagic: [UInt8] = [
        0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe,
        0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
    ]

    init(listenPort: UInt16, guestHost: String, guestPort: UInt16) throws {
        guard let listen = NWEndpoint.Port(rawValue: listenPort),
              NWEndpoint.Port(rawValue: guestPort) != nil else {
            throw NSError(domain: "BedrockSidecar", code: 3,
                          userInfo: [NSLocalizedDescriptionKey: "invalid UDP port"])
        }
        let parameters = NWParameters.udp
        parameters.allowLocalEndpointReuse = true
        parameters.requiredLocalEndpoint = .hostPort(
            host: NWEndpoint.Host("0.0.0.0"), port: listen)
        listener = try NWListener(using: parameters)
        self.guestHost = guestHost
        self.guestPort = guestPort
        queue = DispatchQueue(label: "msc.bedrock.udp-relay.\(listenPort)")
    }

    func start(completion: @escaping @Sendable (Bool) -> Void) {
        startCompletion = completion
        listener.stateUpdateHandler = { [weak self] state in
            switch state {
            case .ready:
                self?.startCompletion?(true)
                self?.startCompletion = nil
            case .failed:
                self?.startCompletion?(false)
                self?.startCompletion = nil
            default:
                break
            }
        }
        listener.newConnectionHandler = { [weak self] client in self?.accept(client) }
        listener.start(queue: queue)
        let timer = DispatchSource.makeTimerSource(queue: queue)
        timer.schedule(deadline: .now() + 30, repeating: 30)
        timer.setEventHandler { [weak self] in self?.removeIdleClients() }
        timer.resume()
        cleanupTimer = timer
    }

    private func accept(_ client: NWConnection) {
        let key = ObjectIdentifier(client)
        let guest: FileHandle
        do {
            guest = try connectedIPv4Socket(
                host: guestHost,
                port: guestPort,
                type: SOCK_DGRAM)
        } catch {
            client.cancel()
            return
        }
        clients[key] = Session(client: client, guest: guest)
        guest.readabilityHandler = { [weak self] handle in
            guard let relay = self else { return }
            switch readNonblocking(handle) {
            case .data(let data):
                relay.queue.async { [weak relay] in
                    guard let relay, relay.clients[key] != nil else { return }
                    relay.clients[key]?.lastActivity = Date()
                    client.send(content: data, completion: .idempotent)
                }
            case .endOfFile, .failed:
                relay.queue.async { [weak relay] in
                    relay?.closeClient(key)
                }
            case .pending:
                break
            }
        }
        client.start(queue: queue)
        pumpClientToGuest(key: key)
    }

    private func pumpClientToGuest(key: ObjectIdentifier) {
        guard let session = clients[key] else { return }
        session.client.receiveMessage { [weak self] data, _, _, error in
            guard let self, self.clients[key] != nil else { return }
            if let data, !data.isEmpty {
                self.clients[key]?.lastActivity = Date()
                guard let guest = self.clients[key]?.guest,
                      sendDatagram(data, to: guest) else {
                    self.closeClient(key)
                    return
                }
            }
            if error == nil {
                self.pumpClientToGuest(key: key)
            } else {
                self.closeClient(key)
            }
        }
    }

    private func removeIdleClients() {
        let cutoff = Date().addingTimeInterval(-120)
        for (key, session) in clients where session.lastActivity < cutoff {
            closeClient(key)
        }
    }

    private func closeClient(_ key: ObjectIdentifier) {
        guard let session = clients.removeValue(forKey: key) else { return }
        session.client.cancel()
        session.guest.readabilityHandler = nil
        try? session.guest.close()
    }

    func verifyBedrockPath(completion: @escaping @Sendable (Bool) -> Void) {
        queue.async { [weak self] in
            self?.verifyBedrockPath(
                deadline: Date().addingTimeInterval(30),
                completion: completion)
        }
    }

    private func verifyBedrockPath(
        deadline: Date,
        completion: @escaping @Sendable (Bool) -> Void
    ) {
        guard Date() < deadline else {
            completion(false)
            return
        }
        let probe = NWConnection(
            host: NWEndpoint.Host("127.0.0.1"),
            port: listener.port ?? NWEndpoint.Port(rawValue: guestPort)!,
            using: .udp)
        let attempt = VerificationAttempt()
        let finish: @Sendable (Bool) -> Void = { [self] success in
            guard !attempt.completed else { return }
            attempt.completed = true
            probe.cancel()
            if success {
                completion(true)
            } else {
                queue.asyncAfter(deadline: .now() + 0.5) { [self] in
                    verifyBedrockPath(deadline: deadline, completion: completion)
                }
            }
        }
        probe.stateUpdateHandler = { state in
            guard case .ready = state else {
                if case .failed = state { finish(false) }
                return
            }
            probe.send(content: Self.rakNetPing(), completion: .contentProcessed { error in
                guard error == nil else {
                    finish(false)
                    return
                }
                probe.receiveMessage { data, _, _, _ in
                    finish(data.isSomeValidRakNetPong)
                }
            })
        }
        probe.start(queue: queue)
        queue.asyncAfter(deadline: .now() + 2) { finish(false) }
    }

    private static func rakNetPing() -> Data {
        var data = Data([0x01])
        var timestamp = UInt64(Date().timeIntervalSince1970 * 1_000).bigEndian
        withUnsafeBytes(of: &timestamp) { data.append(contentsOf: $0) }
        data.append(contentsOf: rakNetMagic)
        var guid = UInt64.random(in: 1 ... UInt64.max).bigEndian
        withUnsafeBytes(of: &guid) { data.append(contentsOf: $0) }
        return data
    }

    func cancel() {
        queue.sync {
            cleanupTimer?.cancel()
            cleanupTimer = nil
            listener.cancel()
            for key in Array(clients.keys) { closeClient(key) }
        }
    }
}

private extension Optional where Wrapped == Data {
    var isSomeValidRakNetPong: Bool {
        guard let data = self, data.count >= 35, data[0] == 0x1c else { return false }
        return Array(data[17 ..< 33]) == UDPRelay.rakNetMagic
    }
}

private func connectedIPv4Socket(host: String, port: UInt16, type: Int32) throws -> FileHandle {
    let descriptor = Darwin.socket(AF_INET, type, 0)
    guard descriptor >= 0 else {
        throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
    }

    var address = sockaddr_in()
    address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
    address.sin_family = sa_family_t(AF_INET)
    address.sin_port = port.bigEndian
    let converted = host.withCString { pointer in
        inet_pton(AF_INET, pointer, &address.sin_addr)
    }
    guard converted == 1 else {
        Darwin.close(descriptor)
        throw NSError(domain: NSPOSIXErrorDomain, code: Int(EINVAL))
    }

    if var interface = interfaceIndex(for: address.sin_addr) {
        let bound = withUnsafePointer(to: &interface) { pointer in
            setsockopt(
                descriptor,
                IPPROTO_IP,
                IP_BOUND_IF,
                pointer,
                socklen_t(MemoryLayout<UInt32>.size))
        }
        guard bound == 0 else {
            let code = errno
            Darwin.close(descriptor)
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(code))
        }
    }

    let result = withUnsafePointer(to: &address) { pointer in
        pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { socketAddress in
            Darwin.connect(
                descriptor,
                socketAddress,
                socklen_t(MemoryLayout<sockaddr_in>.size))
        }
    }
    guard result == 0 else {
        let code = errno
        Darwin.close(descriptor)
        throw NSError(domain: NSPOSIXErrorDomain, code: Int(code))
    }
    let handle = FileHandle(fileDescriptor: descriptor, closeOnDealloc: true)
    do {
        try makeNonblocking(handle)
        return handle
    } catch {
        try? handle.close()
        throw error
    }
}

private func interfaceIndex(for target: in_addr) -> UInt32? {
    var interfaces: UnsafeMutablePointer<ifaddrs>?
    guard getifaddrs(&interfaces) == 0, let first = interfaces else { return nil }
    defer { freeifaddrs(first) }

    var current: UnsafeMutablePointer<ifaddrs>? = first
    while let item = current {
        let interface = item.pointee
        if let rawAddress = interface.ifa_addr,
           let rawMask = interface.ifa_netmask,
           rawAddress.pointee.sa_family == UInt8(AF_INET),
           rawMask.pointee.sa_family == UInt8(AF_INET) {
            let local = UnsafeRawPointer(rawAddress)
                .assumingMemoryBound(to: sockaddr_in.self)
                .pointee.sin_addr.s_addr
            let mask = UnsafeRawPointer(rawMask)
                .assumingMemoryBound(to: sockaddr_in.self)
                .pointee.sin_addr.s_addr
            if local & mask == target.s_addr & mask {
                let index = if_nametoindex(interface.ifa_name)
                if index != 0 { return index }
            }
        }
        current = interface.ifa_next
    }
    return nil
}

func runSidecar(vmWorker: Bool = false) {
    let controller = BedrockSidecarController(hostRelaysEnabled: !vmWorker)
    // Virtualization.framework delivers VM and serial-port callbacks on the
    // main queue. Reading stdin there would block that queue forever after a
    // start request, leaving the sidecar stuck at "process spawned" with no
    // ready signal or guest console output.
    DispatchQueue.global(qos: .userInitiated).async {
        while let line = readLine(strippingNewline: true) {
            DispatchQueue.main.async {
                do {
                    let request = try JSONDecoder().decode(SidecarRequest.self, from: Data(line.utf8))
                    controller.handle(request).forEach { response in
                        writeSidecarResponse(response)
                    }
                } catch {
                    FileHandle.standardError.write(Data("sidecar protocol error: \(error.localizedDescription)\n".utf8))
                }
            }
        }
        // EOF is the agent's shutdown signal. A live VM must not outlive its
        // supervisor, so force the guest down before the sidecar exits.
        DispatchQueue.main.async {
            controller.shutdown {
                CFRunLoopStop(CFRunLoopGetMain())
            }
        }
    }
    // Match CFRunLoopStop above. Foundation's run() re-enters the loop after
    // a stop, leaving an otherwise torn-down VM worker alive indefinitely.
    CFRunLoopRun()
}

/// The root supervisor keeps NAT relays; only this reset-identity child opens
/// the VM's writable share. No client can choose its executable or arguments.
final class BedrockVMWorker: @unchecked Sendable {
    let id: UUID
    private var input: FileHandle?
    private let child: pid_t

    init(id: UUID, response: @escaping @Sendable (SidecarResponse) -> Void,
         closed: @escaping @Sendable () -> Void) throws {
        self.id = id
        guard geteuid() == 0, getuid() != 0, getgid() != 0 else {
            throw NSError(domain: "BedrockSidecar", code: 4,
                          userInfo: [NSLocalizedDescriptionKey: "VM worker requires a configured non-root real identity"])
        }
        let incoming = Pipe()
        let outgoing = Pipe()
        var actions: posix_spawn_file_actions_t?
        var attributes: posix_spawnattr_t?
        var status = posix_spawn_file_actions_init(&actions)
        guard status == 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(status)) }
        defer { posix_spawn_file_actions_destroy(&actions) }
        status = posix_spawnattr_init(&attributes)
        guard status == 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(status)) }
        defer { posix_spawnattr_destroy(&attributes) }
        for (source, destination) in [
            (incoming.fileHandleForReading.fileDescriptor, STDIN_FILENO),
            (outgoing.fileHandleForWriting.fileDescriptor, STDOUT_FILENO),
            (STDERR_FILENO, STDERR_FILENO),
        ] {
            status = posix_spawn_file_actions_adddup2(&actions, source, destination)
            guard status == 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(status)) }
        }
        var defaults = sigset_t()
        sigemptyset(&defaults)
        sigaddset(&defaults, SIGTERM)
        sigaddset(&defaults, SIGINT)
        status = posix_spawnattr_setsigdefault(&attributes, &defaults)
        guard status == 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(status)) }
        status = posix_spawnattr_setflags(&attributes, Int16(POSIX_SPAWN_RESETIDS | POSIX_SPAWN_CLOEXEC_DEFAULT | POSIX_SPAWN_SETSIGDEF))
        guard status == 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(status)) }
        let executable = Bundle.main.executableURL!.path
        let arguments = [executable, "--vm-worker"].map { value in value.withCString { strdup($0)! } }
        let environment = ["PATH=/usr/bin:/bin:/usr/sbin:/sbin"].map { value in value.withCString { strdup($0)! } }
        defer { (arguments + environment).forEach { free($0) } }
        var argv: [UnsafeMutablePointer<CChar>?] = arguments.map { $0 } + [nil]
        var envp: [UnsafeMutablePointer<CChar>?] = environment.map { $0 } + [nil]
        var pid: pid_t = 0
        status = executable.withCString { posix_spawn(&pid, $0, &actions, &attributes, &argv, &envp) }
        guard status == 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(status)) }
        child = pid
        let spawnedPID = pid
        input = incoming.fileHandleForWriting
        incoming.fileHandleForReading.closeFile()
        outgoing.fileHandleForWriting.closeFile()
        DispatchQueue.global(qos: .userInitiated).async {
            defer {
                outgoing.fileHandleForReading.closeFile()
                var result: Int32 = 0
                while waitpid(spawnedPID, &result, 0) < 0 && errno == EINTR {}
                DispatchQueue.main.async(execute: closed)
            }
            var pending = Data()
            var bytes = [UInt8](repeating: 0, count: 4096)
            while true {
                let count = Darwin.read(outgoing.fileHandleForReading.fileDescriptor, &bytes, bytes.count)
                if count < 0 && errno == EINTR { continue }
                guard count > 0 else { break }
                pending.append(bytes, count: count)
                while let newline = pending.firstIndex(of: 0x0A) {
                    let line = Data(pending[..<newline])
                    pending.removeSubrange(...newline)
                    // A single legacy readiness milestone is also logged to stdout.
                    if line.starts(with: Data("bedrock guest: ".utf8)) { continue }
                    guard line.count <= 65536,
                          let decoded = try? JSONDecoder().decode(SidecarResponse.self, from: line) else {
                        kill(spawnedPID, SIGTERM)
                        return
                    }
                    DispatchQueue.main.async { response(decoded) }
                }
                if pending.count > 65536 { kill(spawnedPID, SIGTERM); return }
            }
        }
    }

    func send(_ request: SidecarRequest) throws {
        let object: [String: Any]
        switch request {
        case .provision(let path, let version): object = ["type": "provision", "server_dir": path, "version": version]
        case .start(let memory, let port, let transport): object = ["type": "start", "memory_gb": memory, "bedrock_port": port, "transport": transport]
        case .command(let command): object = ["type": "command", "command": command]
        case .stop: object = ["type": "stop"]
        case .forceStop: object = ["type": "force-stop"]
        }
        var data = try JSONSerialization.data(withJSONObject: object)
        data.append(0x0A)
        guard let input else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(EPIPE)) }
        try input.write(contentsOf: data)
    }

    func closeInput() {
        try? input?.close()
        input = nil
    }

    deinit { closeInput() }
}
