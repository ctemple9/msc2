import XCTest
@testable import MSCRemoteiOS

/// P6.46 proves mutating imports treat `202 Accepted` as a receipt, not
/// completion. The shared `MockURLProtocol` keeps every request local and
/// makes the order of operation polling and post-success refresh observable.
@MainActor
final class Phase6ServerImportOperationTests: XCTestCase {
    private let token = "msc2_testid_testsecret"

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

    private func existingImport(on vm: DashboardViewModel, baseURL: URL) async -> String? {
        await vm.importExistingServer(
            baseURL: baseURL,
            token: token,
            sourcePath: "/imports/server.zip",
            importKind: "zip",
            displayName: "Imported",
            serverType: .java,
            activeWorldName: "world",
            port: 25565,
            maxPlayers: 20,
            acceptEula: true,
            enablePlayit: false
        )
    }

    func testAcceptedImportPollsRunningToSuccessBeforeRefreshing() async throws {
        var operationPolls = 0
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/servers/import"):
                return self.jsonResponse([
                    "success": true, "message": "import_accepted", "operationId": "op-import",
                ], status: 202)
            case ("GET", "/v1/operations/op-import"):
                operationPolls += 1
                let state = operationPolls == 1 ? "running" : "succeeded"
                return self.jsonResponse([
                    "id": "op-import", "type": "server-import", "state": state,
                    "statusLine": state == "running" ? "Copying server files…" : "Import complete.",
                ])
            case ("GET", "/v1/servers"):
                return self.jsonResponse([[
                    "id": "server-1", "name": "Imported", "directory": "/servers/imported",
                    "serverType": "java",
                ]])
            case ("GET", "/v1/status"):
                return self.jsonResponse(["running": false, "activeServerId": "server-1"])
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let error = await existingImport(on: vm, baseURL: baseURL)

        XCTAssertNil(error)
        XCTAssertEqual(vm.activeOperation?.state, .succeeded)
        XCTAssertEqual(vm.servers.first?.id, "server-1")
        XCTAssertEqual(
            MockURLProtocol.recorded.map { $0.request.url?.path ?? "" },
            [
                "/v1/servers/import",
                "/v1/operations/op-import",
                "/v1/operations/op-import",
                "/v1/servers",
                "/v1/status",
            ],
            "the 202 receipt and running snapshot must not refresh or report success before the terminal snapshot"
        )
    }

    func testDurableImportFailureIsSurfacedWithoutRefreshing() async throws {
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/servers/import"):
                return self.jsonResponse([
                    "success": true, "message": "import_accepted", "operationId": "op-failed",
                ], status: 202)
            case ("GET", "/v1/operations/op-failed"):
                return self.jsonResponse([
                    "id": "op-failed", "type": "server-import", "state": "failed",
                    "error": ["code": "copy_failed", "message": "The server folder could not be copied."],
                ])
            default:
                XCTFail("failure must not refresh: \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let error = await existingImport(on: vm, baseURL: baseURL)

        XCTAssertEqual(error, "The server folder could not be copied.")
        XCTAssertEqual(vm.errorMessage, error)
        XCTAssertEqual(vm.activeOperation?.state, .failed)
        XCTAssertEqual(MockURLProtocol.recorded.count, 2)
    }

    func testDurableImportCancellationIsSurfacedWithoutRefreshing() async throws {
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/servers/import"):
                return self.jsonResponse([
                    "success": true, "message": "import_accepted", "operationId": "op-cancelled",
                ], status: 202)
            case ("GET", "/v1/operations/op-cancelled"):
                return self.jsonResponse([
                    "id": "op-cancelled", "type": "server-import", "state": "cancelled",
                    "statusLine": "Import cancelled.",
                ])
            default:
                XCTFail("cancellation must not refresh: \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let error = await existingImport(on: vm, baseURL: baseURL)

        XCTAssertEqual(error, "Import cancelled.")
        XCTAssertEqual(vm.activeOperation?.state, .cancelled)
        XCTAssertTrue(vm.servers.isEmpty)
        XCTAssertEqual(MockURLProtocol.recorded.count, 2)
    }

    func testCompletedLegacyResponseWithoutOperationIdStillRefreshes() async throws {
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/servers/import"):
                return self.jsonResponse([
                    "success": true, "message": "imported", "serverId": "legacy-server",
                ])
            case ("GET", "/v1/servers"):
                return self.jsonResponse([[
                    "id": "legacy-server", "name": "Legacy", "directory": "/servers/legacy",
                    "serverType": "java",
                ]])
            case ("GET", "/v1/status"):
                return self.jsonResponse(["running": false, "activeServerId": "legacy-server"])
            default:
                XCTFail("unexpected request: \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let error = await existingImport(on: vm, baseURL: baseURL)

        XCTAssertNil(error)
        XCTAssertNil(vm.activeOperation)
        XCTAssertEqual(vm.servers.first?.id, "legacy-server")
        XCTAssertEqual(
            MockURLProtocol.recorded.map { $0.request.url?.path ?? "" },
            ["/v1/servers/import", "/v1/servers", "/v1/status"]
        )
    }

    func testTransferImportAlsoFollowsDurableOperation() async throws {
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/servers/import"):
                return self.jsonResponse([
                    "success": true, "message": "import_accepted", "operationId": "op-transfer",
                ], status: 202)
            case ("GET", "/v1/operations/op-transfer"):
                return self.jsonResponse([
                    "id": "op-transfer", "type": "server-import", "state": "succeeded",
                ])
            case ("GET", "/v1/servers"):
                return self.jsonResponse([])
            case ("GET", "/v1/status"):
                return self.jsonResponse(["running": false])
            default:
                XCTFail("unexpected request: \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let error = await vm.importTransferPackage(
            baseURL: baseURL,
            token: token,
            sourcePath: "/imports/server.msctransfer",
            replaceAll: false,
            backupPath: nil
        )

        XCTAssertNil(error)
        XCTAssertEqual(vm.activeOperation?.id, "op-transfer")
        XCTAssertEqual(vm.activeOperation?.state, .succeeded)
        XCTAssertEqual(
            MockURLProtocol.recorded.map { $0.request.url?.path ?? "" },
            ["/v1/servers/import", "/v1/operations/op-transfer", "/v1/servers", "/v1/status"]
        )
    }
}
