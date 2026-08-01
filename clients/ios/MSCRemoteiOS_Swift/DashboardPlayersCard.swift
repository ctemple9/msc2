import SwiftUI

struct DashboardPlayersCard: View {
    let players: [PlayerDTO]
    let isRunning: Bool

    private var title: String {
        isRunning && !players.isEmpty ? "Players (\(players.count))" : "Players"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: title)
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if !isRunning || players.isEmpty {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    Image(systemName: "person.slash")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .accessibilityHidden(true)
                    Text("No players online")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            } else {
                VStack(spacing: 0) {
                    ForEach(players) { player in
                        PlayerRow(player: player)

                        if player.id != players.last?.id {
                            Divider()
                                .background(MSCRemoteStyle.borderSubtle)
                                .padding(.leading, 44)
                        }
                    }
                }
            }
        }
        .mscCard()
    }
}
