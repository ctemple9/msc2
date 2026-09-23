import Foundation
import Network
import Virtualization

private let protocolOutputLock = NSLock()

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
    case start(memoryGB: UInt32, bedrockPort: UInt16)
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
            try rejectUnexpected(["type", "memory_gb", "bedrock_port"])
            self = .start(
                memoryGB: try require("memory_gb", as: UInt32.self),
                bedrockPort: try require("bedrock_port", as: UInt16.self))
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

enum SidecarResponse: Encodable, Equatable {
    case provisioned(ok: Bool, reason: String?)
    case started(accepted: Bool, reason: String?)
    case ready(guestIP: String, port: UInt16, relayUp: Bool)
    case commandResult(ok: Bool, reason: String?)
    case consoleLine(String)
    case metrics(cpuPercent: Double?, ramUsedMB: Double?, ramMaxMB: Double?)
    case terminated(String)

    private enum CodingKeys: String, CodingKey {
        case type, ok, reason, accepted, guestIP = "guest_ip", port, relayUp = "relay_up", command, line
        case cpuPercent = "cpu_percent", ramUsedMB = "ram_used_mb", ramMaxMB = "ram_max_mb"
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

/// The only component in MSC 2 that knows about Virtualization.framework.
/// It intentionally exposes no second API: stdin/stdout are the complete
/// process boundary and `serverDirectory` is the only persistent state.
final class BedrockSidecarController: NSObject, @unchecked Sendable {
    private let resources: ApplianceResourceProvider
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
    private var tcpRelay: TCPRelay?
    private var udpRelays: [UDPRelay] = []
    private var bedrockReady = false
    private var gracefulStopWorkItem: DispatchWorkItem?
    private var didTerminate = false

    init(resources: ApplianceResourceProvider = BundleApplianceResources()) {
        self.resources = resources
    }

