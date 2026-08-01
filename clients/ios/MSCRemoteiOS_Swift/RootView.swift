import SwiftUI
import UIKit

// MARK: - Navigation Item
//
// Represents a top-level destination in both the sidebar (iPad) and
// tab bar (iPhone). A single source of truth means the two nav chrome
// styles are always in sync — add a destination once, it appears in both.

enum NavDestination: String, CaseIterable, Hashable {
    case dashboard
    case console
    case players
    case server
    case settings

    var title: String {
        switch self {
        case .dashboard: return "Home"
        case .console:   return "Console"
        case .players:   return "Players"
        case .server:    return "Server"
        case .settings:  return "Settings"
        }
    }

    var icon: String {
        switch self {
        case .dashboard: return "gauge.with.dots.needle.50percent"
        case .console:   return "terminal"
        case .players:   return "person.2"
        case .server:    return "square.stack.3d.up"
        case .settings:  return "gearshape"
        }
    }
}

// MARK: - RootView

struct RootView: View {
    @EnvironmentObject private var settings: SettingsStore
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

    @StateObject private var dashboardVM = DashboardViewModel()

    @State private var statusPollTask: Task<Void, Never>? = nil
    @State private var backgroundTaskID: UIBackgroundTaskIdentifier = .invalid
    @State private var selectedDestination: NavDestination = .dashboard
    @State private var activeAuthPrompt: BroadcastAuthPromptDTO? = nil
    @State private var ipadSidebarVisible: Bool = true

    @AppStorage("mscremote.hasSeenQuickGuide") private var hasSeenQuickGuide = false
    @State private var showFirstLaunchGuide = false

    /// Passed in from SplashGateView. Flips to true the instant the splash
    /// overlay finishes animating out. The QuickGuide sheet is only shown
    /// after this becomes true, preventing it from appearing on top of the
    /// splash video on first launch.
    var splashIsComplete: Bool

