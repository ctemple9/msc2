import Foundation
import XCTest
import Virtualization

private struct TestResources: ApplianceResourceProvider {
    let kernelURL: URL?
    let initramfsURL: URL?
}

final class BedrockSidecarTests: XCTestCase {
    func testFrozenRequestsDecodeAndRejectWrongTypes() throws {
        let decoder = JSONDecoder()
        XCTAssertEqual(
            try decoder.decode(SidecarRequest.self, from: Data(#"{"type":"provision","server_dir":"/srv/bedrock","version":"1.26.32.2"}"#.utf8)),
            .provision(serverDir: "/srv/bedrock", version: "1.26.32.2"))
        XCTAssertEqual(
            try decoder.decode(SidecarRequest.self, from: Data(#"{"type":"command","command":"say hello"}"#.utf8)),
            .command("say hello"))
        XCTAssertThrowsError(try decoder.decode(SidecarRequest.self, from: Data(#"{"type":"command","command":42}"#.utf8)))
        XCTAssertThrowsError(try decoder.decode(SidecarRequest.self, from: Data(#"{"type":"start","memory_gb":2,"bedrock_port":"19132"}"#.utf8)))
    }

    func testResponsesUseFrozenJSONLinesVocabulary() throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let ready = try encoder.encode(SidecarResponse.ready(guestIP: "192.168.64.7", port: 19132, relayUp: true))
        XCTAssertEqual(String(decoding: ready, as: UTF8.self), #"{"guest_ip":"192.168.64.7","port":19132,"relay_up":true,"type":"ready"}"#)
        let terminated = try encoder.encode(SidecarResponse.terminated("guest-error:boot failed"))
        XCTAssertEqual(String(decoding: terminated, as: UTF8.self), #"{"reason":"guest-error:boot failed","type":"terminated"}"#)
    }

    func testIntelPreconditionAndProvisionKeepStateHostOwned() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let kernel = directory.appendingPathComponent("vmlinuz-kata")
        let initramfs = directory.appendingPathComponent("appliance-initramfs.gz")
        FileManager.default.createFile(atPath: kernel.path, contents: Data([0]))
        FileManager.default.createFile(atPath: initramfs.path, contents: Data([0]))

        let controller = BedrockSidecarController(resources: TestResources(kernelURL: kernel, initramfsURL: initramfs))
        let responses = controller.handle(.provision(serverDir: directory.path, version: "1.26.32.2"))
        if BedrockSidecarController.hostArchitectureIsIntel && VZVirtualMachine.isSupported {
            XCTAssertEqual(responses, [.provisioned(ok: true, reason: nil)])
        } else {
            XCTAssertEqual(responses.count, 1)
            if case .provisioned(false, _) = responses[0] {} else { XCTFail("expected unavailable result") }
        }
        XCTAssertTrue(FileManager.default.fileExists(atPath: kernel.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: initramfs.path))
    }

    func testMissingApplianceIsAnExplicitProvisionFailure() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let controller = BedrockSidecarController(resources: TestResources(kernelURL: nil, initramfsURL: nil))
        XCTAssertEqual(
            controller.handle(.provision(serverDir: directory.path, version: "1.26.32.2")),
            [.provisioned(ok: false, reason: "VM kernel is missing from the app")])
    }

    func testGuestIPParserMatchesOracleShape() {
        XCTAssertEqual(BedrockSidecarController.parseGuestIP("[appliance] dhcp: 192.168.64.7/24"), "192.168.64.7")
        XCTAssertNil(BedrockSidecarController.parseGuestIP("[appliance] dhcp: unavailable"))
    }
}
