import UIKit

// MARK: - Haptic Feedback Helpers
//
// Free functions rather than a View extension because haptics are stateless
// UIKit calls — they don't need access to any view state. Keeping them as
// plain globals makes them callable from anywhere (views, async Task closures,
// view models if needed) without boilerplate.

/// Light tap — use for low-stakes interactions: opening sheets, picker taps,
/// navigating, toggling non-destructive controls.
func hapticLight() {
    let g = UIImpactFeedbackGenerator(style: .light)
    g.prepare()
    g.impactOccurred()
}

/// Success notification — use when an async operation completes successfully:
/// server started, server stopped, pairing saved, test passed.
func hapticSuccess() {
    let g = UINotificationFeedbackGenerator()
    g.prepare()
    g.notificationOccurred(.success)
}

/// Error notification — use when an async operation fails or input is invalid:
/// server failed to start, command send failed, bad QR scan, test failed.
func hapticError() {
    let g = UINotificationFeedbackGenerator()
    g.prepare()
    g.notificationOccurred(.error)
}
