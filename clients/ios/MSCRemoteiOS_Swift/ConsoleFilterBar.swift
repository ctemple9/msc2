import SwiftUI

/// A styled filter bar that appears inside the console output card.
///
/// Design intent: this feels like a native part of the terminal card, not a
/// generic UIKit search bar dropped in. It uses the same `bgElevated` surface
/// as other input controls in the app, with the accent color on focus.
///
/// The parent view is responsible for debouncing — this component just binds
/// to `text` and calls `onClear` when the X button is tapped.
struct ConsoleFilterBar: View {
    @Binding var text: String
    var onClear: () -> Void

    @FocusState private var isFocused: Bool

    var body: some View {
        HStack(spacing: MSCRemoteStyle.spaceSM) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(isFocused ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                .animation(.easeInOut(duration: 0.15), value: isFocused)

            TextField("Filter output…", text: $text)
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .tint(MSCRemoteStyle.accent)
                .focused($isFocused)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
                .submitLabel(.search)

            if !text.isEmpty {
                Button {
                    text = ""
                    onClear()
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 14))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                .buttonStyle(.plain)
                .transition(.opacity.combined(with: .scale(scale: 0.8)))
            }
        }
        .padding(.horizontal, MSCRemoteStyle.spaceMD)
        .padding(.vertical, MSCRemoteStyle.spaceSM)
        .background(MSCRemoteStyle.bgElevated)
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                .strokeBorder(
                    isFocused ? MSCRemoteStyle.accent.opacity(0.45) : MSCRemoteStyle.borderMid,
                    lineWidth: 1
                )
                .animation(.easeInOut(duration: 0.15), value: isFocused)
        )
    }
}
