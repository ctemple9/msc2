import SwiftUI

/// P12: iOS playit.gg tunnel management screen.
/// Shows tunnel status, stored addresses, and start/stop controls (admin only).
/// Presented as a sheet from the Health tab's playit card.
struct PlayitView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    @State private var isActioning: Bool = false
    @State private var errorText: String? = nil
    @State private var toast: String? = nil
    @State private var isLoading: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isAdmin: Bool { vm.connectedRole == "admin" }
    private var status: PlayitStatusResponseDTO? { vm.playitStatusResponse }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                content

                if let toast {
                    VStack {
                        Spacer()
                        Text(toast)
                            .font(.system(size: 13, weight: .medium))
                            .foregroundStyle(.white)
                            .padding(.horizontal, MSCRemoteStyle.spaceLG)
                            .padding(.vertical, MSCRemoteStyle.spaceMD)
                            .frame(maxWidth: MSCRemoteStyle.contentMaxWidth - 40)
                            .background(MSCRemoteStyle.bgElevated)
                            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                            .padding(.bottom, MSCRemoteStyle.spaceLG)
                            .padding(.horizontal, MSCRemoteStyle.spaceLG)
                            .multilineTextAlignment(.center)
                    }
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                }
            }
            .navigationTitle("playit.gg Tunnel")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
                ToolbarItem(placement: .topBarTrailing) {
                    if isLoading {
                        ProgressView().scaleEffect(0.8)
                    } else {
                        Button { Task { await refresh() } } label: {
                            Image(systemName: "arrow.clockwise")
                                .foregroundStyle(MSCRemoteStyle.accent)
                        }
                    }
                }
            }
            .task { await refresh() }
        }
    }

    @ViewBuilder
    private var content: some View {
        ScrollView(showsIndicators: false) {
            VStack(spacing: MSCRemoteStyle.spaceLG) {
                if let s = status {
                    statusCard(s)
                    addressesCard(s)
                    if isAdmin { controlCard(s) }
                } else if isLoading {
                    ProgressView("Loading…")
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                        .frame(maxWidth: .infinity)
                        .padding(.top, 60)
                } else {
                    Text("Could not load playit status.")
                        .font(.system(size: 14))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .frame(maxWidth: .infinity)
                        .padding(.top, 60)
                }

                if let err = errorText {
                    errorBanner(err)
                }
            }
            .padding(.horizontal, MSCRemoteStyle.spaceLG)
            .padding(.top, MSCRemoteStyle.spaceMD)
            .padding(.bottom, MSCRemoteStyle.spaceLG)
        }
    }

    // MARK: - Status card

    private func statusCard(_ s: PlayitStatusResponseDTO) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Tunnel Status")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(alignment: .center, spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 8) {
                        MSCStatusDot(isActive: s.isRunning, size: 10)
                        Text(s.isRunning ? "RUNNING" : "STOPPED")
                            .font(.system(size: 11, weight: .bold, design: .monospaced))
                            .foregroundStyle(s.isRunning ? MSCRemoteStyle.success : MSCRemoteStyle.danger)
                            .kerning(0.8)
                    }
                    if !s.serverName.isEmpty {
                        Text(s.serverName)
                            .font(.system(.subheadline, design: .rounded).weight(.semibold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                            .lineLimit(1)
                    }
                }
                Spacer()
                Image(systemName: "antenna.radiowaves.left.and.right")
                    .font(.system(size: 22))
                    .foregroundStyle(s.isRunning ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            if !s.hasSecretKey {
                noticeRow(icon: "key.fill", color: MSCRemoteStyle.warning,
                          text: "No secret key configured. Set it up on the Mac in Edit Server → Network.")
            } else if !s.playitEnabled {
                noticeRow(icon: "exclamationmark.triangle.fill", color: MSCRemoteStyle.textTertiary,
                          text: "playit is not enabled for this server. Enable it in Edit Server → Network on the Mac.")
            }
        }
        .mscCard()
    }

    // MARK: - Addresses card

    private func addressesCard(_ s: PlayitStatusResponseDTO) -> some View {
        let hasAny = s.javaAddress != nil || s.bedrockAddress != nil || (s.voiceChatEnabled && s.voiceAddress != nil)
        return VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Tunnel Addresses")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if hasAny {
                VStack(spacing: 0) {
                    if let addr = s.javaAddress {
                        addressRow(label: "Java", address: addr)
                    }
                    if let addr = s.bedrockAddress {
                        if s.javaAddress != nil { Divider().background(MSCRemoteStyle.borderSubtle) }
                        addressRow(label: "Bedrock", address: addr)
                    }
                    if s.voiceChatEnabled, let addr = s.voiceAddress {
                        if s.javaAddress != nil || s.bedrockAddress != nil {
                            Divider().background(MSCRemoteStyle.borderSubtle)
                        }
                        addressRow(label: "Voice Chat", address: addr)
                    }
                }
            } else {
                Text(s.isRunning
                     ? "Addresses not yet resolved — the tunnel may still be connecting. Try refreshing."
                     : "Start the tunnel to see addresses.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
        }
        .mscCard()
    }

    private func addressRow(label: String, address: String) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            VStack(alignment: .leading, spacing: 2) {
                Text(label)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                Text(address)
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .textSelection(.enabled)
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
            }
            Spacer()
            Button {
                UIPasteboard.general.string = address
                showToast("Copied \(label) address")
            } label: {
                Image(systemName: "doc.on.doc")
                    .font(.system(size: 14))
                    .foregroundStyle(MSCRemoteStyle.accent)
            }
            .buttonStyle(.plain)
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM)
    }

    // MARK: - Control card (admin only)

    private func controlCard(_ s: PlayitStatusResponseDTO) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Controls")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            let canStart = s.hasSecretKey && s.playitEnabled && !s.isRunning
            let canStop  = s.isRunning

            Button {
                guard !isActioning else { return }
                if canStop {
                    Task { await performStop() }
                } else if canStart {
                    Task { await performStart() }
                }
            } label: {
                HStack(spacing: 8) {
                    if isActioning {
                        ProgressView().scaleEffect(0.8)
                    } else {
                        Image(systemName: canStop ? "stop.fill" : "play.fill")
                            .font(.system(size: 12, weight: .semibold))
                    }
                    Text(canStop ? "Stop Tunnel" : "Start Tunnel")
                        .font(.system(size: 14, weight: .semibold))
                }
                .frame(maxWidth: .infinity)
                .frame(height: 44)
                .foregroundStyle(!canStart && !canStop ? MSCRemoteStyle.textTertiary : (canStop ? .white : MSCRemoteStyle.bgBase))
                .background(!canStart && !canStop ? MSCRemoteStyle.bgElevated : (canStop ? MSCRemoteStyle.danger : MSCRemoteStyle.accent))
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            }
            .buttonStyle(.plain)
            .disabled(isActioning || (!canStart && !canStop))

            if !s.hasSecretKey || !s.playitEnabled {
                Text(s.hasSecretKey ? "Enable playit for this server on the Mac to start the tunnel." : "Configure a secret key on the Mac to use the tunnel.")
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .padding(.top, MSCRemoteStyle.spaceSM)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .multilineTextAlignment(.center)
            }
        }
        .mscCard()
    }

    // MARK: - Helpers

    private func noticeRow(icon: String, color: Color, text: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .font(.system(size: 12))
                .foregroundStyle(color)
            Text(text)
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private func errorBanner(_ message: String) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.danger)
            Text(message)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.danger)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .mscCard(padding: MSCRemoteStyle.spaceMD)
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
                .strokeBorder(MSCRemoteStyle.danger.opacity(0.3), lineWidth: 1)
        )
    }

    private func showToast(_ message: String) {
        withAnimation { toast = message }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { toast = nil }
        }
    }

    // MARK: - Actions

    private func refresh() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isLoading = true
        await vm.fetchPlayitStatus(baseURL: baseURL, token: token)
        isLoading = false
    }

    private func performStart() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isActioning = true
        errorText = nil
        let result = await vm.startPlayit(baseURL: baseURL, token: token)
        isActioning = false
        switch result {
        case "started":         showToast("Tunnel is starting…"); await refresh()
        case "already_running": showToast("Tunnel is already running."); await refresh()
        case "not_enabled":     errorText = "playit is not enabled for this server. Enable it in Edit Server → Network on the Mac."
        case "no_secret_key":   errorText = "No secret key configured. Set it up on the Mac."
        case "no_server":       errorText = "No active server found."
        case nil:               errorText = "Could not reach the Mac. Check your connection."
        default:                errorText = "Unexpected response: \(result ?? "")"
        }
    }

    private func performStop() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isActioning = true
        errorText = nil
        let result = await vm.stopPlayit(baseURL: baseURL, token: token)
        isActioning = false
        switch result {
        case "stopped":     showToast("Tunnel stopped."); await refresh()
        case "not_running": showToast("Tunnel was already stopped."); await refresh()
        case nil:           errorText = "Could not reach the Mac. Check your connection."
        default:            errorText = "Unexpected response: \(result ?? "")"
        }
    }
}