    private var isPaired: Bool {
        settings.resolvedBaseURL() != nil && settings.resolvedToken() != nil
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            Group {
                if horizontalSizeClass == .regular {
                    ipadLayout
                } else {
                    iphoneLayout
                }
            }

            // Auth banner sits just above the tab bar / bottom safe area
            if let prompt = dashboardVM.pendingAuthPrompt, prompt.isPresent {
                XboxAuthBannerView(prompt: prompt) {
                    activeAuthPrompt = prompt
                }
                .padding(.bottom, horizontalSizeClass == .regular ? 16 : 54)
                .zIndex(999)
                .animation(.spring(response: 0.4, dampingFraction: 0.8), value: dashboardVM.pendingAuthPrompt != nil)
            }
        }
        .sheet(item: $activeAuthPrompt, onDismiss: {
            guard let baseURL = settings.resolvedBaseURL(),
                  let token = settings.resolvedToken() else { return }
            Task { await dashboardVM.dismissAuthPrompt(baseURL: baseURL, token: token) }
        }) { prompt in
            XboxAuthSheet(prompt: prompt)
                .environmentObject(settings)
                .environmentObject(dashboardVM)
        }
        .sheet(isPresented: $showFirstLaunchGuide, onDismiss: {
            hasSeenQuickGuide = true
        }) {
            QuickGuideView()
        }
        // Wait for the splash to finish before showing the first-launch guide.
        // Previously this used .onAppear with a 0.5s delay, which fired while
        // the splash was still playing — causing the sheet to cover the video.
        .onChange(of: splashIsComplete) { _, isComplete in
            if isComplete && !hasSeenQuickGuide {
                showFirstLaunchGuide = true
            }
        }
        .task(id: isPaired) {
            stopStatusPolling()
            guard isPaired else { return }
            startStatusPolling()
        }
        .onChange(of: scenePhase) { _, newPhase in
            if newPhase == .background {
                beginBackgroundWindow()
            } else {
                endBackgroundWindow()
            }
        }
        .onDisappear {
            stopStatusPolling()
        }
    }

    // MARK: - iPad Layout (NavigationSplitView)
    //
    // Two-column split: a persistent sidebar on the left, the selected
    // view in the detail column on the right. The sidebar is always
    // visible in landscape; in portrait iPadOS can collapse it, but the
    // user can still drag it in from the edge — this is standard iPadOS
    // behaviour and requires no extra code.
    //
    // We don't use .navigationSplitViewStyle(.balanced) because we want
    // the sidebar to be narrow and the detail to use all remaining space.
    // .prominent keeps the detail column dominant, which is correct for
    // a server control app where the content is the focus.

    private var ipadLayout: some View {
        HStack(spacing: 0) {
            if ipadSidebarVisible {
                ipadSidebar
                    .transition(.move(edge: .leading))
            }
            ZStack(alignment: .topLeading) {
                ipadDetailView
                    .frame(maxWidth: .infinity, maxHeight: .infinity)

                Button {
                    withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                        ipadSidebarVisible.toggle()
                    }
                } label: {
                    Image(systemName: "sidebar.left")
                        .font(.system(size: 18))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                        .padding(10)
                }
                .padding(.top, 28)
                .padding(.leading, 16)
            }
        }
        .tint(MSCRemoteStyle.accent)
        .ignoresSafeArea()
    }

    // MARK: - Sidebar

    private var ipadSidebar: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            VStack(spacing: 0) {
                // App identity header — gives the sidebar a product feel
                // instead of just being a list of tabs. Mirrors what Apple
                // does in apps like Xcode and Console.
                sidebarHeader

                // Navigation list
                // Note: List(_:id:selection:) is macOS-only.
                // On iOS we use a plain List with ForEach + Button rows,
                // tracking selection manually via @State.
                ScrollView {
                    VStack(spacing: 2) {
                        ForEach(NavDestination.allCases, id: \.self) { destination in
                            Button {
                                selectedDestination = destination
                            } label: {
                                sidebarRow(destination)
                                    .background(
                                        RoundedRectangle(cornerRadius: 8, style: .continuous)
                                            .fill(selectedDestination == destination
                                                  ? MSCRemoteStyle.accent.opacity(0.12)
                                                  : Color.clear)
                                    )
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .padding(.horizontal, 12)
                }

                Spacer()

                sidebarFooter
            }
        }
        .frame(width: 240)
        .background(MSCRemoteStyle.bgBase)
    }

    private var sidebarHeader: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 10) {
                AppIconMark(size: 32)
                VStack(alignment: .leading, spacing: 2) {
                    Text("MSC Remote")
                        .font(.system(size: 15, weight: .bold, design: .rounded))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("TempleTech")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 20)
        .padding(.top, 20)
        .padding(.bottom, 16)
    }

    private func sidebarRow(_ destination: NavDestination) -> some View {
        let isSelected = selectedDestination == destination

        return Label(destination.title, systemImage: destination.icon)
            .font(.system(size: 14, weight: isSelected ? .semibold : .regular))
            .foregroundStyle(isSelected ? MSCRemoteStyle.accent : MSCRemoteStyle.textSecondary)
            .padding(.vertical, 6)
            .padding(.horizontal, 4)
            // Extend tap target to full row width
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
    }

    private var sidebarFooter: some View {
        VStack(alignment: .leading, spacing: 6) {
            Divider()
                .background(MSCRemoteStyle.borderSubtle)
                .padding(.horizontal, 20)

            HStack(spacing: 6) {
                Circle()
                    .fill(isPaired ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                    .frame(width: 6, height: 6)
                    .shadow(color: isPaired ? MSCRemoteStyle.success.opacity(0.6) : .clear, radius: 3)
                Text(isPaired ? "Paired" : "Not paired")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(isPaired ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
            }
            .padding(.horizontal, 20)
            .padding(.bottom, 20)
        }
    }

    // MARK: - Detail view

    @ViewBuilder
    private var ipadDetailView: some View {
        switch selectedDestination {
        case .dashboard:
            DashboardView(navigateToConnectivity: { selectedDestination = .server })
                .environmentObject(dashboardVM)
        case .console:
            ConsoleView()
                .environmentObject(dashboardVM)
        case .players:
            PlayersView()
                .environmentObject(dashboardVM)
        case .server:
            ServerView()
                .environmentObject(dashboardVM)
        case .settings:
            SettingsView()
                .environmentObject(dashboardVM)
        }
    }

    // MARK: - iPhone Layout (TabView)
    //
    // Identical to the original TabView — untouched so there are zero
    // regressions on iPhone.

    private var iphoneLayout: some View {
        TabView(selection: $selectedDestination) {
            DashboardView(navigateToConnectivity: { selectedDestination = .server })
                .environmentObject(dashboardVM)
                .tabItem { Label("Home", systemImage: "gauge.with.dots.needle.50percent") }
                .tag(NavDestination.dashboard)

            ConsoleView()
                .environmentObject(dashboardVM)
                .tabItem { Label("Console", systemImage: "terminal") }
                .tag(NavDestination.console)

            PlayersView()
                .environmentObject(dashboardVM)
                .tabItem { Label("Players", systemImage: "person.2") }
                .tag(NavDestination.players)

            ServerView()
                .environmentObject(dashboardVM)
                .tabItem { Label("Server", systemImage: "square.stack.3d.up") }
                .tag(NavDestination.server)
                .badge(dashboardVM.addonsResponse?.updateCount ?? 0)

            SettingsView()
                .environmentObject(dashboardVM)
                .tabItem { Label("Settings", systemImage: "gearshape") }
                .tag(NavDestination.settings)
        }
        .tint(MSCRemoteStyle.accent)
        .onAppear { applyTabBarTint() }
        .onChange(of: settings.accentColorHex) { _, _ in applyTabBarTint() }
    }

    private func applyTabBarTint() {
        UITabBar.appearance().tintColor = UIColor(MSCRemoteStyle.accent)
    }

    // MARK: - Status polling (unchanged from original)

    private func startStatusPolling() {
        statusPollTask = Task {
            while !Task.isCancelled {
                let pairing: (URL, String)? = await MainActor.run {
                    guard let baseURL = settings.resolvedBaseURL(),
                          let token = settings.resolvedToken() else {
                        return nil
                    }
                    return (baseURL, token)
                }

                guard let (baseURL, token) = pairing else { return }

                await dashboardVM.pollStatusAndPlayers(baseURL: baseURL, token: token)

                try? await Task.sleep(nanoseconds: 6_000_000_000)
            }
        }
    }

    private func stopStatusPolling() {
        statusPollTask?.cancel()
        statusPollTask = nil
        endBackgroundWindow()
    }

    private func beginBackgroundWindow() {
        guard backgroundTaskID == .invalid else { return }
        backgroundTaskID = UIApplication.shared.beginBackgroundTask(withName: "MSCRemoteStatusPoll") {
            endBackgroundWindow()
        }
    }

    private func endBackgroundWindow() {
        guard backgroundTaskID != .invalid else { return }
        UIApplication.shared.endBackgroundTask(backgroundTaskID)
        backgroundTaskID = .invalid
    }
}
