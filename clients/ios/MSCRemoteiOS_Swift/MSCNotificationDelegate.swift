import UIKit
import UserNotifications

/// Installs a UNUserNotificationCenterDelegate for MSC Remote.
///
/// By default, iOS will not present banners/sounds for notifications while the
/// app is in the foreground unless a delegate returns presentation options.
/// MSC Remote schedules *local* notifications (not push), so without this
/// delegate it can look like "notifications never fire" while you're actively
/// using the app.
final class MSCNotificationDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        UNUserNotificationCenter.current().delegate = self
        return true
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .list, .sound])
    }
}
