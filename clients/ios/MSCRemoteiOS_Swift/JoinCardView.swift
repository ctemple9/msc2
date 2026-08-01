import SwiftUI

// MARK: - Preset colors (mirrors macOS JoinCardView presets exactly)

private struct CardColorPreset: Identifiable {
    let id = UUID()
    let color: Color
    let hex: String
}

private let cardColorPresets: [CardColorPreset] = [
    CardColorPreset(color: Color(red: 0.18, green: 0.40, blue: 0.20), hex: "#2E6633"),  // Forest green (default)
    CardColorPreset(color: Color(red: 0.13, green: 0.30, blue: 0.58), hex: "#214D94"),  // Deep blue
    CardColorPreset(color: Color(red: 0.42, green: 0.18, blue: 0.62), hex: "#6B2D9E"),  // Purple
    CardColorPreset(color: Color(red: 0.58, green: 0.14, blue: 0.16), hex: "#942428"),  // Deep red
    CardColorPreset(color: Color(red: 0.62, green: 0.36, blue: 0.10), hex: "#9E5C1A"),  // Amber
    CardColorPreset(color: Color(red: 0.16, green: 0.28, blue: 0.38), hex: "#294761"),  // Slate
]

private let defaultCardColorHex = "#2E6633"

// MARK: - Color helpers

private extension Color {
    init?(hexRGB: String) {
        var hex = hexRGB.trimmingCharacters(in: .whitespacesAndNewlines)
        if hex.hasPrefix("#") { hex = String(hex.dropFirst()) }
        guard hex.count == 6, let value = UInt64(hex, radix: 16) else { return nil }
        let r = Double((value >> 16) & 0xFF) / 255
        let g = Double((value >> 8)  & 0xFF) / 255
        let b = Double( value        & 0xFF) / 255
        self.init(red: r, green: g, blue: b)
    }

    func hexRGBString() -> String? {
        guard let components = UIColor(self).cgColor.components, components.count >= 3 else { return nil }
        let r = Int(components[0] * 255)
        let g = Int(components[1] * 255)
        let b = Int(components[2] * 255)
        return String(format: "#%02X%02X%02X", r, g, b)
    }
}

private func colorsMatch(_ a: Color, _ b: Color) -> Bool {
    guard let ha = a.hexRGBString(), let hb = b.hexRGBString() else { return false }
    return ha.lowercased() == hb.lowercased()
}

// MARK: - JoinCardView (inline card shown on Dashboard)

