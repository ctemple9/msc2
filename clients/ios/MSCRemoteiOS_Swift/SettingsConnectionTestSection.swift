import SwiftUI

struct SettingsConnectionTestSection: View {
    @EnvironmentObject private var settings: SettingsStore
    let testIsRunning: Bool
    let lastTestResult: String?
    let lastTestWasSuccess: Bool
    let testAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Connection Test")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            Button(action: testAction) {
                HStack(spacing: 8) {
                    if testIsRunning {
                        ProgressView()
                            .progressViewStyle(.circular)
                            .scaleEffect(0.75)
                            .tint(MSCRemoteStyle.bgBase)
                    } else {
                        Image(systemName: "bolt.horizontal")
                            .font(.system(size: 13, weight: .semibold))
                    }
                    Text(testIsRunning ? "Testing…" : "Test /status")
                        .font(.system(size: 14, weight: .semibold))
                }
                .frame(maxWidth: .infinity)
                .frame(height: 44)
                .foregroundStyle(testIsRunning ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.bgBase)
                .background(testIsRunning ? MSCRemoteStyle.bgElevated : MSCRemoteStyle.accent)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            }
            .disabled(testIsRunning)

            if let msg = lastTestResult {
                let resultColor = lastTestWasSuccess ? MSCRemoteStyle.accent : MSCRemoteStyle.danger
                HStack(alignment: .top, spacing: MSCRemoteStyle.spaceSM) {
                    Image(systemName: lastTestWasSuccess ? "checkmark.circle.fill" : "xmark.circle.fill")
                        .font(.system(size: 13))
                        .foregroundStyle(resultColor)
                        .padding(.top, 1)
                        .accessibilityHidden(true)

                    Text(msg)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(resultColor)
                        .textSelection(.enabled)
                        .fixedSize(horizontal: false, vertical: true)

                    Spacer(minLength: 0)
                }
                .padding(MSCRemoteStyle.spaceMD)
                .frame(maxWidth: .infinity)
                .background(resultColor.opacity(0.08))
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .strokeBorder(resultColor.opacity(0.2), lineWidth: 1)
                )
                .padding(.top, MSCRemoteStyle.spaceMD)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(lastTestWasSuccess ? "Test succeeded" : "Test failed")
                .accessibilityValue(msg)
            }
        }
        .mscCard()
    }
}
