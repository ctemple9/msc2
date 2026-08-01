import SwiftUI

struct DashboardPerformanceCard: View {
    enum MetricHealth {
        case good, warning, bad, neutral

        var color: Color {
            switch self {
            case .good:    return MSCRemoteStyle.accent
            case .warning: return MSCRemoteStyle.warning
            case .bad:     return MSCRemoteStyle.danger
            case .neutral: return MSCRemoteStyle.textTertiary
            }
        }
    }

    let activeServerType: ServerType
    let performanceLatest: DashboardViewModel.PerformancePoint?
    let performanceHistory: [DashboardViewModel.PerformancePoint]
    let performanceErrorMessage: String?
    let errorMessage: String?
    let isRunning: Bool
    let now: Date
    let metricColumnCount: Int
    let perfAgeLabel: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(
                title: "Performance",
                trailing: perfAgeLabel
            )
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            if let msg = performanceErrorMessage, !msg.isEmpty {
                HStack(spacing: 6) {
                    Image(systemName: "exclamationmark.circle").font(.system(size: 11))
                    Text(msg).font(.system(size: 11)).lineLimit(2)
                }
                .foregroundStyle(MSCRemoteStyle.warning)
                .padding(.bottom, MSCRemoteStyle.spaceMD)
            }

            let isPerfStale = performanceLatest.map { now.timeIntervalSince($0.timestamp) > 12 } ?? true
            let hasError = !(errorMessage ?? "").isEmpty || !(performanceErrorMessage ?? "").isEmpty
            let latest: DashboardViewModel.PerformancePoint? =
                (!hasError && isRunning && !isPerfStale) ? performanceLatest : nil

            LazyVGrid(
                columns: Array(repeating: GridItem(.flexible()), count: metricColumnCount),
                spacing: MSCRemoteStyle.spaceMD
            ) {
                if activeServerType == .java {
                    metricTile(title: "TPS 1m", value: formatTPS(latest?.tps1m),
                               icon: "speedometer", health: tpsHealth(latest?.tps1m))
                }
                metricTile(title: "Players", value: formatInt(latest?.playersOnline),
                           icon: "person.2.fill", health: .neutral)
                metricTile(title: "CPU", value: formatPercent(latest?.cpuPercent),
                           icon: "cpu", health: cpuHealth(latest?.cpuPercent))
                metricTile(title: "RAM", value: formatRAM(usedMB: latest?.ramUsedMB, maxMB: latest?.ramMaxMB),
                           icon: "memorychip", health: ramHealth(latest?.ramUsedMB, latest?.ramMaxMB))
                metricTile(title: "World", value: formatMB(latest?.worldSizeMB),
                           icon: "globe", health: .neutral)
                metricTile(title: "Samples", value: latest == nil ? "—" : "\(Set(performanceHistory.map { $0.timestamp }).count)/60",
                           icon: "chart.xyaxis.line", health: .neutral)
            }

            if latest == nil && !hasError {
                Text("Waiting for server data…")
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .padding(.top, MSCRemoteStyle.spaceSM)
            }
        }
        .mscCard()
    }

    private func metricTile(title: String, value: String, icon: String, health: MetricHealth) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 5) {
                Image(systemName: icon)
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(health.color)
                    .accessibilityHidden(true)
                Text(title.uppercased())
                    .font(.system(size: 9, weight: .semibold, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .kerning(0.5)
            }
            Text(value)
                .font(.system(.title2, design: .rounded).weight(.semibold))
                .foregroundStyle(health == .neutral ? MSCRemoteStyle.textPrimary : health.color)
                .lineLimit(1)
                .minimumScaleFactor(0.7)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(MSCRemoteStyle.spaceMD)
        .background(MSCRemoteStyle.bgElevated)
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                .strokeBorder(
                    health == .neutral ? MSCRemoteStyle.borderSubtle : health.color.opacity(0.25),
                    lineWidth: 1
                )
        )
        .accessibilityElement(children: .combine)
        .accessibilityLabel(title)
        .accessibilityValue(value == "—" ? "no data" : value)
    }

    private func formatTPS(_ v: Double?) -> String {
        guard let v else { return "—" }
        return String(format: "%.2f", v)
    }

    private func formatInt(_ v: Int?) -> String {
        guard let v else { return "—" }
        return "\(v)"
    }

    private func formatPercent(_ v: Double?) -> String {
        guard let v else { return "—" }
        return String(format: "%.0f%%", v)
    }

    private func formatMB(_ v: Double?) -> String {
        guard let v else { return "—" }
        if v >= 1024 { return String(format: "%.2f GB", v / 1024.0) }
        return String(format: "%.0f MB", v)
    }

    private func formatRAM(usedMB: Double?, maxMB: Double?) -> String {
        guard let usedMB else { return "—" }
        if let maxMB {
            if usedMB >= 1024 || maxMB >= 1024 {
                return String(format: "%.1f/%.1f G", usedMB / 1024.0, maxMB / 1024.0)
            }
            return String(format: "%.0f/%.0f M", usedMB, maxMB)
        }
        return formatMB(usedMB)
    }

    private func tpsHealth(_ v: Double?) -> MetricHealth {
        guard let v else { return .neutral }
        if v >= 18 { return .good }
        if v >= 12 { return .warning }
        return .bad
    }

    private func cpuHealth(_ v: Double?) -> MetricHealth {
        guard let v else { return .neutral }
        if v < 50 { return .good }
        if v < 80 { return .warning }
        return .bad
    }

    private func ramHealth(_ used: Double?, _ max: Double?) -> MetricHealth {
        guard let used, let max, max > 0 else { return .neutral }
        let pct = used / max
        if pct < 0.60 { return .good }
        if pct < 0.85 { return .warning }
        return .bad
    }
}
