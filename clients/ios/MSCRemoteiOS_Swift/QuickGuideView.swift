import SwiftUI

// MARK: - QuickGuideView
// Modelled after the macOS Welcome Guide: topic list on the left (sidebar),
// content on the right. On compact width (iPhone) this becomes a flat
// navigation stack — tap a topic to drill in, Back button to return.
//
// Access: tapping the help icon on the Dashboard (existing behaviour),
//         AND automatically on first ever launch (via @AppStorage flag).

struct QuickGuideView: View {
    @Environment(\.dismiss) private var dismiss

    /// Nil means "showing the topic list". A non-nil value means a topic is selected.
    @State private var selectedTopic: GuideTopic? = nil

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: 0) {
                    // ── Hero header (mirrors the macOS guide's Minecraft banner area) ──
                    GuideHeroHeader()

                    // ── Topic list ──
                    ScrollView(showsIndicators: false) {
                        VStack(spacing: 0) {
                            ForEach(GuideTopic.allCases) { topic in
                                NavigationLink(value: topic) {
                                    GuideTopicRow(topic: topic)
                                }
                                .buttonStyle(.plain)

                                if topic != GuideTopic.allCases.last {
                                    Divider()
                                        .background(MSCRemoteStyle.borderSubtle)
                                        .padding(.leading, 56)
                                }
                            }
                        }
                        .mscCard(padding: 0)
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        .padding(.top, MSCRemoteStyle.spaceLG)
                        .padding(.bottom, MSCRemoteStyle.spaceLG)
                    }

                    // ── Footer — pinned above home indicator ──
                    Text("TempleTech · MSC REMOTE")
                        .font(.system(size: 10, weight: .regular, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.vertical, MSCRemoteStyle.spaceMD)
                        .padding(.bottom, MSCRemoteStyle.spaceSM)
                }
            }
            .navigationTitle("")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .navigationDestination(for: GuideTopic.self) { topic in
                GuideTopicDetailView(topic: topic)
            }
        }
        .preferredColorScheme(.dark)
    }
}

// MARK: - Hero Header

private struct GuideHeroHeader: View {
    var body: some View {
        ZStack(alignment: .bottomLeading) {
            // Gradient background — tinted by accent colour
            LinearGradient(
                colors: [
                    MSCRemoteStyle.accent.opacity(0.25),
                    MSCRemoteStyle.accent.opacity(0.08),
                    MSCRemoteStyle.bgBase
                ],
                startPoint: .top,
                endPoint: .bottom
            )

            VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
                HStack(spacing: MSCRemoteStyle.spaceMD) {
                    Image(systemName: "gamecontroller.fill")
                        .font(.system(size: 28, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.accent)

                    VStack(alignment: .leading, spacing: 2) {
                        Text("MSC Remote")
                            .font(.system(.title2, design: .rounded).weight(.bold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)

                        Text("Companion app for Minecraft Server Controller")
                            .font(.system(size: 13))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                    }
                }

                Text("Your complete guide to setting up and using MSC Remote on iPhone.")
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .padding(.top, 2)
            }
            .padding(MSCRemoteStyle.spaceLG)
        }
        .frame(maxWidth: .infinity)
        .frame(height: 130)
    }
}

// MARK: - Topic List Row

private struct GuideTopicRow: View {
    let topic: GuideTopic

    var body: some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            ZStack {
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(topic.iconColor.opacity(0.18))
                    .frame(width: 32, height: 32)
                Image(systemName: topic.icon)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(topic.iconColor)
            }

            VStack(alignment: .leading, spacing: 2) {
                Text(topic.title)
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                Text(topic.subtitle)
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
            }

            Spacer()

            Image(systemName: "chevron.right")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
        .padding(.horizontal, MSCRemoteStyle.spaceLG)
        .padding(.vertical, MSCRemoteStyle.spaceMD)
    }
}

// MARK: - Topic Model

enum GuideTopic: String, CaseIterable, Identifiable, Hashable {
    case overview
    case tailscale
    case pairing
    case dashboard
    case console
    case troubleshooting

    var id: String { rawValue }

    var title: String {
        switch self {
        case .overview:        return "What is MSC Remote?"
        case .tailscale:       return "Tailscale Setup"
        case .pairing:         return "Pairing with Your Mac"
        case .dashboard:       return "Dashboard Basics"
        case .console:         return "Console & Commands"
        case .troubleshooting: return "Troubleshooting"
        }
    }

