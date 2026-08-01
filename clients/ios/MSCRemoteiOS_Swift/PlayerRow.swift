import SwiftUI
struct PlayerRow: View {
    let player: PlayerDTO

    // Crafatar avatar URL, if UUID is available.
    // The `overlay=true` param applies the player's helmet layer,
    // which gives a more accurate skin preview.
    private var avatarURL: URL? {
        guard let uuid = player.uuid, !uuid.isEmpty else { return nil }
        return URL(string: "https://crafatar.com/avatars/\(uuid)?size=32&overlay=true")
    }

    var body: some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            // Avatar: AsyncImage handles loading/failure states.
            // If no UUID, falls through to the placeholder immediately.
            Group {
                if let url = avatarURL {
                    AsyncImage(url: url) { phase in
                        switch phase {
                        case .success(let image):
                            image
                                .resizable()
                                .interpolation(.none)  // pixel-art scaling
                                .scaledToFit()
                        case .failure:
                            genericAvatarIcon
                        case .empty:
                            // Loading state: small shimmer substitute
                            RoundedRectangle(cornerRadius: 4, style: .continuous)
                                .fill(MSCRemoteStyle.bgElevated)
                        @unknown default:
                            genericAvatarIcon
                        }
                    }
                } else {
                    genericAvatarIcon
                }
            }
            .frame(width: 32, height: 32)
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
            .accessibilityHidden(true)

            Text(player.name)
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(MSCRemoteStyle.textPrimary)

            Spacer()
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM)
        .accessibilityElement(children: .combine)
    }

    private var genericAvatarIcon: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 4, style: .continuous)
                .fill(MSCRemoteStyle.bgElevated)
            Image(systemName: "person.fill")
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
    }
}
