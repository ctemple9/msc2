import XCTest
@testable import MSCRemoteiOS

/// P6.23 networking tests. Two things this file exists to prove, per
/// `rolling-plan.md`'s own P6.23 step text: every Phase 6 world/backup DTO
/// decodes both a preserved baseline payload and an additive Phase 6
/// payload, and the auth (`Authorization: Bearer`) and version (`/v1/`)
/// headers stay attached on every request shape this step adds --
/// including the two non-JSON ones (staged upload/download raw bytes).
///
/// No live agent: `MockURLProtocol` intercepts every request `RemoteAPIClient`
/// issues so these tests run offline and deterministically.
final class Phase6WorldBackupAPITests: XCTestCase {

    // MARK: - Model decoding: preserved baseline + additive Phase 6 payloads

    func testWorldSlotDTODecodesBaselinePayloadWithoutHasThumbnail() throws {
        let json = """
        {"id":"slot-1","name":"Survival","isActive":true,"createdAt":"2026-01-01T00:00:00Z"}
        """.data(using: .utf8)!
        let slot = try JSONDecoder().decode(WorldSlotDTO.self, from: json)
        XCTAssertEqual(slot.id, "slot-1")
        XCTAssertEqual(slot.name, "Survival")
        XCTAssertTrue(slot.isActive)
        XCTAssertNil(slot.zipSizeBytes)
        XCTAssertNil(slot.worldSeed)
        XCTAssertNil(slot.hasThumbnail, "a baseline payload predating Phase 6 must still decode")
    }

    func testWorldSlotDTODecodesAdditivePhase6PayloadWithHasThumbnail() throws {
        let json = """
        {"id":"slot-2","name":"Creative","isActive":false,"createdAt":"2026-01-02T00:00:00Z",
         "zipSizeBytes":12345,"worldSeed":"42","hasThumbnail":true}
        """.data(using: .utf8)!
        let slot = try JSONDecoder().decode(WorldSlotDTO.self, from: json)
        XCTAssertEqual(slot.zipSizeBytes, 12345)
        XCTAssertEqual(slot.worldSeed, "42")
        XCTAssertEqual(slot.hasThumbnail, true)
    }

    func testWorldSlotsResponseDTODecodesBaselinePayloadWithoutIsRepairing() throws {
        let json = """
        {"slots":[],"serverRunning":false}
        """.data(using: .utf8)!
        let response = try JSONDecoder().decode(WorldSlotsResponseDTO.self, from: json)
        XCTAssertNil(response.activeSlotId)
        XCTAssertNil(response.isRepairing)
    }

    func testWorldExportResultDTODecodes() throws {
        let json = """
        {"stagedDownloadId":"dl-1","expiresAt":"2026-01-01T00:30:00Z","sizeBytes":9001}
        """.data(using: .utf8)!
        let result = try JSONDecoder().decode(WorldExportResultDTO.self, from: json)
        XCTAssertEqual(result.stagedDownloadId, "dl-1")
        XCTAssertEqual(result.sizeBytes, 9001)
    }

    func testStagedUploadBeginResultDTODecodes() throws {
        let json = """
        {"stagedUploadId":"up-1","uploadPath":"/v1/staged-uploads/up-1",
         "expiresAt":"2026-01-01T00:30:00Z","maxBytes":10737418240}
        """.data(using: .utf8)!
        let result = try JSONDecoder().decode(StagedUploadBeginResultDTO.self, from: json)
        XCTAssertEqual(result.uploadPath, "/v1/staged-uploads/up-1")
        XCTAssertEqual(result.maxBytes, 10_737_418_240)
    }

    func testStagedUploadCompleteResultDTODecodes() throws {
        let json = """
        {"stagedUploadId":"up-1","receivedBytes":42,"sha256":"deadbeef"}
        """.data(using: .utf8)!
        let result = try JSONDecoder().decode(StagedUploadCompleteResultDTO.self, from: json)
        XCTAssertEqual(result.receivedBytes, 42)
        XCTAssertEqual(result.sha256, "deadbeef")
    }