    var subtitle: String {
        switch self {
        case .overview:        return "What this app does and who it's for"
        case .tailscale:       return "Remote access from anywhere, step by step"
        case .pairing:         return "Connect your phone to the macOS app"
        case .dashboard:       return "Server status, controls, and performance"
        case .console:         return "Live logs, commands, and player list"
        case .troubleshooting: return "Buttons greyed out? Can't connect? Start here."
        }
    }

    var icon: String {
        switch self {
        case .overview:        return "house.fill"
        case .tailscale:       return "network"
        case .pairing:         return "qrcode.viewfinder"
        case .dashboard:       return "gauge.with.dots.needle.50percent"
        case .console:         return "terminal"
        case .troubleshooting: return "wrench.and.screwdriver.fill"
        }
    }

    var iconColor: Color {
        switch self {
        case .overview:        return Color(hex: "#3EB489")
        case .tailscale:       return Color(hex: "#7B68EE")
        case .pairing:         return Color(hex: "#3EB489")
        case .dashboard:       return Color(hex: "#E8A838")
        case .console:         return Color(hex: "#64B5F6")
        case .troubleshooting: return Color(hex: "#E05C5C")
        }
    }
}

// MARK: - Topic Detail View

struct GuideTopicDetailView: View {
    let topic: GuideTopic

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()
            ScrollView(showsIndicators: false) {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceXL) {
                    // Topic hero
                    HStack(spacing: MSCRemoteStyle.spaceMD) {
                        ZStack {
                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                .fill(topic.iconColor.opacity(0.18))
                                .frame(width: 48, height: 48)
                            Image(systemName: topic.icon)
                                .font(.system(size: 22, weight: .semibold))
                                .foregroundStyle(topic.iconColor)
                        }
                        VStack(alignment: .leading, spacing: 2) {
                            Text(topic.title)
                                .font(.system(.title3, design: .rounded).weight(.bold))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                            Text(topic.subtitle)
                                .font(.system(size: 13))
                                .foregroundStyle(MSCRemoteStyle.textSecondary)
                        }
                    }
                    .padding(MSCRemoteStyle.spaceLG)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(MSCRemoteStyle.bgCard)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous))

                    // Body content
                    topicBody
                }
                .padding(MSCRemoteStyle.spaceLG)
                .padding(.bottom, MSCRemoteStyle.space2XL)
            }
        }
        .navigationTitle(topic.title)
        .navigationBarTitleDisplayMode(.inline)
        .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
        .toolbarColorScheme(.dark, for: .navigationBar)
    }

    @ViewBuilder
    private var topicBody: some View {
        switch topic {
        case .overview:        OverviewContent()
        case .tailscale:       TailscaleContent()
        case .pairing:         PairingContent()
        case .dashboard:       DashboardContent()
        case .console:         ConsoleContent()
        case .troubleshooting: TroubleshootingContent()
        }
    }
}

// MARK: - Shared Guide Components

/// A coloured "Think of it like this" callout — mirrors the macOS guide style.
private struct ThinkCallout: View {
    let text: String
    var icon: String = "bubble.left.fill"
    var color: Color = Color(hex: "#7B68EE")

    var body: some View {
        HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: icon)
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(color)
                .padding(.top, 1)
            Text(text)
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(MSCRemoteStyle.spaceMD)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(color.opacity(0.10))
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
    }
}

/// A numbered step row used for walkthroughs.
private struct StepRow: View {
    let number: Int
    let title: String
    let detail: String
    var linkText: String? = nil
    var linkURL: URL? = nil

    var body: some View {
        HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
            ZStack {
                Circle()
                    .fill(MSCRemoteStyle.accent.opacity(0.18))
                    .frame(width: 28, height: 28)
                Text("\(number)")
                    .font(.system(size: 13, weight: .bold, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.accent)
            }

            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                Text(detail)
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
                if let linkText, let linkURL {
                    Link(linkText, destination: linkURL)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
    }
}

/// A bullet-point info row (no number).
private struct InfoRow: View {
    let icon: String
    let text: LocalizedStringKey
    var iconColor: Color = MSCRemoteStyle.accent

    var body: some View {
        HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: icon)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(iconColor)
                .frame(width: 20)
                .padding(.top, 1)
            Text(text)
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

/// A card wrapper with an optional label header — used to group related info rows.
private struct GuideCard<Content: View>: View {
    var label: String? = nil
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            if let label {
                Text(label.uppercased())
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .kerning(1.2)
            }
            content
        }
        .padding(MSCRemoteStyle.spaceLG)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(MSCRemoteStyle.bgCard)
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
                .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1)
        )
    }
}

// MARK: - Topic Content: Overview

