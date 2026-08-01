import SwiftUI
import AVKit
import AVFoundation
import Combine

/// Splash on cold launch only, then show RootView.
/// - Plays `splash_intro.(mp4|mov|m4v)` once and dismisses when it ends.
/// - If the video is missing, shows a fallback briefly then dismisses.
struct SplashGateView: View {

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isShowingSplash = true
    /// Flips to true the moment the splash finishes. Passed into RootView so
    /// it knows it's safe to show the first-launch QuickGuide sheet.
    @State private var splashDidFinish = false

    // Safety: never get stuck on splash.
    private let maxSplashTime: TimeInterval = 12.0

    var body: some View {
        ZStack {
            RootView(splashIsComplete: splashDidFinish)

            if isShowingSplash {
                SplashView {
                    dismissSplash()
                }
                .transition(.opacity)
                .zIndex(1)
            }
        }
        .statusBarHidden(isShowingSplash)
        .onAppear {
            guard !reduceMotion else {
                // Reduce Motion is on: the splash is a video, so skip straight to content.
                dismissSplash()
                return
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + maxSplashTime) {
                dismissSplash()
            }
        }
    }

    private func dismissSplash() {
        guard isShowingSplash else { return }
        withAnimation(.easeOut(duration: 0.25)) {
            isShowingSplash = false
        }
        // Signal RootView that the splash is gone and it's safe to present sheets.
        splashDidFinish = true
    }
}

private struct SplashView: View {
    let onFinished: () -> Void

    @StateObject private var model = SplashVideoPlayerModel()
    @State private var didFinish = false

    // If the file isn't present, show fallback briefly.
    private let fallbackDuration: TimeInterval = 1.0

    // Your chosen splash background color (matches your screenshot)
    private let bg = Color(.sRGB, red: 25.5/255, green: 23.5/255, blue: 21.5/255, opacity: 1.0)

    // Known aspect of your video (704x1280)
    private let videoAspect: CGFloat = 704.0 / 1280.0

    var body: some View {
        ZStack {
            bg.ignoresSafeArea()

            if model.hasVideo {
                GeometryReader { proxy in
                    // Make the video smaller + centered.
                    // Adjust 0.62 to taste (smaller = 0.55, bigger = 0.70).
                    let targetWidth = min(proxy.size.width, proxy.size.height) * 0.25
                    let targetHeight = targetWidth / videoAspect

                    VideoPlayerView(player: model.player)
                        .frame(width: targetWidth, height: targetHeight)
                        .clipShape(RoundedRectangle(cornerRadius: 28, style: .continuous))
                        .position(x: proxy.size.width / 2, y: proxy.size.height / 2)
                }
                .ignoresSafeArea()
            } else {
                VStack(spacing: 10) {
                    Image(systemName: "gamecontroller")
                        .font(.system(size: 44, weight: .semibold))
                    Text("MSC Remote")
                        .font(.headline)
                        .opacity(0.9)
                }
                .foregroundStyle(.white)
            }
        }
        .onAppear {
            model.configureAudioSessionForSplash()
            model.playFromBeginning()

            if !model.hasVideo {
                DispatchQueue.main.asyncAfter(deadline: .now() + fallbackDuration) {
                    finishOnce()
                }
            }
        }
        .onDisappear {
            model.pause()
        }
        .onReceive(NotificationCenter.default.publisher(for: .AVPlayerItemDidPlayToEndTime)) { notification in
            guard model.hasVideo else { return }
            guard let endedItem = notification.object as? AVPlayerItem else { return }
            guard endedItem === model.currentItem else { return }
            finishOnce()
        }
    }

    private func finishOnce() {
        guard !didFinish else { return }
        didFinish = true
        onFinished()
    }
}

private final class SplashVideoPlayerModel: ObservableObject {
    let player: AVPlayer
    let hasVideo: Bool
    let currentItem: AVPlayerItem?

    init() {
        if let url = Self.findSplashVideoURL() {
            let item = AVPlayerItem(url: url)
            self.currentItem = item
            self.player = AVPlayer(playerItem: item)
            self.hasVideo = true
        } else {
            self.currentItem = nil
            self.player = AVPlayer()
            self.hasVideo = false
        }
    }

    func playFromBeginning() {
        if hasVideo {
            player.seek(to: .zero)
        }
        player.play()
    }

    func pause() {
        player.pause()
    }

    func configureAudioSessionForSplash() {
        // Polite audio: doesn't stomp other audio.
        let session = AVAudioSession.sharedInstance()
        do {
            try session.setCategory(.ambient, mode: .default, options: [.mixWithOthers])
            try session.setActive(true)
        } catch {
            // If this fails, video still plays (system may mute).
        }
    }

    private static func findSplashVideoURL() -> URL? {
        let baseName = "splash_intro"
        for ext in ["mp4", "mov", "m4v"] {
            if let url = Bundle.main.url(forResource: baseName, withExtension: ext) {
                return url
            }
        }
        return nil
    }
}

/// Player VC with clear background so letterboxing uses our SwiftUI background color (not black).
private struct VideoPlayerView: UIViewControllerRepresentable {
    let player: AVPlayer

    func makeUIViewController(context: Context) -> AVPlayerViewController {
        let vc = AVPlayerViewController()
        vc.player = player
        vc.showsPlaybackControls = false

        // No cropping.
        vc.videoGravity = .resizeAspect

        // Critical: prevent black bars coming from the VC itself.
        vc.view.backgroundColor = .clear
        vc.view.isOpaque = false
        vc.contentOverlayView?.backgroundColor = .clear

        return vc
    }

    func updateUIViewController(_ uiViewController: AVPlayerViewController, context: Context) {
        if uiViewController.player !== player {
            uiViewController.player = player
        }
    }
}