    func testWorldConvertRequestDTOEncodesExactlyOneOfTargetNameOrTargetSlotId() throws {
        let byName = WorldConvertRequestDTO(sourceSlotId: "src", targetServerId: "srv-2",
                                            targetFormat: "bedrock", targetName: "New World", targetSlotId: nil)
        let dataByName = try JSONEncoder().encode(byName)
        let objByName = try JSONSerialization.jsonObject(with: dataByName) as? [String: Any]
        XCTAssertEqual(objByName?["targetName"] as? String, "New World")
        XCTAssertNil(objByName?["targetSlotId"], "an absent targetSlotId must be omitted, not encoded as null")

        let bySlot = WorldConvertRequestDTO(sourceSlotId: "src", targetServerId: "srv-2",
                                            targetFormat: "bedrock", targetName: nil, targetSlotId: "slot-9")
        let dataBySlot = try JSONEncoder().encode(bySlot)
        let objBySlot = try JSONSerialization.jsonObject(with: dataBySlot) as? [String: Any]
        XCTAssertEqual(objBySlot?["targetSlotId"] as? String, "slot-9")
        XCTAssertNil(objBySlot?["targetName"])
    }

    func testWorldConvertResultDTODecodesRequiredOperationId() throws {
        let json = """
        {"result":"conversion_started","operationId":"op-1"}
        """.data(using: .utf8)!
        let result = try JSONDecoder().decode(WorldConvertResultDTO.self, from: json)
        XCTAssertEqual(result.operationId, "op-1")
    }

    func testStagedUploadPurposeDTOEncodesKebabCase() throws {
        let body = StagedUploadBeginRequestDTO(purpose: .worldImport, contentType: nil)
        let data = try JSONEncoder().encode(body)
        let obj = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(obj?["purpose"] as? String, "world-import")
        XCTAssertNil(obj?["contentType"], "a nil contentType must be omitted, matching the server's skip_serializing_if")
    }

    // MARK: - Operation model decoding

    func testOperationDTODecodesFullShapeWithProgressAndError() throws {
        let json = """
        {"id":"op-1","type":"world-activate","target":"srv-1","state":"failed",
         "progress":{"current":2,"total":5},"statusLine":"Activating world slot.",
         "result":null,"error":{"code":"world_error","message":"boom","helpId":"h1"}}
        """.data(using: .utf8)!
        let op = try JSONDecoder().decode(OperationDTO.self, from: json)
        XCTAssertEqual(op.state, .failed)
        XCTAssertEqual(op.progress?.current, 2)
        XCTAssertEqual(op.progress?.total, 5)
        XCTAssertEqual(op.statusLine, "Activating world slot.")
        XCTAssertEqual(op.error?.code, "world_error")
        XCTAssertEqual(op.error?.helpId, "h1")
    }

    func testOperationDTODecodesFlatStringMapResult() throws {
        let json = """
        {"id":"op-2","type":"backup-now","state":"succeeded","result":{"result":"backup_created"}}
        """.data(using: .utf8)!
        let op = try JSONDecoder().decode(OperationDTO.self, from: json)
        XCTAssertEqual(op.state, .succeeded)
        XCTAssertEqual(op.result?["result"], "backup_created")
    }

    func testOperationDTOToleratesUnexpectedResultShapeWithoutFailingDecode() throws {
        // A `result` this client doesn't narrowly expect (nested object,
        // not a flat string map) must not break decoding the whole
        // operation record -- `state`/`statusLine` still matter mid-poll
        // even when `result`'s exact shape is something this client
        // doesn't parse further.
        let json = """
        {"id":"op-3","type":"world-conversion","state":"running",
         "result":{"nested":{"a":1}}}
        """.data(using: .utf8)!
        let op = try JSONDecoder().decode(OperationDTO.self, from: json)
        XCTAssertEqual(op.state, .running)
        XCTAssertNil(op.result)
    }

    func testOperationStateDTORoundTripsEveryCase() throws {
        for state in [OperationStateDTO.queued, .running, .succeeded, .failed, .cancelled] {
            let data = try JSONEncoder().encode(state)
            let decoded = try JSONDecoder().decode(OperationStateDTO.self, from: data)
            XCTAssertEqual(decoded, state)
        }
    }

    func testErrorDTODecodesWithAndWithoutHelpId() throws {
        let withHelp = try JSONDecoder().decode(ErrorDTO.self, from: """
        {"code":"not_found","message":"missing","helpId":"h1"}
        """.data(using: .utf8)!)
        XCTAssertEqual(withHelp.helpId, "h1")

        let withoutHelp = try JSONDecoder().decode(ErrorDTO.self, from: """
        {"code":"not_found","message":"missing"}
        """.data(using: .utf8)!)
        XCTAssertNil(withoutHelp.helpId)
    }

    // MARK: - RemoteAPIClient: auth + version headers, real request shapes

    private func makeClient(responder: @escaping (URLRequest) -> (Int, [String: String], Data)) throws -> RemoteAPIClient {
        MockURLProtocol.responder = responder
        MockURLProtocol.recorded = []
        return try RemoteAPIClient(baseURL: URL(string: "http://127.0.0.1:48400")!,
                                   token: "msc2_testid_testsecret",
                                   protocolClasses: [MockURLProtocol.self])
    }