    func handle(_ request: SidecarRequest) -> [SidecarResponse] {
        switch request {
        case .provision(let serverDir, let version):
            return provision(serverDir: serverDir, version: version)
        case .start(let memoryGB, let port):
            return start(memoryGB: memoryGB, bedrockPort: port)
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
        udpRelays.removeAll()
        bedrockReady = false
        didTerminate = false
        state = .provisioned(serverDirectory: URL(fileURLWithPath: serverDir, isDirectory: true), version: version)
        return [.provisioned(ok: true, reason: nil)]
    }

    private func start(memoryGB: UInt32, bedrockPort: UInt16) -> [SidecarResponse] {
        guard case .provisioned(let serverDirectory, _) = state else {
            return [.started(accepted: false, reason: "provision-required-first")]
        }
        guard Self.hostArchitectureIsIntel else {
            return [.started(accepted: false, reason: "apple-silicon-unavailable-no-test-hardware")]
        }
        do {
            let configuration = try makeConfiguration(
                serverDirectory: serverDirectory,
                memoryGB: memoryGB)
            let machine = VZVirtualMachine(configuration: configuration, queue: DispatchQueue.main)
            machine.delegate = self
            vm = machine
            self.bedrockPort = bedrockPort
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
        bootLoader.commandLine = "console=hvc0"
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
        output.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            if data.isEmpty {
                self?.flushOutput()
            } else {
                self?.receiveGuestBytes(data)
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
        guard case .running = state, let input = guestInput else {
            return [.commandResult(ok: false, reason: "not-running")]
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
        guard let machine = vm else {
            finish(reason: "clean")
            return
        }
        machine.stop { [weak self] _ in
            self?.finish(reason: "clean")
        }
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
            do {
                let tcpRelay = try TCPRelay(
                    listenPort: bedrockPort,
                    guestHost: ip,
                    guestPort: bedrockPort)
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
            } catch {
                finish(reason: "start-failed:\(error.localizedDescription)")
                return
            }
        }
        send(.consoleLine(line))
        if Self.isBedrockServerReadyLine(line) {
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
        guard relayReady, bedrockReady, let guestIP else { return }
        guard case .starting = state else { return }
        guard relayPathReady else {
            guard !relayPathCheckInFlight, let tcpRelay else { return }
            relayPathCheckInFlight = true
            tcpRelay.verifyBedrockPath { [weak self] verified in
                DispatchQueue.main.async {
                    guard let self else { return }
                    self.relayPathCheckInFlight = false
                    guard verified else {
                        self.finish(reason: "start-failed:TCP signaling relay did not reach Bedrock")
                        return
                    }
                    self.relayPathReady = true
                    self.emitReadyIfPossible()
                }
            }
            return
        }
        send(.ready(guestIP: guestIP, port: bedrockPort, relayUp: true))
        state = .running
    }

    private func finish(reason: String) {
        stateLock.lock()
        guard !didTerminate else {
            stateLock.unlock()
            return
        }
        didTerminate = true
        state = .terminated
        stateLock.unlock()
        gracefulStopWorkItem?.cancel()
        gracefulStopWorkItem = nil
        tcpRelay?.cancel()
        tcpRelay = nil
        udpRelays.forEach { $0.cancel() }
        udpRelays.removeAll()
        guestOutput?.fileHandleForReading.readabilityHandler = nil
        guestOutput = nil
        guestInput = nil
        vm = nil
        send(.terminated(reason))
    }

    private func send(_ response: SidecarResponse) {
        do {
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.sortedKeys]
            var data = try encoder.encode(response)
            data.append(0x0A)
            protocolOutputLock.lock()
            FileHandle.standardOutput.write(data)
            protocolOutputLock.unlock()
        } catch {
            FileHandle.standardError.write(Data("sidecar response encoding failed: \(error)\n".utf8))
        }
    }

    static func parseGuestIP(_ line: String) -> String? {
        guard let range = line.range(of: #"\d{1,3}(\.\d{1,3}){3}"#, options: .regularExpression) else { return nil }
        return String(line[range])
    }

    static func netherNetUDPPorts(serverPort: UInt16) -> [UInt16] {
        let count: UInt16 = 32
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
    private let guestHost: NWEndpoint.Host
    private let guestPort: NWEndpoint.Port
    private let queue = DispatchQueue(label: "msc.bedrock.tcp-relay")
    private var clients: [ObjectIdentifier: Session] = [:]
    private var startCompletion: (@Sendable (Bool) -> Void)?

    init(listenPort: UInt16, guestHost: String, guestPort: UInt16) throws {
        guard let listen = NWEndpoint.Port(rawValue: listenPort),
              let guest = NWEndpoint.Port(rawValue: guestPort) else {
            throw NSError(domain: "BedrockSidecar", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "invalid TCP port"])
        }
        let parameters = NWParameters.tcp
        parameters.allowLocalEndpointReuse = true
        parameters.requiredLocalEndpoint = .hostPort(
            host: NWEndpoint.Host("0.0.0.0"), port: listen)
        listener = try NWListener(using: parameters)
        self.guestHost = NWEndpoint.Host(guestHost)
        self.guestPort = guest
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
        let guest = NWConnection(host: guestHost, port: guestPort, using: .tcp)
        let key = ObjectIdentifier(client)
        clients[key] = Session(client: client, guest: guest)
        client.stateUpdateHandler = { [weak self] state in
            self?.handleState(state, side: .client, key: key)
        }
        guest.stateUpdateHandler = { [weak self] state in
            self?.handleState(state, side: .guest, key: key)
        }
        client.start(queue: queue)
        guest.start(queue: queue)
    }

    private enum SessionSide: String {
        case client
        case guest
    }

    private func startPumpsIfReady(key: ObjectIdentifier) {
        guard let session = clients[key],
              session.clientReady,
              session.guestReady,
              !session.pumpsStarted else { return }
        session.pumpsStarted = true
        pump(from: session.client, to: session.guest, key: key)
        pump(from: session.guest, to: session.client, key: key)
    }

    private func pump(
        from source: NWConnection,
        to destination: NWConnection,
        key: ObjectIdentifier
    ) {
        source.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) {
            [weak self] data, _, complete, error in
            guard let self, self.clients[key] != nil else { return }
            if let data, !data.isEmpty {
                self.clients[key]?.lastActivity = Date()
                destination.send(content: data, completion: .contentProcessed { [weak self] error in
                    guard let self else { return }
                    if let error {
                        self.log("stream send failed: \(error.localizedDescription)")
                        self.closeClient(key)
                    } else if !complete {
                        self.pump(from: source, to: destination, key: key)
                    }
                })
            } else if !complete, error == nil {
                self.pump(from: source, to: destination, key: key)
            }
            if let error {
                self.log("stream receive failed: \(error.localizedDescription)")
                self.closeClient(key)
            } else if complete {
                self.closeClient(key)
            }
        }
    }

    private func handleState(
        _ state: NWConnection.State,
        side: SessionSide,
        key: ObjectIdentifier
    ) {
        switch state {
        case .ready:
            guard let session = clients[key] else { return }
            switch side {
            case .client: session.clientReady = true
            case .guest: session.guestReady = true
            }
            startPumpsIfReady(key: key)
        case .waiting(let error):
            log("\(side.rawValue) connection waiting: \(error.localizedDescription)")
        case .failed(let error):
            log("\(side.rawValue) connection failed: \(error.localizedDescription)")
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
        session.guest.stateUpdateHandler = nil
        session.client.cancel()
        session.guest.cancel()
    }

    func verifyBedrockPath(completion: @escaping @Sendable (Bool) -> Void) {
        queue.async { [weak self] in
            self?.verifyBedrockPath(attemptsRemaining: 5, completion: completion)
        }
    }

    private func verifyBedrockPath(
        attemptsRemaining: Int,
        completion: @escaping @Sendable (Bool) -> Void
    ) {
        guard attemptsRemaining > 0 else {
            completion(false)
            return
        }
        let probe = NWConnection(
            host: NWEndpoint.Host("127.0.0.1"),
            port: listener.port ?? guestPort,
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
                        attemptsRemaining: attemptsRemaining - 1,
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
    }

    func cancel() {
        queue.sync {
            listener.cancel()
            for key in Array(clients.keys) { closeClient(key) }
        }
    }
}

private final class UDPRelay: @unchecked Sendable {
    private final class Session {
        let client: NWConnection
        let guest: NWConnection
        var lastActivity = Date()

        init(client: NWConnection, guest: NWConnection) {
            self.client = client
            self.guest = guest
        }
    }

    private let listener: NWListener
    private let guestHost: NWEndpoint.Host
    private let guestPort: NWEndpoint.Port
    private let queue: DispatchQueue
    private var clients: [ObjectIdentifier: Session] = [:]
    private var cleanupTimer: DispatchSourceTimer?
    private var startCompletion: (@Sendable (Bool) -> Void)?

    init(listenPort: UInt16, guestHost: String, guestPort: UInt16) throws {
        guard let listen = NWEndpoint.Port(rawValue: listenPort),
              let guest = NWEndpoint.Port(rawValue: guestPort) else {
            throw NSError(domain: "BedrockSidecar", code: 3,
                          userInfo: [NSLocalizedDescriptionKey: "invalid UDP port"])
        }
        let parameters = NWParameters.udp
        parameters.allowLocalEndpointReuse = true
        parameters.requiredLocalEndpoint = .hostPort(
            host: NWEndpoint.Host("0.0.0.0"), port: listen)
        listener = try NWListener(using: parameters)
        self.guestHost = NWEndpoint.Host(guestHost)
        self.guestPort = guest
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
        let guest = NWConnection(host: guestHost, port: guestPort, using: .udp)
        let key = ObjectIdentifier(client)
        clients[key] = Session(client: client, guest: guest)
        client.start(queue: queue)
        guest.start(queue: queue)
        pump(from: client, to: guest, key: key)
        pump(from: guest, to: client, key: key)
    }

    private func pump(from source: NWConnection, to destination: NWConnection, key: ObjectIdentifier) {
        source.receiveMessage { [weak self] data, _, _, error in
            guard let self, self.clients[key] != nil else { return }
            if let data, !data.isEmpty {
                self.clients[key]?.lastActivity = Date()
                destination.send(content: data, completion: .idempotent)
            }
            if error == nil {
                self.pump(from: source, to: destination, key: key)
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
        session.guest.cancel()
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

func runSidecar() {
    let controller = BedrockSidecarController()
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
                        do {
                            let encoder = JSONEncoder()
                            encoder.outputFormatting = [.sortedKeys]
                            var data = try encoder.encode(response)
                            data.append(0x0A)
                            protocolOutputLock.lock()
                            FileHandle.standardOutput.write(data)
                            protocolOutputLock.unlock()
                        } catch {
                            FileHandle.standardError.write(Data("sidecar response encoding failed: \(error)\n".utf8))
                        }
                    }
                } catch {
                    FileHandle.standardError.write(Data("sidecar protocol error: \(error.localizedDescription)\n".utf8))
                }
            }
        }
        // EOF is the agent's shutdown signal. A live VM must not outlive its
        // supervisor, so force the guest down before the sidecar exits.
        DispatchQueue.main.async {
            _ = controller.handle(.forceStop)
            CFRunLoopStop(CFRunLoopGetMain())
        }
    }
    RunLoop.main.run()
}
