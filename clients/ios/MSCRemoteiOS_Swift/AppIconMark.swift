import SwiftUI
import UIKit

struct AppIconMark: View {
    let size: CGFloat

    var body: some View {
        Group {
            if let img = AppIconMark.loadPrimaryAppIcon() {
                Image(uiImage: img)
                    .renderingMode(.original)
                    .resizable()
                    .scaledToFit()
            } else {
                Image(systemName: "app")
                    .resizable()
                    .scaledToFit()
                    .foregroundStyle(.secondary)
            }
        }
        .frame(width: size, height: size)
    }

    private static func loadPrimaryAppIcon() -> UIImage? {
        let info = Bundle.main.infoDictionary

        if let icons = info?["CFBundleIcons"] as? [String: Any],
           let primary = icons["CFBundlePrimaryIcon"] as? [String: Any],
           let files = primary["CFBundleIconFiles"] as? [String],
           let name = files.last,
           let img = UIImage(named: name) {
            return img
        }

        if let icons = info?["CFBundleIcons~ipad"] as? [String: Any],
           let primary = icons["CFBundlePrimaryIcon"] as? [String: Any],
           let files = primary["CFBundleIconFiles"] as? [String],
           let name = files.last,
           let img = UIImage(named: name) {
            return img
        }

        return nil
    }
}