struct JoinCardView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var isFlipped: Bool = false
    @State private var rotation: Double = 0
    @State private var cardColor: Color = Color(hexRGB: defaultCardColorHex) ?? .green
    @State private var showShareSheet: Bool = false
    @State private var shareImage: UIImage? = nil
    @State private var exportToast: String? = nil
    @State private var showCustomize: Bool = false

    private var activeServer: ServerDTO? {
        guard let activeId = vm.status?.activeServerId else {
            return vm.servers.first
        }
        return vm.servers.first(where: { $0.id == activeId }) ?? vm.servers.first
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Join Card")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            // ── Live reachability ─────────────────────────────────────────
            ConnectivityBadge(connectivity: vm.connectivityResponse, showDetail: true)
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            // ── Flip card ─────────────────────────────────────────────────
            ZStack {
                JoinCardFrontFace(server: activeServer, cardColor: cardColor, gamertag: settings.xboxGamertag)
                    .opacity(isFlipped ? 0 : 1)
                    .rotation3DEffect(.degrees(rotation), axis: (x: 0, y: 1, z: 0))

                JoinCardBackFace(server: activeServer, cardColor: cardColor, settings: settings)
                    .opacity(isFlipped ? 1 : 0)
                    .rotation3DEffect(.degrees(rotation - 180), axis: (x: 0, y: 1, z: 0))
            }
            .frame(height: 180)
            .onTapGesture { flipCard() }
            .padding(.bottom, MSCRemoteStyle.spaceSM)

            // Flip hint
            Text(isFlipped
                 ? "Back \u{2014} connection details \u{2014} tap to flip"
                 : "Tap card to flip  \u{2022}  Front is what gets shared")
                .font(.system(size: 10, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            // ── Customize toggle ──────────────────────────────────────────
            Button {
                withAnimation(.easeInOut(duration: 0.2)) { showCustomize.toggle() }
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "slider.horizontal.3")
                        .font(.system(size: 11, weight: .semibold))
                    Text("Customize")
                        .font(.system(size: 12, weight: .semibold))
                    Spacer()
                    Image(systemName: showCustomize ? "chevron.up" : "chevron.down")
                        .font(.system(size: 10, weight: .semibold))
                }
                .foregroundStyle(MSCRemoteStyle.textSecondary)
            }
            .buttonStyle(.plain)
            .padding(.bottom, showCustomize ? MSCRemoteStyle.spaceSM : MSCRemoteStyle.spaceMD)

            if showCustomize {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {

                    // ── Color swatches + custom picker ────────────────────
                    VStack(alignment: .leading, spacing: 6) {
                        Text("COLOR")
                            .font(.system(size: 9, weight: .bold, design: .monospaced))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .kerning(0.8)

                        HStack(spacing: MSCRemoteStyle.spaceSM) {
                            ForEach(cardColorPresets) { preset in
                                colorSwatch(preset.color, isSelected: colorsMatch(cardColor, preset.color)) {
                                    cardColor = preset.color
                                    settings.joinCardColorHex = preset.hex
                                    settings.saveJoinCardPreferences()
                                }
                            }

                            Rectangle()
                                .fill(MSCRemoteStyle.borderMid)
                                .frame(width: 1, height: 22)
                                .padding(.horizontal, 2)

                            ColorPicker("", selection: $cardColor, supportsOpacity: false)
                                .labelsHidden()
                                .frame(width: 28, height: 28)
                                .clipShape(Circle())
                                .overlay(Circle().strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1.5))
                                .onChange(of: cardColor) { _, newColor in
                                    if let hex = newColor.hexRGBString() {
                                        settings.joinCardColorHex = hex
                                        settings.saveJoinCardPreferences()
                                    }
                                }
                        }
                    }

                    // ── Xbox Gamertag input ───────────────────────────────
                    VStack(alignment: .leading, spacing: 4) {
                        Text("XBOX GAMERTAG")
                            .font(.system(size: 9, weight: .bold, design: .monospaced))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .kerning(0.8)

                        TextField("Enter your Xbox gamertag\u{2026}", text: $settings.xboxGamertag)
                            .font(.system(size: 13, weight: .medium))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 8)
                            .background(MSCRemoteStyle.bgElevated)
                            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                            .overlay(
                                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                                    .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                            )
                            .autocorrectionDisabled()
                            .textInputAutocapitalization(.never)
                            .onChange(of: settings.xboxGamertag) { _, _ in
                                settings.saveJoinCardPreferences()
                            }
                    }
                }
                .padding(.bottom, MSCRemoteStyle.spaceMD)
                .transition(.opacity.combined(with: .move(edge: .top)))
            }

            // ── Action buttons ────────────────────────────────────────────
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                Button {
                    flipCard()
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: "arrow.left.arrow.right")
                            .font(.system(size: 12))
                        Text(isFlipped ? "Front" : "Back")
                            .font(.system(size: 12, weight: .semibold))
                    }
                    .frame(maxWidth: .infinity)
                    .frame(height: 38)
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .background(MSCRemoteStyle.bgElevated)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                            .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                    )
                }
                .buttonStyle(.plain)

                Button {
                    renderAndShare()
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: "square.and.arrow.up")
                            .font(.system(size: 12))
                        Text("Share")
                            .font(.system(size: 12, weight: .semibold))
                    }
                    .frame(maxWidth: .infinity)
                    .frame(height: 38)
                    .foregroundStyle(MSCRemoteStyle.bgBase)
                    .background(MSCRemoteStyle.accent)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                }
                .buttonStyle(.plain)
            }

            // Export toast
            if let toast = exportToast {
                Text(toast)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.success)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.top, MSCRemoteStyle.spaceSM)
                    .transition(.opacity)
            }
        }
        .mscCard()
        .onAppear { loadSavedColor() }
        .sheet(isPresented: $showShareSheet) {
            if let image = shareImage {
                ShareSheet(items: [image])
            }
        }
        .animation(.easeInOut(duration: 0.2), value: exportToast)
    }

    // MARK: - Helpers

    private func flipCard() {
        withAnimation(.easeInOut(duration: 0.45)) {
            rotation += 180
            isFlipped.toggle()
        }
    }

    @ViewBuilder
    private func colorSwatch(_ color: Color, isSelected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            ZStack {
                Circle().fill(color).frame(width: 24, height: 24)
                if isSelected {
                    Image(systemName: "checkmark")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundStyle(.white)
                }
            }
        }
        .buttonStyle(.plain)
        .overlay(
            Circle()
                .stroke(isSelected ? color : MSCRemoteStyle.borderMid, lineWidth: isSelected ? 2.5 : 1)
                .frame(width: 28, height: 28)
        )
    }

    private func loadSavedColor() {
        let hex = settings.joinCardColorHex
        cardColor = Color(hexRGB: hex) ?? (Color(hexRGB: defaultCardColorHex) ?? .green)
    }

    private func renderAndShare() {
        let cardWidth = UIScreen.main.bounds.width - 64
        let cardHeight: CGFloat = 180

        let frontRenderer = ImageRenderer(content:
            JoinCardFrontFace(server: activeServer, cardColor: cardColor, gamertag: settings.xboxGamertag)
                .frame(width: cardWidth, height: cardHeight)
        )
        frontRenderer.scale = 3.0

        let backRenderer = ImageRenderer(content:
            JoinCardBackFace(server: activeServer, cardColor: cardColor, settings: settings)
                .frame(width: cardWidth, height: cardHeight)
        )
        backRenderer.scale = 3.0

        guard let frontImage = frontRenderer.uiImage,
              let backImage  = backRenderer.uiImage else {
            showToast("Export failed")
            return
        }

        let gap: CGFloat = 16
        let combinedSize = CGSize(
            width: max(frontImage.size.width, backImage.size.width),
            height: frontImage.size.height + gap + backImage.size.height
        )

        let format = UIGraphicsImageRendererFormat()
        format.scale = 1
        let combined = UIGraphicsImageRenderer(size: combinedSize, format: format).image { _ in
            frontImage.draw(at: .zero)
            backImage.draw(at: CGPoint(x: 0, y: frontImage.size.height + gap))
        }

        shareImage = combined
        showShareSheet = true
    }

    private func showToast(_ text: String) {
        withAnimation { exportToast = text }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { exportToast = nil }
        }
    }
}

