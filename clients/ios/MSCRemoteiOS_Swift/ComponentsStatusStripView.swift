import SwiftUI

/// Compact status strip shown on the Dashboard.
/// Tapping it navigates to the Health tab.
struct ComponentsStatusStripView: View {
    let componentsStatus: ComponentsStatusDTO?
    let broadcastStatus: BroadcastStatusDTO?
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 0) {
                HStack {
                    MSCSectionHeader(title: "Server Health")
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .accessibilityHidden(true)
                }
                .padding(.bottom, MSCRemoteStyle.spaceMD)

                if let status = componentsStatus, !status.components.isEmpty {
                    HStack(spacing: MSCRemoteStyle.spaceMD) {
                        ForEach(status.components) { component in
                            componentDot(component)
                        }
                        if let broadcast = broadcastStatus {
                            broadcastDot(broadcast)
                        }
                        Spacer()
                    }
                } else {
                    HStack(spacing: 6) {
                        ProgressView()
                            .scaleEffect(0.7)
                        Text("Checking components…")
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
            }
        }
        .buttonStyle(.plain)
        .mscCard()
    }

    @ViewBuilder
    private func componentDot(_ component: ComponentStatusDTO) -> some View {
        HStack(spacing: 5) {
            Circle()
                .fill(dotColor(for: component))
                .frame(width: 7, height: 7)
                .shadow(color: dotColor(for: component).opacity(0.5), radius: 3)
            Text(component.name)
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(component.name), \(componentStatusWord(for: component))")
    }

    @ViewBuilder
    private func broadcastDot(_ broadcast: BroadcastStatusDTO) -> some View {
        let isRunning = broadcast.xboxBroadcastRunning || broadcast.bedrockBroadcastRunning
        HStack(spacing: 5) {
            Circle()
                .fill(isRunning ? MSCRemoteStyle.success : MSCRemoteStyle.danger)
                .frame(width: 7, height: 7)
                .shadow(color: (isRunning ? MSCRemoteStyle.success : MSCRemoteStyle.danger).opacity(0.5), radius: 3)
            Text("Broadcast")
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Broadcast, \(isRunning ? "running" : "stopped")")
    }

    private func dotColor(for component: ComponentStatusDTO) -> Color {
        guard component.isInstalled else { return MSCRemoteStyle.textTertiary }
        return component.isUpToDate ? MSCRemoteStyle.success : Color.orange
    }

    private func componentStatusWord(for component: ComponentStatusDTO) -> String {
        guard component.isInstalled else { return "not installed" }
        if !component.isUpdatable { return "installed" }
        return component.isUpToDate ? "up to date" : "update available"
    }
}
