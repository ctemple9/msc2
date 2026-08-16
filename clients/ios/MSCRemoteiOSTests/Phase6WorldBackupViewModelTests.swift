import XCTest
@testable import MSCRemoteiOS

/// P6.24 view-model tests. `Phase6WorldBackupAPITests.swift` (P6.23)
/// already proves the wire-level request/response shapes; this file
/// proves `DashboardViewModel`'s own layer on top: the permission gate
/// `WorldsView`/`ServerView` use to show destructive actions
/// (`hasPermission`), and that every new P6.24 view-model method
/// publishes the right `@Published` state on success and failure --
/// including the operation-polling path `activeOperation` feeds
/// `ConvertWorldView`'s progress UI from.
///
/// Reuses `Phase6WorldBackupAPITests.swift`'s `MockURLProtocol` (same
/// test target, not `private`) rather than redeclaring it.
@MainActor
final class Phase6WorldBackupViewModelTests: XCTestCase {

    // MARK: - hasPermission: the "existing device-auth protection" gate
    //
    // Marked `async` although nothing here awaits anything: a bare
    // synchronous test method on an `@MainActor` `XCTestCase` subclass
    // can be invoked by XCTest's Objective-C runner without actually
    // hopping onto the MainActor executor first, so touching a
    // `@MainActor` type (`DashboardViewModel`) from it races the real
    // app-under-test's own genuine MainActor work (its splash/status-
    // polling `.task`s) and reproducibly corrupts the heap (`malloc:
    // pointer being freed was not allocated`, confirmed independent of
    // test content or added delay). `async` forces proper actor-isolated
    // invocation through Swift's structured-concurrency calling
    // convention instead.

    func testHasPermissionTrueForAdminRegardlessOfPermissionsList() async {
        let vm = DashboardViewModel()
        vm.connectedRole = "admin"
        vm.connectedPermissions = []
        XCTAssertTrue(vm.hasPermission("worlds"))
    }

    func testHasPermissionFalseForGuestRegardlessOfPermissionsList() async {
        let vm = DashboardViewModel()
        vm.connectedRole = "guest"
        vm.connectedPermissions = ["worlds"]
        XCTAssertFalse(vm.hasPermission("worlds"))
    }

    func testHasPermissionForNamedTokenFollowsItsGrantedList() async {
        let vm = DashboardViewModel()
        vm.connectedRole = "named"
        vm.connectedPermissions = ["worlds", "settings"]
        XCTAssertTrue(vm.hasPermission("worlds"))
        XCTAssertFalse(vm.hasPermission("addons"))
    }

    func testHasPermissionFalseWithNoConnectedRoleYet() async {
        let vm = DashboardViewModel()
        vm.connectedRole = nil
        XCTAssertFalse(vm.hasPermission("worlds"))
    }

    // MARK: - Test harness: inject a MockURLProtocol-backed client

    private func makeViewModel(responder: @escaping (URLRequest) -> (Int, [String: String], Data)) throws -> (DashboardViewModel, URL) {
        MockURLProtocol.responder = responder
        MockURLProtocol.recorded = []
        let baseURL = URL(string: "http://127.0.0.1:48400")!
        let client = try RemoteAPIClient(baseURL: baseURL, token: "msc2_testid_testsecret",
                                         protocolClasses: [MockURLProtocol.self])
        let vm = DashboardViewModel()
        // `updateCredentials` only rebuilds `client` when baseURL/token
        // differ from what's already cached -- pre-seeding all three
        // with matching values makes every view-model method's own
        // `updateCredentials` call a no-op, so it reuses this
        // mock-backed client instead of constructing a real one.
        vm.client = client
        vm.clientBaseURL = baseURL
        vm.clientToken = "msc2_testid_testsecret"
        return (vm, baseURL)
    }

    private func jsonResponse(_ object: Any, status: Int = 200) -> (Int, [String: String], Data) {
        (status, ["Content-Type": "application/json"], try! JSONSerialization.data(withJSONObject: object))
    }

    // MARK: - World management: success refreshes worldsResponse, failure sets errorMessage

