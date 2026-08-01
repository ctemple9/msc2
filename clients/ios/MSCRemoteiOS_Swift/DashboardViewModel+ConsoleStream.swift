import Foundation

extension DashboardViewModel {
    func connectConsoleStream(baseURL: URL, token: String) async {
        updateCredentials(baseURL: baseURL, token: token)
        errorMessage = nil
        if isStreamingConsole { return }
        do {
            let client = try requireClient()
            let task = try client.makeConsoleStreamTask()
            consoleStream.removeAll()
            webSocketTask = task
            isStreamingConsole = true
            task.resume()
            webSocketReceiveTask = Task { [weak self] in
                guard let self else { return }
                await self.receiveWebSocketLoop(task: task)
            }
        } catch {
            errorMessage = error.localizedDescription
            isStreamingConsole = false
            webSocketTask = nil
        }
    }

    func disconnectConsoleStream() {
        webSocketReceiveTask?.cancel()
        webSocketReceiveTask = nil
        if let task = webSocketTask {
            task.cancel(with: .goingAway, reason: nil)
        }
        webSocketTask = nil
        isStreamingConsole = false
    }

    func receiveWebSocketLoop(task: URLSessionWebSocketTask) async {
        while !Task.isCancelled {
            do {
                let msg = try await task.receive()
                let text: String?
                switch msg {
                case .string(let s): text = s
                case .data(let d):   text = String(data: d, encoding: .utf8)
                @unknown default:    text = nil
                }
                guard let text, let dto = decodeConsoleLineDTO(from: text) else { continue }
                appendConsoleStreamLine(dto)
            } catch {
                if Task.isCancelled { return }

                let nsErr = error as NSError
                if nsErr.domain == NSURLErrorDomain && nsErr.code == NSURLErrorCancelled {
                    return
                }

                errorMessage = "Console stream ended: \(error.localizedDescription)"
                disconnectConsoleStream()
                return
            }
        }
    }

    func decodeConsoleLineDTO(from text: String) -> ConsoleLineDTO? {
        guard let data = text.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(ConsoleLineDTO.self, from: data)
    }

    func appendConsoleStreamLine(_ dto: ConsoleLineDTO) {
        consoleStream.append(dto)
        if consoleStream.count > 2000 {
            consoleStream.removeFirst(consoleStream.count - 2000)
        }
    }

    func trimConsoleStream(to n: Int) {
        guard consoleStream.count > n else { return }
        consoleStream.removeFirst(consoleStream.count - n)
    }
}