// MARK: - Card Front

struct JoinCardFrontFace: View {
    let server: ServerDTO?
    let cardColor: Color
    let gamertag: String

    private var serverName: String   { server?.name ?? "My Server" }
    private var serverType: ServerType { server?.resolvedServerType ?? .java }
    private var isBedrock: Bool      { serverType == .bedrock }

    private var resolvedGamertag: String? {
        let trimmed = gamertag.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [cardColor, cardColor.opacity(0.75)],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            // Subtle grid texture — identical to macOS version
            Canvas { ctx, size in
                let spacing: CGFloat = 24
                var x: CGFloat = 0
                while x < size.width {
                    var y: CGFloat = 0
                    while y < size.height {
                        ctx.fill(Path(CGRect(x: x, y: y, width: spacing - 1, height: spacing - 1)),
                                 with: .color(.white.opacity(0.03)))
                        y += spacing
                    }
                    x += spacing
                }
            }

            VStack(alignment: .leading, spacing: 0) {

                // ── Top row: cube icon + branding ─────────────────
                HStack {
                    Image(systemName: "cube.fill")
                        .font(.system(size: 24, weight: .bold))
                        .foregroundStyle(.white.opacity(0.85))
                    Spacer()
                    Text("Hosted with MSC")
                        .font(.system(size: 8, weight: .medium))
                        .foregroundStyle(.white.opacity(0.30))
                }

                Spacer()

                // ── Server name + subtitle ────────────────────────
                Text(serverName)
                    .font(.system(size: 20, weight: .bold, design: .rounded))
                    .foregroundStyle(.white)
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)

                Text(isBedrock ? "Bedrock Server" : "Java Server")
                    .font(.system(size: 9, weight: .bold))
                    .tracking(1.5)
                    .foregroundStyle(.white.opacity(0.45))
                    .padding(.top, 2)

                Divider()
                    .background(Color.white.opacity(0.20))
                    .padding(.vertical, 8)

                // ── Gamertag box ──────────────────────────────────
                HStack(spacing: 8) {
                    Image(systemName: "person.badge.plus")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(.white.opacity(0.75))
                    VStack(alignment: .leading, spacing: 2) {
                        Text(resolvedGamertag.map { "Add \u{201C}\($0)\u{201D} as a friend" } ?? "Set your gamertag in Settings")
                            .font(.system(size: 15, weight: .bold))
                            .foregroundStyle(.white.opacity(0.95))
                            .lineLimit(1)
                            .minimumScaleFactor(0.75)
                        Text("The server will appear in your Worlds tab")
                            .font(.system(size: 10, weight: .medium))
                            .foregroundStyle(.white.opacity(0.55))
                    }
                    Spacer()
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 7)
                .background(
                    RoundedRectangle(cornerRadius: 6, style: .continuous)
                        .fill(Color.white.opacity(0.06))
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 6, style: .continuous)
                        .stroke(Color.white.opacity(0.10), lineWidth: 0.75)
                )

