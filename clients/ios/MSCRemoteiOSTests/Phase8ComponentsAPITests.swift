import XCTest
@testable import MSCRemoteiOS

/// P8.26 contract/client tests for the copied components/add-on flows.
/// Offline only: `MockURLProtocol` intercepts every request.
@MainActor
final class Phase8ComponentsAPITests: XCTestCase {
    private let token = "msc2_testid_testsecret"

    func testClientExportResponseDTODecodesStagedDownloadId() throws {
        let json = """
        {"serverName":"Modded","serverType":"java","exportKind":"zip","isPaperLike":false,
         "items":[],"selectedCount":1,"zipFileName":"mods.zip","stagedDownloadId":"dl-1"}
        """.data(using: .utf8)!
        let result = try JSONDecoder().decode(ClientExportResponseDTO.self, from: json)
        XCTAssertEqual(result.stagedDownloadId, "dl-1")
        XCTAssertEqual(result.zipFileName, "mods.zip")
    }

    func testAddonUpdateResultDTODecodesOperationId() throws {
        let json = """
        {"result":"update_started","jarStem":"fabric-api","count":1,"operationId":"op-1"}
        """.data(using: .utf8)!
        let result = try JSONDecoder().decode(AddonUpdateResultDTO.self, from: json)
        XCTAssertEqual(result.operationId, "op-1")
    }

    func testCatalogInstallResultDTODecodesOperationIdAndDependencies() throws {
        let json = """
        {"success":true,"message":"Install started.","projectId":"AABBCC",
         "operationId":"op-install","installedDependencies":["dep-1","dep-2"]}
        """.data(using: .utf8)!
        let result = try JSONDecoder().decode(CatalogInstallResultDTO.self, from: json)
        XCTAssertEqual(result.operationId, "op-install")
        XCTAssertEqual(result.installedDependencies ?? [], ["dep-1", "dep-2"])
    }

    func testHealthRepairResultDTODecodesOperationIdWithoutUpdatedProblems() throws {
        let json = """
        {"success":true,"message":"Repair started.","operationId":"op-repair"}
        """.data(using: .utf8)!
        let result = try JSONDecoder().decode(HealthRepairResultDTO.self, from: json)
        XCTAssertEqual(result.operationId, "op-repair")
        XCTAssertNil(result.updated)
    }

    func testDownloadStagedDownloadUsesBearerTokenAndV1Path() async throws {
        MockURLProtocol.responder = { request in
            (200, ["Content-Type": "application/zip"], Data("zip".utf8))
        }
        MockURLProtocol.recorded = []
        let client = try RemoteAPIClient(
            baseURL: URL(string: "http://127.0.0.1:48400")!,
            token: token,
            protocolClasses: [MockURLProtocol.self]
        )

        let data = try await client.downloadStagedDownload(id: "dl-1")

        XCTAssertEqual(data, Data("zip".utf8))
        XCTAssertEqual(MockURLProtocol.recorded.count, 1)
        let request = MockURLProtocol.recorded[0].request
        XCTAssertEqual(request.httpMethod, "GET")
        XCTAssertEqual(request.url?.path, "/v1/staged-downloads/dl-1")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Authorization"), "Bearer \(token)")
    }

    func testUpdateAddonPollsDurableOperationToTerminalState() async throws {
        var polls = 0
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/components/update"):
                return self.jsonResponse([
                    "result": "update_started", "jarStem": "fabric-api", "count": 1, "operationId": "op-addon",
                ], status: 202)
            case ("GET", "/v1/operations/op-addon"):
                polls += 1
                return self.jsonResponse([
                    "id": "op-addon", "type": "addon-update",
                    "state": polls == 1 ? "running" : "succeeded",
                ])
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let result = await vm.updateAddon(baseURL: baseURL, token: token, jarStem: "fabric-api")

        XCTAssertEqual(result, "update_started")
        XCTAssertEqual(vm.activeOperation?.state, .succeeded)
    }

    func testRepairHealthProblemPollsDurableOperationAndRefreshesProblems() async throws {
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/health/repair"):
                return self.jsonResponse([
                    "success": true, "message": "Repair started.", "operationId": "op-repair",
                ], status: 202)
            case ("GET", "/v1/operations/op-repair"):
                return self.jsonResponse([
                    "id": "op-repair", "type": "health-repair", "state": "succeeded",
                ])
            case ("GET", "/v1/health/problems"):
                return self.jsonResponse([
                    "serverType": "java", "serverRunning": false, "isSoftFail": false, "problems": [],
                ])
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let error = await vm.repairHealthProblem(
            baseURL: baseURL,
            token: token,
            problemId: "problem-1",
            action: "update"
        )

        XCTAssertNil(error)
        XCTAssertEqual(vm.activeOperation?.state, .succeeded)
        XCTAssertEqual(vm.healthProblemsResponse?.problems.count, 0)
    }

    private func makeViewModel(
        responder: @escaping (URLRequest) -> (Int, [String: String], Data)
    ) throws -> (DashboardViewModel, URL) {
        MockURLProtocol.responder = responder
        MockURLProtocol.recorded = []
        let baseURL = URL(string: "http://127.0.0.1:48400")!
        let client = try RemoteAPIClient(
            baseURL: baseURL,
            token: token,
            protocolClasses: [MockURLProtocol.self]
        )
        let vm = DashboardViewModel()
        vm.client = client
        vm.clientBaseURL = baseURL
        vm.clientToken = token
        return (vm, baseURL)
    }

    private func jsonResponse(_ object: Any, status: Int = 200) -> (Int, [String: String], Data) {
        (status, ["Content-Type": "application/json"], try! JSONSerialization.data(withJSONObject: object))
    }
}
