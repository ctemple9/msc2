import SwiftUI

// MARK: - App-wide sticky banner shown when Xbox Broadcast needs a device code

struct XboxAuthBannerView: View {
    let prompt: BroadcastAuthPromptDTO
    let onTap: () -> Void

    var body: some View {
        Button { onTap() } label: {
            HStack(spacing: 10) {
                Image(systemName: "person.badge.key.fill")
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(Color.orange)

                VStack(alignment: .leading, spacing: 1) {
                    Text("Xbox sign-in required")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("Tap to view your device code")
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                }

                Spacer()

                Image(systemName: "chevron.right")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .fill(MSCRemoteStyle.bgElevated)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10, style: .continuous)
                            .strokeBorder(Color.orange.opacity(0.5), lineWidth: 1)
                    )
            )
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 12)
        .padding(.bottom, 4)
        .transition(.move(edge: .bottom).combined(with: .opacity))
    }
}

// MARK: - Sheet shown when banner is tapped

struct XboxAuthSheet: View {
    let prompt: BroadcastAuthPromptDTO
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss
    @State private var copied = false

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    Image(systemName: "person.badge.key.fill")
                        .font(.system(size: 48))
                        .foregroundStyle(Color.orange)
                        .padding(.top, MSCRemoteStyle.spaceLG)

                    VStack(spacing: MSCRemoteStyle.spaceSM) {
                        Text("Xbox Sign-In")
                            .font(.system(size: 22, weight: .bold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)

                        Text("Your Mac needs you to sign in to Xbox Live.\nOpen the link below and enter the code.")
                            .font(.system(size: 14))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                            .multilineTextAlignment(.center)
                    }

                    // Code display
                    if let code = prompt.code {
                        VStack(spacing: 6) {
                            Text("DEVICE CODE")
                                .font(.system(size: 10, weight: .bold, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                                .kerning(1.2)

                            Text(code)
                                .font(.system(size: 36, weight: .bold, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .padding(.horizontal, 24)
                                .padding(.vertical, 14)
                                .background(
                                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                                        .fill(MSCRemoteStyle.bgElevated)
                                        .overlay(
                                            RoundedRectangle(cornerRadius: 12, style: .continuous)
                                                .strokeBorder(Color.orange.opacity(0.4), lineWidth: 1.5)
                                        )
                                )

                            Button {
                                UIPasteboard.general.string = code
                                copied = true
                            } label: {
                                Label(copied ? "Copied!" : "Tap to copy", systemImage: copied ? "checkmark" : "doc.on.doc")
                                    .font(.system(size: 12))
                                    .foregroundStyle(copied ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                            }
                            .buttonStyle(.plain)
                        }
                    }

                    // Open link button
                    if let urlString = prompt.linkURL, let url = URL(string: urlString) {
                        Link(destination: url) {
                            HStack(spacing: 8) {
                                Image(systemName: "safari")
                                    .font(.system(size: 14, weight: .semibold))
                                Text("Open microsoft.com/link")
                                    .font(.system(size: 15, weight: .semibold))
                            }
                            .frame(maxWidth: .infinity)
                            .frame(height: 48)
                            .foregroundStyle(.white)
                            .background(Color.orange)
                            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        }
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    }

                    // Done button — synchronous dismiss, cleanup happens in onDismiss
                    Button { dismiss() } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "checkmark.circle")
                                .font(.system(size: 14, weight: .semibold))
                            Text("Done — I've entered the code")
                                .font(.system(size: 15, weight: .semibold))
                        }
                        .frame(maxWidth: .infinity)
                        .frame(height: 48)
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        .overlay(
                            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                                .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)

                    Spacer()
                }
                .padding(.top, MSCRemoteStyle.spaceMD)
            }
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Close") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                }
            }
        }
    }
}