    private func jsonResponse(_ object: Any, status: Int = 200) -> (Int, [String: String], Data) {
        (status, ["Content-Type": "application/json"], try! JSONSerialization.data(withJSONObject: object))
    }

    func testGetWorldsAttachesBearerTokenAndV1PathPrefix() async throws {
        let client = try makeClient { _ in
            self.jsonResponse(["slots": [], "serverRunning": false])
        }
        _ = try await client.getWorlds()

        XCTAssertEqual(MockURLProtocol.recorded.count, 1)
        let request = MockURLProtocol.recorded[0].request
        XCTAssertEqual(request.httpMethod, "GET")
        XCTAssertEqual(request.url?.path, "/v1/worlds")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Authorization"), "Bearer msc2_testid_testsecret")
    }

    func testCreateWorldPostsJSONBodyWithAuthAndVersionHeaders() async throws {
        let client = try makeClient { _ in
            self.jsonResponse(["success": true, "message": "created", "updated": NSNull()])
        }
        _ = try await client.createWorld(name: "Survival", seed: "42")

        let request = MockURLProtocol.recorded[0].request
        let body = MockURLProtocol.recorded[0].body
        XCTAssertEqual(request.httpMethod, "POST")
        XCTAssertEqual(request.url?.path, "/v1/worlds/create")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Authorization"), "Bearer msc2_testid_testsecret")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Content-Type"), "application/json")
        let obj = try JSONSerialization.jsonObject(with: body) as? [String: Any]
        XCTAssertEqual(obj?["name"] as? String, "Survival")
        XCTAssertEqual(obj?["seed"] as? String, "42")
    }

    /// The one transport this step adds that isn't JSON both ways: begin
    /// a staged upload (JSON), `PUT` the raw ZIP bytes (not JSON), then
    /// redeem it (JSON). Proves the `Authorization`/`/v1/` prefix survive
    /// on the raw-bytes leg too, and -- the actual regression this test
    /// exists to catch -- that the server-returned `uploadPath` (already
    /// `/v1/...`-prefixed) does not get double-prefixed to `/v1/v1/...`.
    func testImportWorldZipRoundTripsRawBytesWithCorrectPathAndHeaders() async throws {
        let fileBytes = Data("PK\u{03}\u{04} fake zip bytes".utf8)
        let client = try makeClient { request in
            switch (request.httpMethod, request.url?.path) {
            case ("POST", "/v1/staged-uploads"):
                return self.jsonResponse([
                    "stagedUploadId": "up-1",
                    "uploadPath": "/v1/staged-uploads/up-1",
                    "expiresAt": "2026-01-01T00:30:00Z",
                    "maxBytes": 10_737_418_240,
                ])
            case ("PUT", "/v1/staged-uploads/up-1"):
                return self.jsonResponse(["stagedUploadId": "up-1", "receivedBytes": fileBytes.count, "sha256": "abc"])
            case ("POST", "/v1/worlds/import"):
                return self.jsonResponse(["success": true, "message": "imported", "updated": NSNull()])
            default:
                XCTFail("unexpected request: \(request.httpMethod ?? "?") \(request.url?.path ?? "?")")
                return (404, [:], Data())
            }
        }

        let result = try await client.importWorldZip(name: "Imported", data: fileBytes)
        XCTAssertTrue(result.success)

        XCTAssertEqual(MockURLProtocol.recorded.count, 3)
        let putRequest = MockURLProtocol.recorded[1].request
        let putBody = MockURLProtocol.recorded[1].body
        XCTAssertEqual(putRequest.url?.path, "/v1/staged-uploads/up-1", "must not double the /v1 prefix the server already returned")
        XCTAssertEqual(putRequest.value(forHTTPHeaderField: "Authorization"), "Bearer msc2_testid_testsecret")
        XCTAssertEqual(putRequest.value(forHTTPHeaderField: "Content-Type"), "application/octet-stream")
        XCTAssertEqual(putBody, fileBytes, "the exact local file bytes must be uploaded verbatim")
    }

    func testExportWorldSlotDownloadsRawBytesWithAuthHeader() async throws {
        let zipBytes = Data("PK\u{03}\u{04} exported zip bytes".utf8)
        let client = try makeClient { request in
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

        let downloaded = try await client.exportWorldSlot(slotId: "slot-1")
        XCTAssertEqual(downloaded, zipBytes)

        let getRequest = MockURLProtocol.recorded[1].request
        XCTAssertEqual(getRequest.value(forHTTPHeaderField: "Authorization"), "Bearer msc2_testid_testsecret")
    }

    func testConvertWorldRejectsNeitherOrBothTargetFields() async throws {
        let client = try makeClient { _ in self.jsonResponse(["result": "x", "operationId": "op-1"]) }

        do {
            _ = try await client.convertWorld(sourceSlotId: "s", targetServerId: "t", targetFormat: "bedrock",
                                              targetName: nil, targetSlotId: nil)
            XCTFail("expected a validation error when neither target field is given")
        } catch {
            XCTAssertTrue(MockURLProtocol.recorded.isEmpty, "an invalid request must never reach the network")
        }
    }

    func testGetOperationAndCancelOperationAttachAuthHeaderAndCorrectPaths() async throws {
        let client = try makeClient { request in
            self.jsonResponse([
                "id": "op-1", "type": "world-activate", "state": "running",
            ])
        }
        _ = try await client.getOperation(id: "op-1")
        XCTAssertEqual(MockURLProtocol.recorded[0].request.url?.path, "/v1/operations/op-1")
        XCTAssertEqual(MockURLProtocol.recorded[0].request.httpMethod, "GET")

        _ = try await client.cancelOperation(id: "op-1")
        XCTAssertEqual(MockURLProtocol.recorded[1].request.url?.path, "/v1/operations/op-1/cancel")
        XCTAssertEqual(MockURLProtocol.recorded[1].request.httpMethod, "POST")
        XCTAssertEqual(MockURLProtocol.recorded[1].request.value(forHTTPHeaderField: "Authorization"), "Bearer msc2_testid_testsecret")
    }

    func testPollOperationToTerminalPollsUntilSucceededAndReportsEveryUpdate() async throws {
        var callCount = 0
        let client = try makeClient { _ in
            callCount += 1
            let state = callCount < 3 ? "running" : "succeeded"
            return self.jsonResponse(["id": "op-1", "type": "backup-now", "state": state])
        }

        var observedStates: [OperationStateDTO] = []
        let final = try await client.pollOperationToTerminal(id: "op-1") { update in
            observedStates.append(update.state)
        }

        XCTAssertEqual(final.state, .succeeded)
        XCTAssertEqual(observedStates, [.running, .running, .succeeded])
        XCTAssertEqual(MockURLProtocol.recorded.count, 3)
    }

    func testDeleteBackupPostsCorrectBodyAndPath() async throws {
        let client = try makeClient { _ in self.jsonResponse(["result": "deleted"]) }
        _ = try await client.deleteBackup(backupId: "backup-1.zip")

        let request = MockURLProtocol.recorded[0].request
        let body = MockURLProtocol.recorded[0].body
        XCTAssertEqual(request.url?.path, "/v1/backups/delete")
        let obj = try JSONSerialization.jsonObject(with: body) as? [String: Any]
        XCTAssertEqual(obj?["backupId"] as? String, "backup-1.zip")
    }
}

