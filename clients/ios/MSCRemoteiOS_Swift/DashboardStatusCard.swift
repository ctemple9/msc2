import SwiftUI

struct DashboardStatusCard: View {
    let isRunning: Bool
    let isPaired: Bool
    let activeServerNameText: String
    let serverStartedAt: Date?
    let now: Date
    let refreshAction: () -> Void
    var connectivity: ConnectivityResponseDTO? = nil
    var connectivityAction: (() -> Void)? = nil

    private var uptimeString: String? {
        guard isRunning, let start = serverStartedAt else { return nil }
        let secs = Int(now.timeIntervalSince(start))
        guard secs >= 0 else { return nil }
        let h = secs / 3600
        let m = (secs % 3600) / 60
        let s = secs % 60
        if h > 0 { return String(format: "%dh %02dm %02ds", h, m, s) }
        if m > 0 { return String(format: "%dm %02ds", m, s) }
        return String(format: "%ds", s)
    }

    private var statusAccessibilityValue: String {
        guard isPaired else { return "Not paired" }
        var parts = [activeServerNameText, isRunning ? "Running" : "Stopped"]
        if let uptime = uptimeString { parts.append("uptime \(uptime)") }
        return parts.joined(separator: ", ")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Server Status")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(alignment: .center, spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 6) {
                    HStack(spacing: 8) {
                        MSCStatusDot(isActive: isRunning, size: 10)
                        Text(isRunning ? "RUNNING" : "STOPPED")
                            .font(.system(size: 11, weight: .bold, design: .monospaced))
                            .foregroundStyle(isRunning ? MSCRemoteStyle.success : MSCRemoteStyle.danger)
                            .kerning(0.8)
                    }
                    Text(activeServerNameText)
                        .font(.system(.title3, design: .rounded).weight(.semibold))
                        .foregroundStyle(isPaired ? MSCRemoteStyle.textPrimary : MSCRemoteStyle.textSecondary)
                        .lineLimit(1)

                    if let uptime = uptimeString {
                        HStack(spacing: 5) {
                            Image(systemName: "clock")
                                .font(.system(size: 10))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                                .accessibilityHidden(true)
                            Text(uptime)
                                .font(.system(size: 11, weight: .medium, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                        }
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel("Server status")
                .accessibilityValue(statusAccessibilityValue)
                Spacer()
                Button(action: refreshAction) {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 15, weight: .semibold))
                        .foregroundStyle(isPaired ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                        .frame(width: 36, height: 36)
                        .background(isPaired ? MSCRemoteStyle.accentDim : MSCRemoteStyle.bgElevated)
                        .clipShape(Circle())
                }
                .disabled(!isPaired)
                .accessibilityLabel("Refresh status")
            }

            if !isPaired {
                HStack(spacing: 6) {
                    Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 11))
                    Text("Not paired — open Settings to connect.")
                        .font(.system(size: 12))
                }
                .foregroundStyle(MSCRemoteStyle.warning)
                .padding(.top, MSCRemoteStyle.spaceMD)
                .frame(maxWidth: .infinity, alignment: .leading)
            } else if connectivity != nil {
                Divider()
                    .background(MSCRemoteStyle.borderSubtle)
                    .padding(.vertical, MSCRemoteStyle.spaceSM)
                if let connectivityAction {
                    Button(action: connectivityAction) {
                        HStack(spacing: MSCRemoteStyle.spaceSM) {
                            ConnectivityBadge(connectivity: connectivity, showDetail: false)
                            Spacer(minLength: 0)
                            Image(systemName: "chevron.right")
                                .font(.system(size: 10, weight: .semibold))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Connectivity")
                    .accessibilityHint("Opens Settings")
                } else {
                    ConnectivityBadge(connectivity: connectivity, showDetail: false)
                }
            }

            Spacer(minLength: 0)
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .mscCard()
    }
}
