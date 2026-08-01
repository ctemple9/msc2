import SwiftUI

struct DashboardServerCard: View {
    let servers: [ServerDTO]
    let activeServerId: String
    let activeServerNameText: String
    let activeServerType: ServerType
    let isPaired: Bool
    let isRunning: Bool
    @Binding var selectedServerId: String
    let manageAction: () -> Void
    let startAction: () -> Void
    let stopAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Active Server")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if servers.isEmpty {
                Text(isPaired ? "Loading servers…" : "Pair to load servers.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.bottom, MSCRemoteStyle.spaceMD)
            } else {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    Menu {
                        ForEach(servers, id: \.id) { server in
                            Button {
                                selectedServerId = server.id
                            } label: {
                                if server.id == activeServerId {
                                    Label(server.name, systemImage: "checkmark")
                                } else {
                                    Label(server.name, systemImage: server.resolvedServerType.iconName)
                                }
                            }
                        }
                    } label: {
                        HStack {
                            VStack(alignment: .leading, spacing: 3) {
                                Text(activeServerNameText)
                                    .font(.system(size: 15, weight: .medium, design: .rounded))
                                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                                HStack(spacing: 5) {
                                    Image(systemName: activeServerType.iconName)
                                        .font(.system(size: 10))
                                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                                    Text(activeServerType.displayName)
                                        .font(.system(size: 11))
                                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                                    Text("·")
                                        .font(.system(size: 11))
                                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                                    Text("Tap to switch server")
                                        .font(.system(size: 11))
                                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                                }
                            }
                            Spacer()
                            Image(systemName: "chevron.up.chevron.down")
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                        }
                        .padding(MSCRemoteStyle.spaceMD)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        .overlay(
                            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                                .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                        )
                    }
                    .disabled(!isPaired)
                    .accessibilityLabel("Active server: \(activeServerNameText)")
                    .accessibilityHint("Double tap to choose a different server")

                    Button(action: manageAction) {
                        Image(systemName: "gearshape")
                            .font(.system(size: 15, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.accent)
                            .frame(width: 44, height: 44)
                            .background(MSCRemoteStyle.bgElevated)
                            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                            .overlay(
                                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                                    .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                            )
                        }
                    .buttonStyle(.plain)
                    .disabled(!isPaired)
                    .accessibilityLabel("Manage Servers")
                }
                .padding(.bottom, MSCRemoteStyle.spaceMD)
            }

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                MSCActionButton(title: "Start", icon: "play.fill", style: .primary,
                                isEnabled: isPaired && !isRunning,
                                action: startAction)
                MSCActionButton(title: "Stop", icon: "stop.fill", style: .danger,
                                isEnabled: isPaired && isRunning,
                                action: stopAction)
            }
        }
        .mscCard()
    }
}
