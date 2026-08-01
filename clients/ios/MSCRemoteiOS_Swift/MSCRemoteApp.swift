import SwiftUI

@main
struct MSCRemoteiOSApp: App {
    @UIApplicationDelegateAdaptor(MSCNotificationDelegate.self) private var notificationDelegate
    @StateObject private var settings = SettingsStore()

    var body: some Scene {
        WindowGroup {
            SplashGateView()
                .environmentObject(settings)
                .preferredColorScheme(.dark)
                .onOpenURL { url in
                    settings.handleIncomingURL(url)
                }
        }
    }

    init() {
        // Request notification permission once at launch. The system shows
        // the permission sheet on first run only — subsequent launches are
        // silently no-ops if the user has already decided.
        //
        // We call this in init() rather than in a .onAppear modifier because:
        // 1. It fires before any view appears, so the splash doesn't have to
        //    wait for it.
        // 2. It avoids the risk of it being called multiple times if the root
        //    view re-appears (e.g., during scene lifecycle changes).
        // 3. init() runs exactly once for the lifetime of the app process.
        NotificationManager.shared.requestPermission()
    }
}