    func testDeleteWorldSlotSuccessRefreshesWorldsResponseAndClearsError() async throws {
        let (vm, baseURL) = try makeViewModel { _ in
            self.jsonResponse([
                "success": true,
                "message": "deleted",
                "updated": ["slots": [], "serverRunning": false],
            ])
        }
        vm.errorMessage = "stale error from a previous call"

        let err = await vm.deleteWorldSlot(baseURL: baseURL, token: "msc2_testid_testsecret", slotId: "slot-1")

        XCTAssertNil(err)
        XCTAssertNil(vm.errorMessage)
        XCTAssertEqual(vm.worldsResponse?.slots.count, 0)
    }

    func testDeleteWorldSlotFailureSurfacesTransportError() async throws {
        let (vm, baseURL) = try makeViewModel { _ in
            (409, [:], try! JSONSerialization.data(withJSONObject: ["code": "conflict", "message": "server is running"]))
        }

        let err = await vm.deleteWorldSlot(baseURL: baseURL, token: "msc2_testid_testsecret", slotId: "slot-1")

        XCTAssertNotNil(err)
        XCTAssertTrue(err?.contains("server is running") ?? false, "\(err ?? "nil")")
    }

    func testDuplicateWorldSlotSuccessRefreshesWorldsResponse() async throws {
        let (vm, baseURL) = try makeViewModel { _ in
            self.jsonResponse([
                "success": true,
                "message": "duplicated",
                "updated": ["slots": [
                    ["id": "slot-1", "name": "Survival", "isActive": true, "createdAt": "2026-01-01T00:00:00Z"],
                    ["id": "slot-2", "name": "Survival copy", "isActive": false, "createdAt": "2026-01-02T00:00:00Z"],
                ], "serverRunning": false],
            ])
        }

        let err = await vm.duplicateWorldSlot(baseURL: baseURL, token: "msc2_testid_testsecret", slotId: "slot-1")

        XCTAssertNil(err)
        XCTAssertEqual(vm.worldsResponse?.slots.count, 2)
    }

    func testImportWorldZipUploadsBytesAndRefreshesWorldsResponse() async throws {
        let fileBytes = Data("PK\u{03}\u{04} fake zip".utf8)
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/staged-uploads"):
                return self.jsonResponse([
                    "stagedUploadId": "up-1", "uploadPath": "/v1/staged-uploads/up-1",
                    "expiresAt": "2026-01-01T00:30:00Z", "maxBytes": 10_737_418_240,
                ])
            case ("PUT", "/v1/staged-uploads/up-1"):
                return self.jsonResponse(["stagedUploadId": "up-1", "receivedBytes": fileBytes.count, "sha256": "abc"])
            case ("POST", "/v1/worlds/import"):
                return self.jsonResponse([
                    "success": true, "message": "imported",
                    "updated": ["slots": [["id": "slot-3", "name": "Imported", "isActive": false, "createdAt": "2026-01-03T00:00:00Z"]],
                                "serverRunning": false],
                ])
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let err = await vm.importWorldZip(baseURL: baseURL, token: "msc2_testid_testsecret", name: "Imported", data: fileBytes)

        XCTAssertNil(err)
        XCTAssertEqual(vm.worldsResponse?.slots.first?.name, "Imported")
    }

    func testExportWorldSlotReturnsDownloadedBytes() async throws {
        let zipBytes = Data("PK\u{03}\u{04} exported".utf8)
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/worlds/export"):
                return self.jsonResponse(["stagedDownloadId": "dl-1", "expiresAt": "2026-01-01T00:30:00Z", "sizeBytes": zipBytes.count])
            case ("GET", "/v1/staged-downloads/dl-1"):
                return (200, ["Content-Type": "application/zip"], zipBytes)
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let downloaded = await vm.exportWorldSlot(baseURL: baseURL, token: "msc2_testid_testsecret", slotId: "slot-1")

        XCTAssertEqual(downloaded, zipBytes)
        XCTAssertNil(vm.errorMessage)
    }