private struct OverviewContent: View {
    var body: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceLG) {
            Text("MSC Remote is the iPhone companion to **Minecraft Server Controller** — the macOS app that manages your Minecraft server. Instead of walking over to your Mac (or SSHing in), you can check on your server, start or stop it, run commands, and watch the live console from your phone.")
                .font(.system(size: 15))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .fixedSize(horizontal: false, vertical: true)

            ThinkCallout(
                text: "Think of it like a remote control. Minecraft Server Controller is the machine sitting at home. MSC Remote is the TV remote in your hand.",
                icon: "gamecontroller.fill",
                color: MSCRemoteStyle.accent
            )

            GuideCard(label: "In this app") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "arrow.right", text: "Start and stop your Paper server with a single tap")
                    InfoRow(icon: "arrow.right", text: "Watch the live console — see exactly what your server is doing")
                    InfoRow(icon: "arrow.right", text: "Send commands without typing in a terminal")
                    InfoRow(icon: "arrow.right", text: "Monitor CPU, RAM, and TPS performance metrics")
                    InfoRow(icon: "arrow.right", text: "See which players are online and manage them")
                }
            }

            GuideCard(label: "What you need") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "checkmark.circle.fill", text: "A Mac running **Minecraft Server Controller** with Remote API enabled")
                    InfoRow(icon: "checkmark.circle.fill", text: "Your iPhone on the same Wi-Fi network — **or** Tailscale for remote access")
                    InfoRow(icon: "checkmark.circle.fill", text: "A pairing token from the macOS app")
                }
            }
        }
    }
}

// MARK: - Topic Content: Tailscale

private struct TailscaleContent: View {
    var body: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceLG) {

            Text("By default, MSC Remote only works when your iPhone is on the same Wi-Fi network as your Mac. **Tailscale** fixes that — it creates a secure, private tunnel between your devices so you can control your server from anywhere: your car, a friend's house, school, anywhere with internet.")
                .font(.system(size: 15))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .fixedSize(horizontal: false, vertical: true)

            ThinkCallout(
                text: "Tailscale is like giving your Mac a permanent phone number that only your approved devices can call — no matter where any of you are in the world.",
                icon: "network",
                color: Color(hex: "#7B68EE")
            )

            GuideCard(label: "Step-by-step: Set up Tailscale") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceXL) {
                    StepRow(
                        number: 1,
                        title: "Create a free Tailscale account",
                        detail: "Go to tailscale.com and sign up. It's free for personal use with up to 3 users.",
                        linkText: "→ tailscale.com/download",
                        linkURL: URL(string: "https://tailscale.com/download")
                    )

                    StepRow(
                        number: 2,
                        title: "Install Tailscale on your Mac",
                        detail: "Download the Tailscale macOS app from the Mac App Store or the website. Open it and sign in with the same account you just created."
                    )

                    StepRow(
                        number: 3,
                        title: "Connect your Mac to Tailscale",
                        detail: "Click the Tailscale icon in your Mac's menu bar and press Connect. Your Mac now has a Tailscale IP address — it starts with 100. (for example, 100.64.0.1). Write it down or copy it."
                    )

                    StepRow(
                        number: 4,
                        title: "Install Tailscale on your iPhone",
                        detail: "Search for \"Tailscale\" in the App Store and install it. Open it, sign in with the same account, and tap Connect.",
                        linkText: "→ Tailscale on the App Store",
                        linkURL: URL(string: "https://apps.apple.com/app/tailscale/id1470499037")
                    )

                    StepRow(
                        number: 5,
                        title: "Verify both devices are connected",
                        detail: "In the Tailscale app on your iPhone, you should see your Mac listed as a connected device with a green dot. If you see it, you're done with Tailscale setup."
                    )

                    StepRow(
                        number: 6,
                        title: "Use your Tailscale IP when pairing MSC Remote",
                        detail: "When setting the Base URL in MSC Remote, use your Mac's Tailscale IP instead of your local Wi-Fi IP. For example: http://100.64.0.1:48400. The port (48400 by default) stays the same."
                    )
                }
            }

            GuideCard(label: "Why use Tailscale instead of a local IP?") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "checkmark.seal.fill", text: "Works from **anywhere** with an internet connection, not just home Wi-Fi")
                    InfoRow(icon: "checkmark.seal.fill", text: "**Secure by default** — no open ports, no port forwarding, no firewall changes needed on your router")
                    InfoRow(icon: "checkmark.seal.fill", text: "Your Tailscale IP **never changes**, so pairing stays valid even when your home IP changes")
                    InfoRow(icon: "checkmark.seal.fill", text: "Free for personal use — no subscription needed")
                }
            }

            GuideCard(label: "More information") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "book.fill", text: "The Tailscale docs are excellent for beginners — they have guides for every platform.", iconColor: Color(hex: "#7B68EE"))
                    Link("→ Read the Tailscale documentation", destination: URL(string: "https://tailscale.com/kb/1017/install")!)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(Color(hex: "#7B68EE"))
                }
            }
        }
    }
}

