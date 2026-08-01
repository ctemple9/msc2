import SwiftUI

/// P7: lets admins pick and apply a server JAR version from the Mac's version lists.
/// Presented as a sheet from the Health tab's Version card; on dismiss (if changed)
/// the caller refreshes so Components reflects the new JAR.
struct ServerVersionView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    /// Called after a successful (or inconclusive) version change so the parent can refresh.
    var onDidChange: () -> Void = {}

    @State private var versionsResponse: VersionsResponseDTO? = nil
    @State private var isLoading: Bool = false
    @State private var errorText: String? = nil
    @State private var selectedVersionId: String? = nil
    @State private var isApplying: Bool = false
    @State private var toast: String? = nil
    @State private var didChange: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isAdmin: Bool { vm.connectedRole == "admin" }
    private var serverRunning: Bool { vm.status?.running == true }

    private var selectedEntry: VersionEntryDTO? {
        versionsResponse?.versions.first { $0.id == selectedVersionId }
    }

    private var isCurrentVersionSelected: Bool {
        guard let current = versionsResponse?.currentVersion, let sel = selectedVersionId else { return false }
        return current == sel
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                content

                if let toast {
                    VStack {
                        Spacer()
                        Text(toast)
                            .font(.system(size: 13, weight: .medium))
                            .foregroundStyle(.white)
                            .padding(.horizontal, MSCRemoteStyle.spaceLG)
                            .padding(.vertical, MSCRemoteStyle.spaceMD)
                            .frame(maxWidth: MSCRemoteStyle.contentMaxWidth - 40)
                            .background(MSCRemoteStyle.bgElevated)
                            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                            .padding(.bottom, MSCRemoteStyle.spaceLG)
                            .padding(.horizontal, MSCRemoteStyle.spaceLG)
                            .multilineTextAlignment(.center)
                    }
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                }
            }
            .navigationTitle("Change Version")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") {
                        if didChange { onDidChange() }
                        dismiss()
                    }
                    .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .task { await loadVersions() }
        }
    }

    // MARK: - Content

    @ViewBuilder
    private var content: some View {
        if isLoading && versionsResponse == nil {
            loadingState
        } else if let r = versionsResponse, !r.supportsVersions {
            unsupportedState(note: r.note)
        } else if let errorText {
            errorState(errorText)
        } else if let r = versionsResponse {
            mainContent(r)
        } else {
            Color.clear
        }
    }

    private var loadingState: some View {
        VStack(spacing: MSCRemoteStyle.spaceMD) {
            ProgressView()
            Text("Loading versions…")
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func unsupportedState(note: String?) -> some View {
        let message: String
        switch note {
        case "no_active_server":  message = "No active server. Select a server first."
        case "latest_only":       message = "This server flavor only supports the latest version."
        case "not_available":     message = "Version selection isn't available right now."
        default:                  message = "Version selection isn't supported for this server."
        }
        return VStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "server.rack")
                .font(.system(size: 32, weight: .light))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Text(message)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func errorState(_ text: String) -> some View {
        VStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "wifi.exclamationmark")
                .font(.system(size: 32, weight: .light))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Text("Couldn't load versions")
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
            Text(text)
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
            Button("Retry") { Task { await loadVersions() } }
                .foregroundStyle(MSCRemoteStyle.accent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder
    private func mainContent(_ r: VersionsResponseDTO) -> some View {
        VStack(spacing: 0) {
            warningBanner
            Divider().background(MSCRemoteStyle.borderSubtle)

            ScrollView(showsIndicators: false) {
                LazyVStack(spacing: 0) {
                    if let current = r.currentVersion {
                        currentVersionHeader(current, flavorName: r.flavorName)
                    }
                    ForEach(Array(r.versions.enumerated()), id: \.element.id) { idx, entry in
                        versionRow(entry, currentVersionId: r.currentVersion)
                        if idx < r.versions.count - 1 {
                            Divider()
                                .background(MSCRemoteStyle.borderSubtle)
                                .padding(.leading, MSCRemoteStyle.spaceLG)
                        }
                    }
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.vertical, MSCRemoteStyle.spaceMD)
                .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                .frame(maxWidth: .infinity)
            }

            if isAdmin {
                Divider().background(MSCRemoteStyle.borderSubtle)
                applyFooter
            }
        }
    }

    private var warningBanner: some View {
        Group {
            if serverRunning {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.danger)
                    Text("Stop the server before changing versions.")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Spacer()
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.vertical, MSCRemoteStyle.spaceMD)
                .background(MSCRemoteStyle.danger.opacity(0.12))
            } else {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    Image(systemName: "info.circle.fill")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.warning)
                    Text("The server will need a restart after applying. For modded flavors, the installer will re-run.")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Spacer()
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.vertical, MSCRemoteStyle.spaceMD)
                .background(MSCRemoteStyle.warning.opacity(0.1))
            }
        }
    }

    private func currentVersionHeader(_ current: String, flavorName: String) -> some View {
        HStack(spacing: 6) {
            Image(systemName: "checkmark.seal.fill")
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.success)
            Text(flavorName.isEmpty ? "Current: \(current)" : "\(flavorName) · \(current)")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.bottom, MSCRemoteStyle.spaceSM)
    }

    @ViewBuilder
    private func versionRow(_ entry: VersionEntryDTO, currentVersionId: String?) -> some View {
        let isSelected = selectedVersionId == entry.id
        let isCurrent = entry.id == currentVersionId

        Button {
            guard isAdmin && !serverRunning && !isApplying else { return }
            selectedVersionId = entry.id
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        Text(entry.displayLabel)
                            .font(.system(size: 14, weight: isSelected ? .semibold : .regular))
                            .foregroundStyle(isSelected ? MSCRemoteStyle.accent : MSCRemoteStyle.textPrimary)
                        if entry.isLatest {
                            versionBadge("LATEST", color: MSCRemoteStyle.accent)
                        }
                        if !entry.isStable {
                            versionBadge("BETA", color: MSCRemoteStyle.warning)
                        }
                    }
                    if let loader = entry.loaderVersion {
                        Text("Loader: \(loader)")
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    } else if let build = entry.buildLabel {
                        Text(build)
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
                Spacer(minLength: 6)
                if isSelected {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 18))
                        .foregroundStyle(isCurrent ? MSCRemoteStyle.success : MSCRemoteStyle.accent)
                } else if isCurrent {
                    Image(systemName: "checkmark.circle")
                        .font(.system(size: 18))
                        .foregroundStyle(MSCRemoteStyle.success.opacity(0.5))
                }
            }
            .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .opacity((serverRunning || isApplying) ? 0.5 : 1)
    }

    private func versionBadge(_ label: String, color: Color) -> some View {
        Text(label)
            .font(.system(size: 8, weight: .semibold))
            .tracking(0.5)
            .padding(.horizontal, 5)
            .padding(.vertical, 2)
            .background(Capsule().fill(color.opacity(0.18)))
            .foregroundStyle(color)
    }

    private var applyFooter: some View {
        VStack(spacing: MSCRemoteStyle.spaceMD) {
            if isApplying {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    ProgressView()
                    Text("Applying… this may take a while.")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                }
                .padding(.vertical, MSCRemoteStyle.spaceMD)
            } else {
                MSCActionButton(
                    title: "Apply Version",
                    icon: "arrow.triangle.2.circlepath",
                    style: .primary,
                    isEnabled: !isCurrentVersionSelected && selectedVersionId != nil && !serverRunning
                ) {
                    Task { await applySelected() }
                }
            }
        }
        .padding(.horizontal, MSCRemoteStyle.spaceLG)
        .padding(.vertical, MSCRemoteStyle.spaceMD)
        .padding(.bottom, MSCRemoteStyle.spaceSM)
    }

    // MARK: - Actions

    private func loadVersions() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else {
            errorText = "Not paired. Set Base URL + Token in Settings."
            return
        }
        isLoading = true
        errorText = nil
        let result = await vm.fetchVersions(baseURL: baseURL, token: token)
        if let result {
            versionsResponse = result
            if selectedVersionId == nil, let current = result.currentVersion {
                selectedVersionId = current
            }
        } else {
            errorText = "Couldn't load versions. Check your connection and try again."
        }
        isLoading = false
    }

    private func applySelected() async {
        guard let entry = selectedEntry,
              let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isApplying = true
        let result = await vm.changeVersion(baseURL: baseURL, token: token, entry: entry)
        isApplying = false

        if let result {
            if result.success {
                didChange = true
                showToast("Applied — restart the server to load the new version.")
                scheduleAutoDismiss()
            } else {
                switch result.message {
                case "server_running":
                    showToast("Stop the server before changing versions.")
                case "download_in_progress":
                    showToast("A download is already in progress. Try again in a moment.")
                case "no_active_server":
                    showToast("No active server selected.")
                case "backup_failed":
                    showToast("Pre-downgrade backup failed. Resolve the backup issue on your Mac and try again.")
                default:
                    showToast(result.message)
                }
            }
        } else {
            // Inconclusive — the request timed out but the install may still be running on the Mac.
            didChange = true
            showToast("This is taking a while — check Components to confirm it landed, then restart.")
            scheduleAutoDismiss()
        }
    }

    private func scheduleAutoDismiss() {
        Task {
            try? await Task.sleep(nanoseconds: 2_500_000_000)
            if didChange { onDidChange() }
            dismiss()
        }
    }

    private func showToast(_ message: String) {
        withAnimation { toast = message }
        Task {
            try? await Task.sleep(nanoseconds: 4_000_000_000)
            withAnimation { toast = nil }
        }
    }
}
