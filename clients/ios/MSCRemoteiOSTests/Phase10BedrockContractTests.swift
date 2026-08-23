import XCTest
@testable import MSCRemoteiOS

/// P10.22 contract tests for additive Bedrock runtime state and shared routes.
/// These tests decode the public wire shapes and exercise the client without a
/// live agent so an unavailable runtime remains a first-class result.
@MainActor
final class Phase10BedrockContractTests: XCTestCase {
    private let token = "msc2_testid_testsecret"

    func testRuntimeStateAndCapabilitiesDecodeUnknownAdditions() throws {
        let json = """
        {
          "runtime": {
            "state": "unavailable",
            "backend": "vz-sidecar",
            "hostOs": "macos",
            "reasonCode": "no_test_hardware",
            "message": "No Bedrock test hardware is configured.",
            "helpId": "bedrock.runtime.no_test_hardware",
            "futureField": true
          },
          "serverTypes": {
            "vanilla": true, "paper": true, "fabric": true,
            "forge": true, "neoforge": true,
            "bedrock": {
              "supported": false,
              "backend": "vz-sidecar",
              "runtime": {
                "state": "unavailable",
                "reasonCode": "no_test_hardware"
              }
            }
          },
          "agentVersion": "2.0.0",
          "apiMajor": 1,
          "apiMinor": 6,
          "hostOs": "macos",
          "permissions": [],
          "helpers": {"playit": false, "duckdns": false, "geyser": true}
        }
        """.data(using: .utf8)!

        let capabilities = try JSONDecoder().decode(CapabilitiesDTO.self, from: json)

        XCTAssertFalse(capabilities.serverTypes.bedrock.supported)
        XCTAssertEqual(capabilities.serverTypes.bedrock.runtime?.reasonCode, "no_test_hardware")
        XCTAssertEqual(capabilities.serverTypes.bedrock.runtime?.state, "unavailable")
    }

    func testSettingsAllowlistAndOperationDecodeBedrockAdditions() throws {
        let settingsJSON = """
        {
          "serverType": "bedrock", "serverName": "Survival", "serverRunning": false,
          "editable": true,
          "sections": [{
            "id": "game", "title": "Game", "icon": "gamecontroller",
            "fields": [{
              "key": "max-players", "label": "Max Players", "helpId": "bedrock.max_players",
              "type": "int", "value": "10", "minInt": 1, "maxInt": 100
            }]
          }],
          "note": null,
          "runtime": {"state": "provisioning_required", "reasonCode": "missing_bds"}
        }
        """.data(using: .utf8)!
        let settings = try JSONDecoder().decode(SettingsResponseDTO.self, from: settingsJSON)

        XCTAssertEqual(settings.sections[0].fields[0].helpId, "bedrock.max_players")
        XCTAssertEqual(settings.sections[0].fields[0].help, "More help: bedrock.max_players")
        XCTAssertEqual(settings.runtime?.state, "provisioning_required")

        let allowlistJSON = """
        {"serverType":"bedrock","entries":[{"name":"Alex","xuid":"123","ignoresPlayerLimit":false}],
         "runtime":{"state":"available","backend":"native","hostOs":"linux"}}
        """.data(using: .utf8)!
        let allowlist = try JSONDecoder().decode(AllowlistResponseDTO.self, from: allowlistJSON)
        XCTAssertEqual(allowlist.entries.first?.xuid, "123")
        XCTAssertTrue(allowlist.runtime?.isAvailable == true)

        let operationJSON = """
        {"id":"op-bedrock-start","type":"server-start","state":"failed","cancelable":false,
         "error":{"code":"capability_unavailable","message":"Bedrock is unavailable.",
           "helpId":"bedrock.runtime.no_test_hardware",
           "details":{"capability":"server.lifecycle","serverType":"bedrock",
             "state":"unavailable","reasonCode":"no_test_hardware"}}}
        """.data(using: .utf8)!
        let operation = try JSONDecoder().decode(OperationDTO.self, from: operationJSON)
        XCTAssertEqual(operation.cancelable, false)
        XCTAssertEqual(operation.error?.details?.reasonCode, "no_test_hardware")
    }