// MARK: - Topic Content: Pairing

private struct PairingContent: View {
    var body: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceLG) {

            Text("Pairing links MSC Remote on your iPhone to the Minecraft Server Controller app on your Mac. Once paired, every button tap sends a request to your Mac, which runs the actual server.")
                .font(.system(size: 15))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .fixedSize(horizontal: false, vertical: true)

            ThinkCallout(
                text: "The pairing token is like a secret password. Your iPhone presents it with every request so the Mac knows the command is coming from you, not someone else on the network.",
                icon: "lock.shield.fill",
                color: MSCRemoteStyle.accent
            )

            // Method A: QR
            GuideCard(label: "Method A — Scan QR (recommended)") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceXL) {
                    StepRow(
                        number: 1,
                        title: "Open Minecraft Server Controller on your Mac",
                        detail: "Make sure it's running and the Remote API is enabled. You'll find the Remote API toggle in the macOS app's Settings or sidebar."
                    )
                    StepRow(
                        number: 2,
                        title: "Find the QR code",
                        detail: "In the macOS app, look for the \"Remote Access\" or \"MSC Remote\" section. There should be a QR code displayed with your connection details embedded in it."
                    )
                    StepRow(
                        number: 3,
                        title: "Open MSC Remote → Settings on your iPhone",
                        detail: "Tap the Settings tab at the bottom of the screen."
                    )
                    StepRow(
                        number: 4,
                        title: "Tap \"Scan QR Code\"",
                        detail: "Point your iPhone camera at the QR code on your Mac's screen. The Base URL and Token fields fill in automatically."
                    )
                    StepRow(
                        number: 5,
                        title: "Go back to Dashboard and tap Refresh",
                        detail: "If the server status shows (green or red), pairing worked. You're connected."
                    )
                }
            }

            // Method B: Manual
            GuideCard(label: "Method B — Manual entry") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceXL) {
                    StepRow(
                        number: 1,
                        title: "Find your Mac's IP and port",
                        detail: "In Minecraft Server Controller on your Mac, the Remote API section shows your URL — for example: http://10.0.0.142:48400. If using Tailscale, use your Tailscale IP instead (starts with 100.)."
                    )
                    StepRow(
                        number: 2,
                        title: "Copy the API token",
                        detail: "Also in the macOS Remote API section — there's a token (a long string of letters and numbers). Copy it exactly."
                    )
                    StepRow(
                        number: 3,
                        title: "Open MSC Remote → Settings",
                        detail: "Paste the URL into the Base URL field and the token into the Token field."
                    )
                    StepRow(
                        number: 4,
                        title: "Tap Save, then go to Dashboard and Refresh",
                        detail: "Server status appearing means you're connected."
                    )
                }
            }

            GuideCard(label: "Good to know") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "lock.fill", text: "Your token is stored in the **iOS Keychain** — the same secure storage used by banking apps. It is not visible in plain text anywhere.")
                    InfoRow(icon: "wifi", text: "On the same Wi-Fi: use your Mac's local IP (System Settings → Wi-Fi → Details).")
                    InfoRow(icon: "network", text: "Away from home: use your Mac's **Tailscale IP** (see the Tailscale Setup guide).")
                }
            }
        }
    }
}

// MARK: - Topic Content: Dashboard