// MARK: - Mock URL protocol

/// Intercepts every request `RemoteAPIClient` issues in this file's tests,
/// so they run offline and deterministically -- no live agent needed.
/// Registered directly on the client's own `URLSessionConfiguration` via
/// `RemoteAPIClient`'s test-only `protocolClasses:` initializer parameter
/// rather than the deprecated global `URLProtocol.registerClass`.
final class MockURLProtocol: URLProtocol {
    nonisolated(unsafe) static var responder: ((URLRequest) -> (Int, [String: String], Data))?
    nonisolated(unsafe) static var recorded: [(request: URLRequest, body: Data)] = []

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        let body = Self.extractBody(from: request)
        MockURLProtocol.recorded.append((request, body))

        guard let responder = MockURLProtocol.responder else {
            client?.urlProtocol(self, didFailWithError: URLError(.unsupportedURL))
            return
        }
        let (status, headers, data) = responder(request)
        let response = HTTPURLResponse(url: request.url!, statusCode: status, httpVersion: "HTTP/1.1", headerFields: headers)!
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        client?.urlProtocol(self, didLoad: data)
        client?.urlProtocolDidFinishLoading(self)
    }

    override func stopLoading() {}

    private static func extractBody(from request: URLRequest) -> Data {
        if let body = request.httpBody {
            return body
        }
        guard let stream = request.httpBodyStream else { return Data() }
        stream.open()
        defer { stream.close() }
        var data = Data()
        let bufferSize = 4096
        var buffer = [UInt8](repeating: 0, count: bufferSize)
        while stream.hasBytesAvailable {
            let read = stream.read(&buffer, maxLength: bufferSize)
            guard read > 0 else { break }
            data.append(buffer, count: read)
        }
        return data
    }
}
