import SwiftUI

struct TailscaleHelpSheet: View {
    @Binding var isPresented: Bool

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceLG) {
                        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                            MSCSectionHeader(title: "Overview")
                            Text("Same Wi-Fi works without Tailscale. Not on the same Wi-Fi? You'll need a VPN like Tailscale.")
                                .font(.system(size: 13))
                                .foregroundStyle(MSCRemoteStyle.textSecondary)
                        }
                        .mscCard()

                        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                            MSCSectionHeader(title: "2-Minute Setup")
                            VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                                ForEach(Array([
                                    "Install Tailscale on your Mac and iPhone.",
                                    "Sign into the same Tailscale account on both.",
                                    "Turn Tailscale ON on both devices.",
                                    "In the Mac app, enable \"Expose Remote API on LAN/VPN\".",
                                    "In this app, Scan QR or paste the pairing link.",
                                    "Tap Test /status."
                                ].enumerated()), id: \.offset) { idx, step in
                                    HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
                                        Text("\(idx + 1)")
                                            .font(.system(size: 11, weight: .bold, design: .monospaced))
                                            .foregroundStyle(MSCRemoteStyle.accent)
                                            .frame(width: 20, alignment: .trailing)
                                        Text(step)
                                            .font(.system(size: 13))
                                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                                            .fixedSize(horizontal: false, vertical: true)
                                    }
                                }
                            }
                        }
                        .mscCard()

                        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                            MSCSectionHeader(title: "Tip")
                            Text("Use your Mac's Tailscale MagicDNS name as the Base URL — example: http://your-mac.tailXXXX.ts.net:48400")
                                .font(.system(size: 12, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                                .textSelection(.enabled)
                        }
                        .mscCard()
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.vertical, MSCRemoteStyle.spaceMD)
                }
            }
            .navigationTitle("Tailscale Setup")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") {
                        hapticLight()
                        isPresented = false
                    }
                    .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
    }
}