                Spacer()

                // ── Bottom instruction ────────────────────────────
                HStack(spacing: 4) {
                    Image(systemName: "gamecontroller")
                        .font(.system(size: 9))
                        .foregroundStyle(.white.opacity(0.50))
                    Text("Console: Friends tab \u{2192} Add Friend \u{2192} server appears in Worlds")
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(.white.opacity(0.60))
                        .lineLimit(1)
                        .minimumScaleFactor(0.7)
                }
            }
            .padding(16)
        }
        .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .stroke(Color.white.opacity(0.12), lineWidth: 1)
        )
        .shadow(color: .black.opacity(0.25), radius: 12, y: 6)
    }
}

// MARK: - Card Back

struct JoinCardBackFace: View {
    let server: ServerDTO?
    let cardColor: Color
    let settings: SettingsStore

    private var serverName: String { server?.name ?? "My Server" }
    private var serverType: ServerType { server?.resolvedServerType ?? .java }

    private var hostAddress: String {
        if let addr = server?.hostAddress, !addr.isEmpty { return addr }
        if let url = settings.resolvedBaseURL(), let host = url.host { return host }
        return "Not configured"
    }

    private var portDisplay: String { String(server?.resolvedGamePort ?? (serverType == .bedrock ? 19132 : 25565)) }