    func testClientReadsCapabilitiesAndStatusRuntime() async throws {
        MockURLProtocol.responder = { request in
            switch (request.httpMethod, request.url?.path) {
            case ("GET", "/v1/capabilities"):
                return (200, ["Content-Type": "application/json"], Data("""
                {"agentVersion":"2.0.0","apiMajor":1,"apiMinor":6,"hostOs":"macos",
                 "permissions":[],"serverTypes":{"vanilla":true,"paper":true,"fabric":true,
                 "forge":true,"neoforge":true,"bedrock":{"supported":false,"backend":null,
                 "runtime":{"state":"unavailable","reasonCode":"no_test_hardware"}}},
                 "helpers":{"playit":false,"duckdns":false,"geyser":false}}
                """.utf8))
            case ("GET", "/v1/status"):
                return (200, ["Content-Type": "application/json"], Data("""
                {"running":false,"activeServerId":null,"pid":null,"serverType":"bedrock",
                 "runtime":{"state":"unavailable","reasonCode":"no_test_hardware"}}
                """.utf8))
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }
        MockURLProtocol.recorded = []
        let client = try RemoteAPIClient(
            baseURL: URL(string: "http://127.0.0.1:48400")!,
            token: token,
            protocolClasses: [MockURLProtocol.self]
        )

        let capabilities = try await client.getCapabilities()
        let status = try await client.getStatus()

        XCTAssertFalse(capabilities.serverTypes.bedrock.supported)
        XCTAssertEqual(status.runtime?.reasonCode, "no_test_hardware")
        XCTAssertEqual(MockURLProtocol.recorded.map { $0.request.url?.path }, [
            "/v1/capabilities", "/v1/status",
        ])
        XCTAssertTrue(MockURLProtocol.recorded.allSatisfy {
            $0.request.value(forHTTPHeaderField: "Authorization") == "Bearer \(token)"
        })
    }

    func testProductionResultsKeepUnknownBedrockValuesAdditive() throws {
        let json = """
        {
          "success": true,
          "message": "Bedrock server creation started.",
          "operationId": "op-bedrock-create",
          "runtime": {
            "state": "future_state",
            "backend": "future-backend",
            "hostOs": "linux",
            "reasonCode": "future_reason_code"
          }
        }
        """.data(using: .utf8)!

        let create = try JSONDecoder().decode(ServerCreateResultDTO.self, from: json)
        XCTAssertEqual(create.runtime?.backend, "future-backend")
        XCTAssertEqual(create.runtime?.reasonCode, "future_reason_code")

        let importJSON = """
        {
          "success": true,
          "message": "Server import accepted.",
          "operationId": "op-bedrock-import",
          "runtime": {"state": "provisioning_required", "reasonCode": "missing_bds"}
        }
        """.data(using: .utf8)!
        let imported = try JSONDecoder().decode(ServerImportResultDTO.self, from: importJSON)
        XCTAssertEqual(imported.runtime?.state, "provisioning_required")

        let lifecycleJSON = """
        {
          "result": "start_requested",
          "activeServerId": "bedrock-cli",
          "operationId": "op-bedrock-start",
          "runtime": {"state": "unavailable", "reasonCode": "no_test_hardware"}
        }
        """.data(using: .utf8)!
        let lifecycle = try JSONDecoder().decode(SimpleResult.self, from: lifecycleJSON)
        XCTAssertEqual(lifecycle.runtime?.reasonCode, "no_test_hardware")

        let versionsJSON = """
        {
          "supportsVersions": false,
          "flavorName": "bedrock",
          "currentVersion": null,
          "isBedrock": true,
          "versions": [],
          "note": "Bedrock versions are limited to the verified distribution selected for this runtime.",
          "runtime": {
            "state": "unavailable",
            "reasonCode": "no_test_hardware"
          }
        }
        """.data(using: .utf8)!
        let versions = try JSONDecoder().decode(VersionsResponseDTO.self, from: versionsJSON)
        XCTAssertEqual(versions.runtime?.reasonCode, "no_test_hardware")
    }
}
