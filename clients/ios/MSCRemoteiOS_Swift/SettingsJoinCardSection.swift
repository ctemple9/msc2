import SwiftUI

struct SettingsJoinCardSection: View {
    @EnvironmentObject private var settings: SettingsStore
    @Binding var showJoinCard: Bool
    let saveAction: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Join Card")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Show Join Card on Dashboard")
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("Displays a flippable server join card below the history chart. Front is shareable as an image. Back shows connection details.")
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer()
                Toggle("", isOn: $showJoinCard)
                    .labelsHidden()
                    .tint(MSCRemoteStyle.accent)
                    .onChange(of: showJoinCard) { _, _ in
                        saveAction()
                    }
                    .accessibilityLabel("Show Join Card on Dashboard")
            }
        }
        .mscCard()
    }
}
