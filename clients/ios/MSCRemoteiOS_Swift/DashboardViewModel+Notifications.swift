import Foundation

extension DashboardViewModel {
    func notificationPref(_ key: String, default defaultValue: Bool) -> Bool {
        if let v = UserDefaults.standard.object(forKey: key) as? Bool { return v }
        if let n = UserDefaults.standard.object(forKey: key) as? NSNumber { return n.boolValue }
        return defaultValue
    }

    func evaluateNotifications(
        previousRunning: Bool?,
        previousPlayerNames: Set<String>
    ) {
        let newRunning = status?.running ?? false

        let serverName: String = {
            if let activeId = status?.activeServerId,
               let server = servers.first(where: { $0.id == activeId }) {
                return server.name
            }
            return ""
        }()

        if let prev = previousRunning {
            if prev && !newRunning {
                if notificationPref(SettingsStore.NotificationKeys.serverWentOffline, default: true) {
                    notifications.notifyServerWentOffline(serverName: serverName)
                }
            } else if !prev && newRunning {
                if notificationPref(SettingsStore.NotificationKeys.serverCameOnline, default: true) {
                    notifications.notifyServerCameOnline(serverName: serverName)
                }
            }
        }

        self.previousRunning = newRunning

        if previousRunning != nil {
            let newPlayerNames = Set((players?.players ?? []).map(\.name))
            let joinedNames = newPlayerNames.subtracting(previousPlayerNames)

            if notificationPref(SettingsStore.NotificationKeys.playerJoined, default: false) {
                for name in joinedNames.sorted() {
                    notifications.notifyPlayerJoined(playerName: name, serverName: serverName)
                }
            }

            self.previousPlayerNames = newPlayerNames
        } else {
            self.previousPlayerNames = Set((players?.players ?? []).map(\.name))
        }
    }
}
