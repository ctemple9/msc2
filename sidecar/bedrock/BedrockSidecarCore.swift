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
    private var relay: UDPRelay?
    private var bedrockPort: UInt16 = 19132
    private var guestIP: String?
    private var relayReady = false
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
                let relay = try UDPRelay(listenPort: bedrockPort, guestHost: ip, guestPort: bedrockPort)
                self.relay = relay
                relay.start { [weak self] started in
                    guard started else {
                        self?.finish(reason: "start-failed:UDP relay could not bind")
                        return
                    }
                    self?.relayReady = true
                    self?.emitReadyIfPossible()
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

    private func emitReadyIfPossible() {
        guard relayReady, bedrockReady, let guestIP else { return }
        guard case .starting = state else { return }
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
        relay?.cancel()
        relay = nil
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

private final class UDPRelay: @unchecked Sendable {
    private let listener: NWListener
    private let guestHost: NWEndpoint.Host
    private let guestPort: NWEndpoint.Port
    private let queue = DispatchQueue(label: "msc.bedrock.udp-relay")
    private var clients: [(NWConnection, NWConnection)] = []
    private var startCompletion: ((Bool) -> Void)?

    init(listenPort: UInt16, guestHost: String, guestPort: UInt16) throws {
        guard let listen = NWEndpoint.Port(rawValue: listenPort),
              let guest = NWEndpoint.Port(rawValue: guestPort) else {
            throw NSError(domain: "BedrockSidecar", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "invalid UDP port"])
        }
        listener = try NWListener(using: .udp, on: listen)
        self.guestHost = NWEndpoint.Host(guestHost)
        self.guestPort = guest
    }

    func start(completion: @escaping (Bool) -> Void) {
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
        let guest = NWConnection(host: guestHost, port: guestPort, using: .udp)
        clients.append((client, guest))
        client.stateUpdateHandler = { _ in }
        guest.stateUpdateHandler = { _ in }
        client.start(queue: queue)
        guest.start(queue: queue)
        receive(from: client, to: guest)
        receive(from: guest, to: client)
    }

    private func receive(from source: NWConnection, to destination: NWConnection) {
        source.receiveMessage { [weak self] data, _, _, error in
            if let data, error == nil {
                destination.send(content: data, completion: .contentProcessed { _ in })
            }
            if error == nil { self?.receive(from: source, to: destination) }
        }
    }

    func cancel() {
        listener.cancel()
        clients.forEach { $0.0.cancel(); $0.1.cancel() }
        clients.removeAll()
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
