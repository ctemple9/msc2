import SwiftUI
import UIKit

// Updated by SettingsStore when the user changes accent color.
// nonisolated(unsafe) because it is written on MainActor and read from view bodies on MainActor too,
// but the static nature means Swift can't verify that — the runtime access pattern is safe.
nonisolated(unsafe) var _mscAccentHex: String = "#3EB489"

// MARK: - MSC Remote Design System
// Single source of truth for all visual constants.

enum MSCRemoteStyle {
    // MARK: Backgrounds
    static let bgBase        = Color(hex: "#0D0D0F")
    static let bgCard        = Color(hex: "#16181C")
    static let bgElevated    = Color(hex: "#1E2026")
    static let borderSubtle  = Color.white.opacity(0.07)
    static let borderMid     = Color.white.opacity(0.14)

    // MARK: Accent — user-configurable tint colour
    static var accent:    Color { Color(hex: _mscAccentHex) }
    static var accentDim: Color { Color(hex: _mscAccentHex).opacity(0.15) }

    // MARK: Semantic (fixed — not affected by accent choice)
    static let success       = Color(hex: "#3EB489")
    static let danger        = Color(hex: "#E05C5C")
    static let warning       = Color(hex: "#E8A838")

    // MARK: Player action tints
    static let actionMessage   = Color(hex: "#5B9BD5")
    static let actionKick      = Color(hex: "#E8A838")
    static let actionBan       = Color(hex: "#9B6FD4")

    // MARK: Command category tints
    static let cmdGameplay     = Color(hex: "#E8A838")
    static let cmdEnvironment  = Color(hex: "#B87333")
    static let cmdPlayer       = Color(hex: "#7B9FD4")
    static let cmdModeration   = Color(hex: "#5B9BD5")
    static let cmdServer       = Color(hex: "#9B6FD4")

    // MARK: Deep backgrounds
    static let bgDeep          = Color(hex: "#0A0C0E")

    // MARK: Text
    static let textPrimary   = Color.white
    static let textSecondary = Color.white.opacity(0.55)
    static let textTertiary  = Color.white.opacity(0.30)

    // MARK: Spacing (4pt grid)
    static let spaceXS: CGFloat  = 4
    static let spaceSM: CGFloat  = 8
    static let spaceMD: CGFloat  = 12
    static let spaceLG: CGFloat  = 16
    static let spaceXL: CGFloat  = 24
    static let space2XL: CGFloat = 32

    // MARK: iPad layout
    //
    // Cards on iPad are constrained to a readable maximum width and centered.
    // 700pt prevents a single card stretching across a 12" screen.
    // iPadContentPadding gives the detail column comfortable gutters.
    static let contentMaxWidth: CGFloat    = 700
    static let iPadContentPadding: CGFloat = 32

    // MARK: Corner radius
    static let radiusSM: CGFloat = 10
    static let radiusMD: CGFloat = 14
    static let radiusLG: CGFloat = 18
}

extension Color {
    func toHex() -> String? {
        guard let components = UIColor(self).cgColor.components, components.count >= 3 else { return nil }
        let r = Int(components[0] * 255)
        let g = Int(components[1] * 255)
        let b = Int(components[2] * 255)
        return String(format: "#%02X%02X%02X", r, g, b)
    }

    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch hex.count {
        case 3:  (a, r, g, b) = (255, (int >> 8) * 17, (int >> 4 & 0xF) * 17, (int & 0xF) * 17)
        case 6:  (a, r, g, b) = (255, int >> 16, int >> 8 & 0xFF, int & 0xFF)
        case 8:  (a, r, g, b) = (int >> 24, int >> 16 & 0xFF, int >> 8 & 0xFF, int & 0xFF)
        default: (a, r, g, b) = (255, 0, 0, 0)
        }
        self.init(.sRGB, red: Double(r)/255, green: Double(g)/255, blue: Double(b)/255, opacity: Double(a)/255)
    }
}

// MARK: - Card modifier
struct MSCCardModifier: ViewModifier {
    var padding: CGFloat = MSCRemoteStyle.spaceLG
    func body(content: Content) -> some View {
        content
            .padding(padding)
            .background(MSCRemoteStyle.bgCard)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
                    .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1)
            )
    }
}

extension View {
    func mscCard(padding: CGFloat = MSCRemoteStyle.spaceLG) -> some View {
        modifier(MSCCardModifier(padding: padding))
    }
}

// MARK: - Reusable components

struct MSCSectionHeader: View {
    let title: String
    var trailing: String? = nil
    var body: some View {
        HStack {
            Text(title.uppercased())
                .font(.system(size: 10, weight: .semibold, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .kerning(1.2)
            Spacer()
            if let trailing {
                Text(trailing)
                    .font(.system(size: 10, weight: .regular, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
        }
        .padding(.horizontal, 2)
    }
}

struct MSCStatusDot: View {
    let isActive: Bool
    var size: CGFloat = 8
    var body: some View {
        Circle()
            .fill(isActive ? MSCRemoteStyle.success : MSCRemoteStyle.danger)
            .frame(width: size, height: size)
            .shadow(color: isActive ? MSCRemoteStyle.success.opacity(0.6) : MSCRemoteStyle.danger.opacity(0.4), radius: 4)
    }
}

struct MSCActionButton: View {
    let title: String
    let icon: String
    enum Style { case primary, danger }
    let style: Style
    let isEnabled: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Label(title, systemImage: icon)
                .font(.system(size: 14, weight: .semibold))
                .frame(maxWidth: .infinity)
                .frame(height: 44)
                .foregroundStyle(
                    isEnabled
                        ? (style == .primary ? MSCRemoteStyle.bgBase : .white)
                        : MSCRemoteStyle.textTertiary
                )
                .background(
                    isEnabled
                        ? (style == .primary ? MSCRemoteStyle.accent : MSCRemoteStyle.danger)
                        : MSCRemoteStyle.bgElevated
                )
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .strokeBorder(isEnabled ? Color.clear : MSCRemoteStyle.borderSubtle, lineWidth: 1)
                )
        }
        .disabled(!isEnabled)
    }
}

