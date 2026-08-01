import SwiftUI

struct SettingsPairingCard: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    let isPaired: Bool
    let isFirstRun: Bool
    let hasToken: Bool
    let safetyWarning: String?
    let saveConfirmed: Bool
    @Binding var showQRScanner: Bool
    @Binding var showTailscaleHelp: Bool
    let saveAction: () -> Void
    let clearTokenAction: () -> Void
    let clearBaseURLAction: () -> Void
    let pastePairingAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .center) {
                MSCSectionHeader(title: "Pairing")
                Spacer()
                pairedStatusPill
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            if isFirstRun {
                HStack(alignment: .top, spacing: MSCRemoteStyle.spaceSM) {
                    Image(systemName: "info.circle")
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.accent)
                        .padding(.top, 1)
                        .accessibilityHidden(true)
                    Text("Enter your Mac's IP address and the token shown in MSC's Preferences → Remote API to get started. Or scan the QR code for instant setup.")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .padding(MSCRemoteStyle.spaceMD)
                .background(MSCRemoteStyle.accentDim)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .strokeBorder(MSCRemoteStyle.accent.opacity(0.2), lineWidth: 1)
                )
                .padding(.bottom, MSCRemoteStyle.spaceLG)
            }

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                quickPairButton(title: "Scan QR", icon: "qrcode.viewfinder") {
                    hapticLight()
                    showQRScanner = true
                }
                quickPairButton(title: "Paste Link", icon: "doc.on.clipboard") {
                    pastePairingAction()
                }
            }
            .padding(.bottom, MSCRemoteStyle.spaceLG)

            VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
                fieldLabel("Base URL")
                TextField("http://192.168.1.50:48400", text: $settings.baseURLString)
                    .font(.system(size: 13, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .keyboardType(.URL)
                    .padding(MSCRemoteStyle.spaceMD)
                    .background(MSCRemoteStyle.bgElevated)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                            .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                    )
                    .accessibilityLabel("Base URL")

                fieldLabel("Token")
                SecureField(
                    hasToken ? "Token saved — enter new to replace" : "Bearer token",
                    text: $settings.tokenDraft
                )
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .padding(MSCRemoteStyle.spaceMD)
                .background(MSCRemoteStyle.bgElevated)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .strokeBorder(
                            hasToken ? MSCRemoteStyle.accent.opacity(0.4) : MSCRemoteStyle.borderMid,
                            lineWidth: 1
                        )
                )
                .accessibilityLabel("Token")
                .accessibilityValue(hasToken ? "saved" : "empty")

                if hasToken && settings.tokenDraft.isEmpty {
                    HStack(spacing: 5) {
                        Image(systemName: "key.fill")
                            .font(.system(size: 9, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.accent)
                            .accessibilityHidden(true)
                        Text("Token stored in Keychain  ••••••••")
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(MSCRemoteStyle.accent.opacity(0.8))
                    }
                    .padding(.top, 2)
                }
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            if let warning = safetyWarning {
                HStack(spacing: 6) {
                    Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 11))
                        .accessibilityHidden(true)
                    Text(warning).font(.system(size: 11)).textSelection(.enabled)
                }
                .foregroundStyle(MSCRemoteStyle.warning)
                .padding(.bottom, MSCRemoteStyle.spaceMD)
                .accessibilityElement(children: .combine)
            }

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Button(action: saveAction) {
                    HStack(spacing: 6) {
                        Image(systemName: saveConfirmed ? "checkmark" : "square.and.arrow.down")
                            .font(.system(size: 13, weight: .semibold))
                            .transition(.scale.combined(with: .opacity))
                        Text(saveConfirmed ? "Saved" : "Save")
                            .font(.system(size: 14, weight: .semibold))
                    }
                    .frame(maxWidth: .infinity)
                    .frame(height: 44)
                    .foregroundStyle(MSCRemoteStyle.bgBase)
                    .background(saveConfirmed ? MSCRemoteStyle.success : MSCRemoteStyle.accent)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    .animation(.easeInOut(duration: 0.15), value: saveConfirmed)
                }

                Menu {
                    Button(role: .destructive, action: clearTokenAction) {
                        Label("Clear Token", systemImage: "key.slash")
                    }
                    Button(role: .destructive, action: clearBaseURLAction) {
                        Label("Clear Base URL", systemImage: "xmark.circle")
                    }
                } label: {
                    Image(systemName: "ellipsis")
                        .font(.system(size: 15, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                        .frame(width: 44, height: 44)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        .overlay(
                            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                                .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                        )
                }
                .accessibilityLabel("More pairing options")
            }

            Button {
                hapticLight()
                showTailscaleHelp = true
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "lock.shield")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.accent)
                        .accessibilityHidden(true)
                    Text("Away from home? Use Tailscale (2-min setup)")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.accent)
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .accessibilityHidden(true)
                }
                .padding(.top, MSCRemoteStyle.spaceMD)
            }
        }
        .mscCard()
    }

    private var pairedStatusPill: some View {
        HStack(spacing: MSCRemoteStyle.spaceSM) {
            HStack(spacing: 5) {
                Circle()
                    .fill(isPaired ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                    .frame(width: 6, height: 6)
                    .shadow(color: isPaired ? MSCRemoteStyle.success.opacity(0.7) : .clear, radius: 3)
                Text(isPaired ? "Paired" : "Not paired")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(isPaired ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                    .kerning(0.3)
            }
            .padding(.horizontal, MSCRemoteStyle.spaceSM)
            .padding(.vertical, 5)
            .background(
                isPaired
                    ? MSCRemoteStyle.success.opacity(0.10)
                    : MSCRemoteStyle.bgElevated
            )
            .clipShape(Capsule())
            .overlay(
                Capsule().strokeBorder(
                    isPaired ? MSCRemoteStyle.success.opacity(0.25) : MSCRemoteStyle.borderSubtle,
                    lineWidth: 1
                )
            )
            .animation(.easeInOut(duration: 0.2), value: isPaired)

            if let role = vm.connectedRole, isPaired {
                Text(role == "guest" ? "Guest" : "Admin")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(role == "guest" ? MSCRemoteStyle.warning : MSCRemoteStyle.accent)
                    .padding(.horizontal, MSCRemoteStyle.spaceSM)
                    .padding(.vertical, 5)
                    .background((role == "guest" ? MSCRemoteStyle.warning : MSCRemoteStyle.accent).opacity(0.10))
                    .clipShape(Capsule())
                    .overlay(
                        Capsule().strokeBorder(
                            (role == "guest" ? MSCRemoteStyle.warning : MSCRemoteStyle.accent).opacity(0.25),
                            lineWidth: 1
                        )
                    )
                    .animation(.easeInOut(duration: 0.2), value: vm.connectedRole)
            }
        }
        .accessibilityElement(children: .combine)
    }

    private func quickPairButton(title: String, icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.system(size: 22, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.accent)
                    .accessibilityHidden(true)
                Text(title)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
            }
            .frame(maxWidth: .infinity)
            .frame(height: 72)
            .background(MSCRemoteStyle.accentDim)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(MSCRemoteStyle.accent.opacity(0.25), lineWidth: 1)
            )
        }
    }

    private func fieldLabel(_ text: String) -> some View {
        Text(text.uppercased())
            .font(.system(size: 9, weight: .semibold, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .kerning(1.0)
    }
}