    func testExportWorldSlotFailureSetsErrorMessageAndReturnsNil() async throws {
        let (vm, baseURL) = try makeViewModel { _ in
            (404, [:], try! JSONSerialization.data(withJSONObject: ["code": "not_found", "message": "no such slot"]))
        }

        let downloaded = await vm.exportWorldSlot(baseURL: baseURL, token: "msc2_testid_testsecret", slotId: "missing")

        XCTAssertNil(downloaded)
        XCTAssertNotNil(vm.errorMessage)
    }

    func testDeleteBackupReturnsTrueOnSuccessAndFalseOnFailure() async throws {
        let (successVM, baseURL) = try makeViewModel { _ in self.jsonResponse(["result": "deleted"]) }
        let successResult = await successVM.deleteBackup(baseURL: baseURL, token: "msc2_testid_testsecret", backupId: "b1.zip")
        XCTAssertTrue(successResult)

        let (failureVM, failureBaseURL) = try makeViewModel { _ in
            (409, [:], try! JSONSerialization.data(withJSONObject: ["code": "sole_verified_backup", "message": "cannot delete the last verified backup"]))
        }
        let failureResult = await failureVM.deleteBackup(baseURL: failureBaseURL, token: "msc2_testid_testsecret", backupId: "b1.zip")
        XCTAssertFalse(failureResult)
        XCTAssertNotNil(failureVM.errorMessage)
    }

    // MARK: - Operation polling: convertWorld publishes activeOperation and cancelOperation

    func testConvertWorldPublishesActiveOperationThroughToTerminalState() async throws {
        var pollCount = 0
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/worlds/convert"):
                return self.jsonResponse(["result": "conversion_started", "operationId": "op-1"])
            case ("GET", "/v1/operations/op-1"):
                pollCount += 1
                let state = pollCount < 2 ? "running" : "succeeded"
                return self.jsonResponse(["id": "op-1", "type": "world-conversion", "state": state])
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }
        XCTAssertNil(vm.activeOperation)

        let terminal = await vm.convertWorld(baseURL: baseURL, token: "msc2_testid_testsecret",
                                             sourceSlotId: "slot-1", targetServerId: "srv-2",
                                             targetFormat: "bedrock", targetName: "Converted", targetSlotId: nil)

        XCTAssertEqual(terminal?.state, .succeeded)
        XCTAssertEqual(vm.activeOperation?.state, .succeeded)
        XCTAssertEqual(vm.activeOperation?.id, "op-1")
    }

    func testConvertWorldFailureLeavesFailedOperationInActiveOperation() async throws {
        let (vm, baseURL) = try makeViewModel { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/worlds/convert"):
                return self.jsonResponse(["result": "conversion_started", "operationId": "op-2"])
            case ("GET", "/v1/operations/op-2"):
                return self.jsonResponse([
                    "id": "op-2", "type": "world-conversion", "state": "failed",
                    "error": ["code": "world_error", "message": "Chunker is not installed."],
                ])
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let terminal = await vm.convertWorld(baseURL: baseURL, token: "msc2_testid_testsecret",
                                             sourceSlotId: "slot-1", targetServerId: "srv-2",
                                             targetFormat: "bedrock", targetName: "Converted", targetSlotId: nil)

        XCTAssertEqual(terminal?.state, .failed)
        XCTAssertEqual(vm.activeOperation?.error?.message, "Chunker is not installed.")
    }

    func testCancelOperationReturnsTrueOnSuccessAndFalseOnFailure() async throws {
        let (successVM, baseURL) = try makeViewModel { _ in
            self.jsonResponse(["id": "op-1", "type": "world-conversion", "state": "cancelled"])
        }
        let ok = await successVM.cancelOperation(baseURL: baseURL, token: "msc2_testid_testsecret", operationId: "op-1")
        XCTAssertTrue(ok)

        let (failureVM, failureBaseURL) = try makeViewModel { _ in
            (409, [:], try! JSONSerialization.data(withJSONObject: ["code": "conflict", "message": "already terminal"]))
        }
        let failed = await failureVM.cancelOperation(baseURL: failureBaseURL, token: "msc2_testid_testsecret", operationId: "op-1")
        XCTAssertFalse(failed)
    }
}
