import Foundation
import UIKit
import UserNotifications

// MARK: - NotificationManager
//
// Wraps UNUserNotificationCenter for all MSC Remote local notification work.
//
// Design decisions:
//
// Singleton pattern is appropriate here because UNUserNotificationCenter itself
// is a device-global singleton. Wrapping it in a singleton prevents duplicate
// permission requests and keeps call sites simple.
//
// One-way dependency: DashboardViewModel calls into NotificationManager.
// NotificationManager never calls back out. That strict one-way flow means
// this class has no knowledge of views, settings, or network state — it just
// schedules notifications when asked.
//
// Per-event-type enable/disable is handled in the *caller* (DashboardViewModel),
// not here. NotificationManager's job is "schedule this notification correctly."
// Whether to call it at all is the ViewModel's concern. This separation of
// concerns keeps both classes simpler.

@MainActor
final class NotificationManager {

    static let shared = NotificationManager()
    private init() {}

    // MARK: - Permission

    /// Call once at app launch. Shows the system permission sheet if the user
    /// hasn't been asked yet. Safe to call on subsequent launches — the system
    /// is a no-op once a decision has been made.
    ///
    /// We request .alert + .sound + .badge together because all three are
    /// needed for a useful urgent notification. Requesting less would produce
    /// a worse experience — a silent notification about a crashed server is
    /// too easy to miss.
    func requestPermission() {
        UNUserNotificationCenter.current().requestAuthorization(
            options: [.alert, .sound, .badge]
        ) { _, _ in
            // We intentionally don't cache this result. Before every
            // schedule() call we re-query the live authorization status.
            // Caching would go stale if the user changes the permission in
            // iOS Settings between launches.
        }
    }

    // MARK: - Event kinds
    //
    // Typed enum prevents typo bugs at call sites and makes it easy to add
    // new event types in the future. The rawValue string becomes the
    // UNNotificationRequest identifier — this drives deduplication (see
    // schedule() below).

    enum EventKind: String {
        case serverWentOffline = "msc.server.offline"
        case serverCameOnline  = "msc.server.online"
        case playerJoined      = "msc.player.joined"
    }

    // MARK: - Scheduling API

    func notifyServerWentOffline(serverName: String) {
        let name = serverName.isEmpty ? "Server" : serverName
        schedule(
            kind: .serverWentOffline,
            title: "⚠️ \(name) went offline",
            body: "Your Minecraft server has stopped. Tap to check the dashboard."
        )
    }

    func notifyServerCameOnline(serverName: String) {
        let name = serverName.isEmpty ? "Server" : serverName
        schedule(
            kind: .serverCameOnline,
            title: "✅ \(name) is online",
            body: "Your Minecraft server is up and running."
        )
    }

    func notifyPlayerJoined(playerName: String, serverName: String) {
        let server = serverName.isEmpty ? "the server" : serverName
        schedule(
            kind: .playerJoined,
            title: "🧑‍💻 \(playerName) joined",
            body: "\(playerName) connected to \(server)."
        )
    }

    // MARK: - Internal

    private func schedule(kind: EventKind, title: String, body: String) {
        // Only present alerts when the app is not active.
        // When the user is inside MSC Remote, the dashboard already reflects state.
        guard UIApplication.shared.applicationState != .active else { return }

        // Always check live authorization status. Never assume.
        UNUserNotificationCenter.current().getNotificationSettings { settings in
            guard settings.authorizationStatus == .authorized ||
                  settings.authorizationStatus == .provisional else { return }

            let content = UNMutableNotificationContent()
            content.title = title
            content.body = body
            content.sound = .default

            // timeInterval must be > 0. 0.5s is imperceptible to the user.
            let trigger = UNTimeIntervalNotificationTrigger(timeInterval: 0.5, repeats: false)

            // Using EventKind.rawValue as the identifier means a second
            // notification of the same type *replaces* the pending one
            // instead of stacking. This prevents pile-ups if the server
            // rapidly flaps between polls.
            let request = UNNotificationRequest(
                identifier: kind.rawValue,
                content: content,
                trigger: trigger
            )

            UNUserNotificationCenter.current().add(request) { error in
                if let error {
                    // Scheduling failures are non-fatal. The app continues
                    // working; the user just misses this one notification.
                    #if DEBUG
                    print("[NotificationManager] Schedule failed (\(kind.rawValue)): \(error)")
                    #endif
                }
            }
        }
    }
}

