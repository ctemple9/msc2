import SwiftUI

/// P6: the iOS mod browser. Searches the Mac's Modrinth catalog (filtered to the
/// active server's loader + Minecraft version by the server) and installs the latest
/// compatible version into the server's add-on folder. Presented as a sheet from the
/// Server tab's Mods card; on dismiss the caller refreshes so new mods appear.
struct CatalogBrowserView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    /// Called after at least one successful install so the parent can refresh its lists.
    var onDidInstall: () -> Void = {}

    @State private var searchText: String = ""
    @State private var response: CatalogSearchResponseDTO? = nil
    @State private var isLoading: Bool = false
    @State private var errorText: String? = nil
    @State private var searchTask: Task<Void, Never>? = nil

    @State private var installing: Set<String> = []
    @State private var installed: Set<String> = []
    @State private var toast: String? = nil
    @State private var didInstallAnything: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isAdmin: Bool { vm.connectedRole == "admin" }

    private var addonNoun: String {
        switch response?.addonKind {
        case "plugin": return "plugins"
        case "mod":    return "mods"
        default:       return "mods"
        }
    }

    private var subtitle: String? {
        guard let r = response, r.supportsAddons else { return nil }
        let loader = r.loaderName ?? ""
        let ver = r.gameVersion.map { " · \($0)" } ?? ""
        return "Modrinth · \(loader)\(ver)"
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: 0) {
                    searchBar
                    Divider().background(MSCRemoteStyle.borderSubtle)
                    content
                }

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
            .navigationTitle("Add \(addonNoun.capitalized)")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") {
                        if didInstallAnything { onDidInstall() }
                        dismiss()
                    }
                    .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .task { await runSearch() }
            .onChange(of: searchText) { _, _ in scheduleSearch() }
        }
    }

    // MARK: - Search bar

    private var searchBar: some View {
        VStack(spacing: 6) {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                TextField("Search \(addonNoun)…", text: $searchText)
                    .textFieldStyle(.plain)
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .autocorrectionDisabled()
                    .textInputAutocapitalization(.never)
                if !searchText.isEmpty {
                    Button { searchText = "" } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                    .buttonStyle(.plain)
                }
                if isLoading { ProgressView().scaleEffect(0.7) }
            }
            .padding(.horizontal, MSCRemoteStyle.spaceMD)
            .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
            .background(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .fill(MSCRemoteStyle.bgElevated)
            )

            if let subtitle {
                Text(subtitle)
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .padding(.horizontal, MSCRemoteStyle.spaceLG)
        .padding(.vertical, MSCRemoteStyle.spaceMD)
    }

    // MARK: - Content

    @ViewBuilder
    private var content: some View {
        if let r = response, !r.supportsAddons {
            unsupportedState(note: r.note)
        } else if let errorText {
            errorState(errorText)
        } else if isLoading && (response?.results.isEmpty ?? true) {
            VStack(spacing: MSCRemoteStyle.spaceMD) {
                ProgressView()
                Text("Searching Modrinth…")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let results = response?.results, results.isEmpty {
            emptyState
        } else if let results = response?.results {
            ScrollView(showsIndicators: false) {
                LazyVStack(spacing: 0) {
                    ForEach(Array(results.enumerated()), id: \.element.id) { idx, item in
                        resultRow(item)
                        if idx < results.count - 1 {
                            Divider().background(MSCRemoteStyle.borderSubtle)
                        }
                    }
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.vertical, MSCRemoteStyle.spaceMD)
                .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                .frame(maxWidth: .infinity)
            }
        } else {
            Color.clear
        }
    }

    private func resultRow(_ item: CatalogItemDTO) -> some View {
        HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
            icon(item)
            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(item.title)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                        .lineLimit(1)
                    if item.isClientOnly { clientOnlyBadge }
                }
                Text("by \(item.author) · \(formatDownloads(item.downloads)) downloads")
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                if !item.description.isEmpty {
                    Text(item.description)
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                        .lineLimit(2)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: 6)
            installControl(item)
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
        .opacity(item.isClientOnly ? 0.7 : 1)
    }

    @ViewBuilder
    private func icon(_ item: CatalogItemDTO) -> some View {
        let placeholder = RoundedRectangle(cornerRadius: 8, style: .continuous)
            .fill(MSCRemoteStyle.bgElevated)
            .overlay(
                Image(systemName: "shippingbox")
                    .font(.system(size: 16))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            )
        if let urlStr = item.iconURL, let url = URL(string: urlStr) {
            AsyncImage(url: url) { phase in
                if let image = phase.image {
                    image.resizable().scaledToFit()
                } else {
                    placeholder
                }
            }
            .frame(width: 44, height: 44)
            .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
        } else {
            placeholder.frame(width: 44, height: 44)
        }
    }

    @ViewBuilder
    private func installControl(_ item: CatalogItemDTO) -> some View {
        if installed.contains(item.projectId) {
            Label("Added", systemImage: "checkmark.circle.fill")
                .labelStyle(.iconOnly)
                .font(.system(size: 20))
                .foregroundStyle(MSCRemoteStyle.success)
                .frame(width: 72, height: 30)
        } else if installing.contains(item.projectId) {
            ProgressView()
                .frame(width: 72, height: 30)
        } else if isAdmin {
            Button {
                Task { await install(item) }
            } label: {
                Text("Install")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(.white)
                    .frame(width: 72, height: 30)
                    .background(MSCRemoteStyle.accent)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            }
            .buttonStyle(.plain)
        } else {
            // Guests can browse but not install.
            EmptyView()
        }
    }

    private var clientOnlyBadge: some View {
        Text("CLIENT-ONLY")
            .font(.system(size: 8, weight: .semibold))
            .tracking(0.5)
            .padding(.horizontal, 5)
            .padding(.vertical, 2)
            .background(Capsule().fill(MSCRemoteStyle.warning.opacity(0.18)))
            .foregroundStyle(MSCRemoteStyle.warning)
    }

    // MARK: - States

    private func unsupportedState(note: String?) -> some View {
        let message: String
        switch note {
        case "no_active_server": message = "No active server. Select a server first."
        case "not_supported":    message = "This server type doesn't support installable mods."
        default:                 message = "Mods aren't available for this server."
        }
        return VStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "shippingbox")
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

    private var emptyState: some View {
        VStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 32, weight: .light))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Text(searchText.isEmpty
                 ? "No \(addonNoun) found for this server."
                 : "No results for \"\(searchText)\".")
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
            Text("Couldn't reach the catalog")
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
            Text(text)
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
            Button("Retry") { Task { await runSearch() } }
                .foregroundStyle(MSCRemoteStyle.accent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Actions

    private func scheduleSearch() {
        searchTask?.cancel()
        searchTask = Task {
            try? await Task.sleep(nanoseconds: 350_000_000)
            guard !Task.isCancelled else { return }
            await runSearch()
        }
    }

    private func runSearch() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else {
            errorText = "Not paired. Set Base URL + Token in Settings."
            return
        }
        isLoading = true
        errorText = nil
        let result = await vm.searchCatalog(baseURL: baseURL, token: token, query: searchText)
        if let result {
            response = result
            errorText = nil
        } else {
            errorText = "The search request failed. Check your connection and try again."
        }
        isLoading = false
    }

    private func install(_ item: CatalogItemDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        installing.insert(item.projectId)
        let result = await vm.installCatalogAddon(baseURL: baseURL, token: token, item: item)
        installing.remove(item.projectId)

        if let result {
            if result.success {
                installed.insert(item.projectId)
                didInstallAnything = true
                showToast("\(result.message) — restart the server to apply.")
            } else {
                showToast(result.message)
            }
        } else {
            // Inconclusive: the request timed out or the connection dropped, but the
            // install may still be completing on the Mac. Treat as tentatively done.
            installed.insert(item.projectId)
            didInstallAnything = true
            showToast("Install is taking a while — check the Components list to confirm it landed.")
        }
    }

    private func showToast(_ message: String) {
        withAnimation { toast = message }
        Task {
            try? await Task.sleep(nanoseconds: 3_500_000_000)
            withAnimation { toast = nil }
        }
    }

    private func formatDownloads(_ n: Int) -> String {
        if n >= 1_000_000 { return String(format: "%.1fM", Double(n) / 1_000_000) }
        if n >= 1_000 { return String(format: "%.1fK", Double(n) / 1_000) }
        return "\(n)"
    }
}
