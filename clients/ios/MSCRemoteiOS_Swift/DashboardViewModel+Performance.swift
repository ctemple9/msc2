import Foundation

extension DashboardViewModel {
    func fetchPerformanceSnapshot(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let client = try requireClient()
            let snap = try await client.getPerformanceSnapshot()
            let point = PerformancePoint(
                timestamp: Date(),
                tps1m: snap.tps1m,
                playersOnline: snap.playersOnline,
                cpuPercent: snap.cpuPercent,
                ramUsedMB: snap.ramUsedMB,
                ramMaxMB: snap.ramMaxMB,
                worldSizeMB: snap.worldSizeMB
            )
            performanceLatest = point
            performanceHistory.append(point)
            if performanceHistory.count > maxPerformanceSamples {
                performanceHistory.removeFirst(performanceHistory.count - maxPerformanceSamples)
            }
            performanceErrorMessage = nil
        } catch let err as RemoteAPIError {
            switch err {
            case .httpStatus(let code, _):
                if code == 404 {
                    performanceErrorMessage = "Performance endpoint not available yet. (Update macOS Remote API to support /performance.)"
                } else {
                    performanceErrorMessage = err.localizedDescription
                }
            default:
                performanceErrorMessage = err.localizedDescription
            }
        } catch {
            performanceErrorMessage = error.localizedDescription
        }
    }

    func fetchPlayers(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        do {
            let result = try await requireClient().getPlayers()
            players = result
        } catch let err as RemoteAPIError {
            if case .httpStatus(404, _) = err { return }
        } catch {
        }
    }
}