    private var instructionText: String {
        serverType == .bedrock
            ? "Minecraft \u{2192} Play \u{2192} Friends \u{2192} Add Server"
            : "Multiplayer \u{2192} Add Server \u{2192} Paste address"
    }

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [cardColor, cardColor.opacity(0.75)],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )

            // Subtle grid texture — matches front
            Canvas { ctx, size in
                let spacing: CGFloat = 24
                var x: CGFloat = 0
                while x < size.width {
                    var y: CGFloat = 0
                    while y < size.height {
                        ctx.fill(Path(CGRect(x: x, y: y, width: spacing - 1, height: spacing - 1)),
                                 with: .color(.white.opacity(0.03)))
                        y += spacing
                    }
                    x += spacing
                }
            }

            VStack(alignment: .leading, spacing: 0) {

                // ── Top row: app icon + branding ──────────────────
                HStack {
                    if let uiIcon = UIImage(named: "AppIcon") {
                        Image(uiImage: uiIcon)
                            .resizable()
                            .frame(width: 32, height: 32)
                            .clipShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
                    } else {
                        Image(systemName: "cube.fill")
                            .font(.system(size: 26, weight: .bold))
                            .foregroundStyle(.white.opacity(0.85))
                            .frame(width: 32, height: 32)
                    }
                    Spacer()
                    Text("Hosted with MSC")
                        .font(.system(size: 8, weight: .medium))
                        .foregroundStyle(.white.opacity(0.30))
                }

                Spacer()

                // ── Server name + subtitle ────────────────────────
                Text(serverName)
                    .font(.system(size: 20, weight: .bold, design: .rounded))
                    .foregroundStyle(.white)
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)

                Text("DIRECT CONNECT")
                    .font(.system(size: 9, weight: .bold))
                    .tracking(1.5)
                    .foregroundStyle(.white.opacity(0.45))
                    .padding(.top, 2)

                Divider()
                    .background(Color.white.opacity(0.20))
                    .padding(.vertical, 8)

                // ── Combined address:port field ───────────────────
                HStack(spacing: 8) {
                    Image(systemName: "network")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(.white.opacity(0.75))
                    VStack(alignment: .leading, spacing: 2) {
                        Text("\(hostAddress):\(portDisplay)")
                            .font(.system(size: 19, weight: .bold, design: .monospaced))
                            .foregroundStyle(.white.opacity(0.95))
                            .lineLimit(1)
                            .minimumScaleFactor(0.65)
                        Text("Enter in the Add Server screen")
                            .font(.system(size: 10, weight: .medium))
                            .foregroundStyle(.white.opacity(0.55))
                    }
                    Spacer()
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 7)
                .background(
                    RoundedRectangle(cornerRadius: 6, style: .continuous)
                        .fill(Color.white.opacity(0.06))
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 6, style: .continuous)
                        .stroke(Color.white.opacity(0.10), lineWidth: 0.75)
                )

                Spacer()

                // ── Bottom instruction ────────────────────────────
                HStack(spacing: 4) {
                    Image(systemName: "desktopcomputer")
                        .font(.system(size: 9))
                        .foregroundStyle(.white.opacity(0.50))
                    Text(instructionText)
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(.white.opacity(0.60))
                        .lineLimit(1)
                        .minimumScaleFactor(0.7)
                }
            }
            .padding(16)
        }
        .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .stroke(Color.white.opacity(0.12), lineWidth: 1)
        )
        .shadow(color: .black.opacity(0.25), radius: 12, y: 6)
    }

    @ViewBuilder
    private func connectionField(label: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(label)
                .font(.system(size: 8, weight: .bold))
                .tracking(1.0)
                .foregroundStyle(.white.opacity(0.35))
            Text(value)
                .font(.system(size: 14, weight: .bold, design: .monospaced))
                .foregroundStyle(.white.opacity(0.92))
                .lineLimit(1)
                .minimumScaleFactor(0.7)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(
            RoundedRectangle(cornerRadius: 6, style: .continuous)
                .fill(Color.white.opacity(0.06))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 6, style: .continuous)
                .stroke(Color.white.opacity(0.10), lineWidth: 0.75)
        )
    }
}

// MARK: - Connectivity badge (P11)

/// Live "can people connect right now?" indicator, driven by GET /connectivity.
/// Used on the Join card (with detail) and the Dashboard status card (compact).
struct ConnectivityBadge: View {
    let connectivity: ConnectivityResponseDTO?
    var showDetail: Bool = false

    private var severity: String { connectivity?.severity ?? "gray" }

    private var color: Color {
        switch severity {
        case "green":  return MSCRemoteStyle.success
        case "yellow": return MSCRemoteStyle.warning
        case "red":    return MSCRemoteStyle.danger
        default:       return MSCRemoteStyle.textTertiary
        }
    }

    private var label: String {
        guard let c = connectivity else { return "Checking reachability…" }
        switch c.status {
        case "reachable":   return "Reachable"
        case "unreachable": return "Not reachable"
        case "offline":     return "Server off"
        case "starting":    return "Starting up…"
        default:            return "Unverified"
        }
    }

    var body: some View {
        HStack(alignment: showDetail ? .top : .center, spacing: 8) {
            Circle()
                .fill(color)
                .frame(width: 8, height: 8)
                .padding(.top, showDetail ? 4 : 0)
            VStack(alignment: .leading, spacing: 2) {
                Text(label)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(color)
                if showDetail, let detail = connectivity?.detail {
                    Text(detail)
                        .font(.system(size: 10.5))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: 0)
            if let players = connectivity?.playersOnline, let maxP = connectivity?.playersMax,
               connectivity?.status == "reachable" {
                Text("\(players)/\(maxP)")
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
        }
    }
}

// MARK: - UIKit share sheet wrapper

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]

    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }

    func updateUIViewController(_ uvc: UIActivityViewController, context: Context) {}
}
