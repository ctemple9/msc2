import SwiftUI
import Charts

struct DashboardChartsCard: View {
    let performanceHistory: [DashboardViewModel.PerformancePoint]
    @Binding var showRAMLine: Bool
    let isIPad: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .center) {
                MSCSectionHeader(title: "History")
                Spacer()
                ramTogglePill
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            if performanceHistory.count < 2 {
                collectingDataPlaceholder
            } else {
                performanceChart
            }
        }
        .mscCard()
    }

    private var ramTogglePill: some View {
        Button {
            withAnimation(.easeInOut(duration: 0.2)) {
                showRAMLine.toggle()
            }
        } label: {
            HStack(spacing: 5) {
                Circle()
                    .fill(showRAMLine ? MSCRemoteStyle.warning : MSCRemoteStyle.textTertiary)
                    .frame(width: 6, height: 6)
                Text("RAM")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(showRAMLine ? MSCRemoteStyle.warning : MSCRemoteStyle.textTertiary)
                    .kerning(0.5)
            }
            .padding(.horizontal, MSCRemoteStyle.spaceSM)
            .padding(.vertical, 5)
            .background(
                showRAMLine
                    ? MSCRemoteStyle.warning.opacity(0.12)
                    : MSCRemoteStyle.bgElevated
            )
            .clipShape(Capsule())
            .overlay(
                Capsule().strokeBorder(
                    showRAMLine ? MSCRemoteStyle.warning.opacity(0.3) : MSCRemoteStyle.borderSubtle,
                    lineWidth: 1
                )
            )
        }
        .buttonStyle(.plain)
        .accessibilityLabel(showRAMLine ? "Hide RAM overlay" : "Show RAM overlay")
    }

    private var chartAccessibilitySummary: String {
        guard let latest = performanceHistory.last?.tps1m else { return "No performance data yet" }
        var trend = "steady"
        if performanceHistory.count >= 5 {
            let earlier = performanceHistory[performanceHistory.count - 5].tps1m ?? latest
            let delta = latest - earlier
            if delta > 1 { trend = "rising" }
            else if delta < -1 { trend = "falling" }
            else { trend = "flat" }
        }
        var summary = "TPS \(String(format: "%.0f", latest)) out of 20, trending \(trend)"
        if showRAMLine, let usedMB = performanceHistory.last?.ramUsedMB, let maxMB = performanceHistory.last?.ramMaxMB, maxMB > 0 {
            let pct = Int((usedMB / maxMB) * 100)
            summary += ". RAM \(pct) percent"
        }
        return summary
    }

    private var collectingDataPlaceholder: some View {
        ZStack {
            Color(hex: "#0A0C0E")
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))

            VStack(spacing: MSCRemoteStyle.spaceSM) {
                Image(systemName: "chart.line.uptrend.xyaxis")
                    .font(.system(size: 20, weight: .light))
                    .foregroundStyle(MSCRemoteStyle.textTertiary.opacity(0.4))
                Text("Collecting data…")
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                Text("Chart appears after 2+ samples (≈10 seconds)")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary.opacity(0.5))
            }
        }
        .frame(height: isIPad ? 200 : 160)
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1)
        )
    }

    private var performanceChart: some View {
        let history = performanceHistory
        let ramMax = history.compactMap(\.ramMaxMB).last

        return VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                legendDot(color: MSCRemoteStyle.accent, label: "TPS (1m)", range: "0–20")
                if showRAMLine, let maxMB = ramMax {
                    legendDot(
                        color: MSCRemoteStyle.warning,
                        label: "RAM",
                        range: maxMB >= 1024
                            ? String(format: "0–%.0f GB", maxMB / 1024)
                            : String(format: "0–%.0f MB", maxMB)
                    )
                }
            }
            .padding(.bottom, 4)

            Chart {
                ForEach(history) { point in
                    if let tps = point.tps1m {
                        LineMark(
                            x: .value("Time", point.timestamp),
                            y: .value("TPS", tps),
                            series: .value("Metric", "TPS")
                        )
                        .foregroundStyle(MSCRemoteStyle.accent)
                        .interpolationMethod(.catmullRom)
                        .lineStyle(StrokeStyle(lineWidth: 2))

                        AreaMark(
                            x: .value("Time", point.timestamp),
                            y: .value("TPS", tps),
                            series: .value("Metric", "TPS")
                        )
                        .foregroundStyle(
                            LinearGradient(
                                colors: [
                                    MSCRemoteStyle.accent.opacity(0.20),
                                    MSCRemoteStyle.accent.opacity(0.0)
                                ],
                                startPoint: .top,
                                endPoint: .bottom
                            )
                        )
                        .interpolationMethod(.catmullRom)
                    }
                }

                if showRAMLine, let maxMB = ramMax {
                    ForEach(history) { point in
                        if let usedMB = point.ramUsedMB, maxMB > 0 {
                            let normalised = (usedMB / maxMB) * 20.0
                            LineMark(
                                x: .value("Time", point.timestamp),
                                y: .value("RAM", normalised),
                                series: .value("Metric", "RAM")
                            )
                            .foregroundStyle(MSCRemoteStyle.warning.opacity(0.85))
                            .interpolationMethod(.catmullRom)
                            .lineStyle(StrokeStyle(lineWidth: 1.5, dash: [4, 3]))

                            AreaMark(
                                x: .value("Time", point.timestamp),
                                y: .value("RAM", normalised),
                                series: .value("Metric", "RAM")
                            )
                            .foregroundStyle(
                                LinearGradient(
                                    colors: [
                                        MSCRemoteStyle.warning.opacity(0.10),
                                        MSCRemoteStyle.warning.opacity(0.0)
                                    ],
                                    startPoint: .top,
                                    endPoint: .bottom
                                )
                            )
                            .interpolationMethod(.catmullRom)
                        }
                    }
                }
            }
            .chartYScale(domain: 0...20)
            .chartYAxis {
                AxisMarks(values: [0, 10, 20]) { value in
                    AxisGridLine()
                        .foregroundStyle(MSCRemoteStyle.borderSubtle)
                    if let v = value.as(Double.self), v == 20 || v == 0 {
                        AxisValueLabel()
                            .font(.system(size: 9, design: .monospaced))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
            }
            .chartXAxis {
                AxisMarks(values: [history.first!.timestamp, history.last!.timestamp]) { _ in
                    AxisGridLine()
                        .foregroundStyle(MSCRemoteStyle.borderSubtle)
                    AxisValueLabel(format: .dateTime.hour().minute().second())
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            .chartPlotStyle { plot in
                plot.background(Color(hex: "#0A0C0E"))
            }
            .frame(height: isIPad ? 200 : 160)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1)
            )
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("Performance history chart")
            .accessibilityValue(chartAccessibilitySummary)
        }
    }

    private func legendDot(color: Color, label: String, range: String) -> some View {
        HStack(spacing: 5) {
            RoundedRectangle(cornerRadius: 1)
                .fill(color)
                .frame(width: 12, height: 2)
            Text(label)
                .font(.system(size: 9, weight: .semibold, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .kerning(0.3)
            Text("· \(range)")
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
    }
}