private struct DashboardContent: View {
    var body: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceLG) {

            Text("The Dashboard is your home screen. It shows the current state of your server and gives you controls to start, stop, and manage it.")
                .font(.system(size: 15))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .fixedSize(horizontal: false, vertical: true)

            GuideCard(label: "Status card") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "circle.fill", text: "**Green dot** — server is running. **Red dot** — server is stopped or unreachable.", iconColor: MSCRemoteStyle.success)
                    InfoRow(icon: "server.rack", text: "**Active Server** — if you manage multiple servers with Minecraft Server Controller, this lets you switch which one the remote is targeting.")
                    InfoRow(icon: "arrow.clockwise", text: "**Refresh** — manually fetch the latest status. The app also polls automatically every 6 seconds when paired.")
                }
            }

            GuideCard(label: "Controls") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "play.fill", text: "**Start / Stop** — sends a start or stop command to the macOS app, which in turn controls the Java server process.", iconColor: MSCRemoteStyle.accent)
                    InfoRow(icon: "exclamationmark.triangle.fill", text: "Commands that could affect players (like Stop) require a confirmation tap to prevent accidents.", iconColor: MSCRemoteStyle.warning)
                }
            }

            GuideCard(label: "Performance metrics") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "cpu", text: "**CPU %** and **RAM** show current server resource usage.")
                    InfoRow(icon: "gauge", text: "**TPS** (Ticks Per Second) — a healthy Minecraft server runs at 20 TPS. Below 15 usually means lag.")
                    InfoRow(icon: "person.2.fill", text: "**Players** — how many are connected right now.")
                    InfoRow(icon: "chart.line.uptrend.xyaxis", text: "The chart shows historical data from the current session — useful for spotting lag spikes.")
                }
            }
        }
    }
}

// MARK: - Topic Content: Console & Commands

private struct ConsoleContent: View {
    var body: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceLG) {

            GuideCard(label: "Console tab") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "terminal", text: "**Console Tail** — fetches a snapshot of the most recent server log lines. Good for a quick check.")
                    InfoRow(icon: "dot.radiowaves.left.and.right", text: "**Live Stream** — opens a continuous connection and shows log output in real time as it happens. Uses more battery.")
                    InfoRow(icon: "magnifyingglass", text: "You can search and filter console output to find specific events or errors.")
                }
            }

            GuideCard(label: "Commands tab") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "chevron.right.2", text: "Send any Minecraft server command — /op, /kick, /say, /weather, /time, and so on.")
                    InfoRow(icon: "list.bullet", text: "Frequently used commands appear as quick-tap buttons so you don't have to type them each time.")
                    InfoRow(icon: "exclamationmark.circle", text: "Commands that could disrupt gameplay (like stopping the server or kicking a player) show a confirmation prompt before sending.")
                }
            }

            GuideCard(label: "Players tab") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "person.2", text: "Shows all currently connected players with their avatar and username.")
                    InfoRow(icon: "hand.raised.fill", text: "Tap a player to see quick actions — op, deop, kick, or ban.", iconColor: MSCRemoteStyle.warning)
                }
            }
        }
    }
}

// MARK: - Topic Content: Troubleshooting

private struct TroubleshootingContent: View {
    var body: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceLG) {

            GuideCard(label: "Buttons are greyed out") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "exclamationmark.circle.fill", text: "This almost always means MSC Remote isn't paired yet — the Base URL or Token field is empty.", iconColor: MSCRemoteStyle.danger)
                    InfoRow(icon: "arrow.right", text: "Go to **Settings** and check that both the Base URL and Token are filled in. Scan the QR code from the macOS app to fill them automatically.")
                }
            }

            GuideCard(label: "Can't connect / status won't load") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "wifi.slash", text: "**On Wi-Fi?** Make sure your iPhone and Mac are on the same network. The Base URL should be your Mac's local IP, not localhost or 127.0.0.1.")
                    InfoRow(icon: "network.slash", text: "**Not on Wi-Fi?** You need Tailscale set up first. See the Tailscale Setup guide.")
                    InfoRow(icon: "server.rack", text: "**Is Minecraft Server Controller running?** The macOS app needs to be open for the Remote API to work. Check that Remote API is enabled in the macOS app settings.")
                    InfoRow(icon: "lock.open", text: "**Is the port blocked?** The macOS app uses port 48400 by default. Make sure no firewall or security software is blocking it.")
                }
            }

            GuideCard(label: "Performance shows 'endpoint not available'") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "arrow.up.circle", text: "Your macOS app needs to be updated to a version that supports the /performance endpoint. Update Minecraft Server Controller on your Mac.", iconColor: MSCRemoteStyle.warning)
                }
            }

            GuideCard(label: "Token rejected / 403 error") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "key.slash", text: "The token stored in MSC Remote doesn't match the one the macOS app is using. Re-pair by scanning the QR code from Minecraft Server Controller.", iconColor: MSCRemoteStyle.danger)
                }
            }

            GuideCard(label: "Still stuck?") {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                    InfoRow(icon: "arrow.clockwise", text: "Try quitting and relaunching both apps, then tap Refresh on the Dashboard.")
                    InfoRow(icon: "questionmark.circle.fill", text: "Check the macOS app's built-in guide — it has more detail on Remote API configuration and network setup.", iconColor: Color(hex: "#7B68EE"))
                }
            }
        }
    }
}

// MARK: - Preview

#Preview {
    QuickGuideView()
}

