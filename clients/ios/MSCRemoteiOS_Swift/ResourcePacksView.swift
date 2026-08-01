import SwiftUI

/// P8: iOS resource pack manager. Lists Java, Bedrock, and Geyser packs; lets
/// admins activate/deactivate, toggle Geyser packs, add by URL (Java), and remove.
/// Presented as a sheet from the Health tab's Resource Packs card.
struct ResourcePacksView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    var onDidChange: () -> Void = {}

    @State private var response: ResourcePacksResponseDTO? = nil
    @State private var isLoading: Bool = false
    @State private var errorText: String? = nil
    @State private var toast: String? = nil
    @State private var didChange: Bool = false

    // Action states
    @State private var actingOnPackId: String? = nil
    @State private var packToRemove: ResourcePackItemDTO? = nil
    @State private var showRemoveConfirm: Bool = false

    // Add by URL sheet
    @State private var showAddURL: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isAdmin: Bool { vm.connectedRole == "admin" }
    private var activeServer: ServerDTO? {
        if let activeId = vm.status?.activeServerId,
           let server = vm.servers.first(where: { $0.id == activeId }) {
            return server
        }
        return vm.servers.first
    }
    private var supportsGeyserPacks: Bool {
        activeServer?.supportsJavaBedrockCrossPlay ?? true
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
            .navigationTitle("Resource Packs")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    if let r = response, r.isJava, isAdmin {
                        Button {
                            showAddURL = true
                        } label: {
                            Image(systemName: "link.badge.plus")
                                .foregroundStyle(MSCRemoteStyle.accent)
                        }
                    }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") {
                        if didChange { onDidChange() }
                        dismiss()
                    }
                    .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .task { await loadPacks() }
            .sheet(isPresented: $showAddURL) {
                AddPackByURLView { url, sha1, require in
                    Task { await applySetURL(url: url, sha1: sha1, require: require) }
                }
                .environmentObject(settings)
                .environmentObject(vm)
            }
            .confirmationDialog(
                "Remove \"\(packToRemove?.name ?? "")\"?",
                isPresented: $showRemoveConfirm,
                titleVisibility: .visible
            ) {
                Button("Remove Pack", role: .destructive) {
                    if let pack = packToRemove {
                        Task { await applyRemove(pack: pack) }
                    }
                }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("The file will be permanently deleted from the server folder.")
            }
        }
    }

    // MARK: - Content

    @ViewBuilder
    private var content: some View {
        if isLoading && response == nil {
            loadingState
        } else if let errorText {
            errorState(errorText)
        } else if let r = response {
            mainContent(r)
        } else {
            Color.clear
        }
    }

    private var loadingState: some View {
        VStack(spacing: MSCRemoteStyle.spaceMD) {
            ProgressView()
            Text("Loading packs…")
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func errorState(_ text: String) -> some View {
        VStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "wifi.exclamationmark")
                .font(.system(size: 32, weight: .light))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Text("Couldn't load packs")
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
            Text(text)
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
            Button("Retry") { Task { await loadPacks() } }
                .foregroundStyle(MSCRemoteStyle.accent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder
    private func mainContent(_ r: ResourcePacksResponseDTO) -> some View {
        ScrollView(showsIndicators: false) {
            VStack(spacing: MSCRemoteStyle.spaceLG) {
                if r.isJava, let activeUrl = r.activePackUrl {
                    activeURLBanner(activeUrl, require: r.requirePack)
                }

                if r.packs.isEmpty && !(r.isGeyserAvailable && supportsGeyserPacks) {
                    emptyState(isJava: r.isJava)
                } else {
                    if !r.packs.isEmpty || r.isJava {
                        packsSection(packs: r.packs, isJava: r.isJava)
                    }
                    if r.isGeyserAvailable && supportsGeyserPacks {
                        geyserSection(packs: r.geyserPacks)
                    }
                }
            }
            .padding(.horizontal, MSCRemoteStyle.spaceLG)
            .padding(.top, MSCRemoteStyle.spaceMD)
            .padding(.bottom, MSCRemoteStyle.spaceLG)
            .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
            .frame(maxWidth: .infinity)
        }
    }

    // MARK: - Active URL banner

    private func activeURLBanner(_ url: String, require: Bool) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Image(systemName: "link.circle.fill")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.success)
                Text(require ? "Pack required (players must accept)" : "Pack optional (players may decline)")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                Spacer()
                if isAdmin {
                    Button {
                        Task { await applyActivate(packId: nil, require: false) }
                    } label: {
                        Text("Clear")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundStyle(MSCRemoteStyle.danger)
                    }
                    .buttonStyle(.plain)
                    .disabled(actingOnPackId == "__clear__")
                }
            }
            Text(url)
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .lineLimit(2)
        }
        .padding(MSCRemoteStyle.spaceMD)
        .background(MSCRemoteStyle.success.opacity(0.08))
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
    }

    // MARK: - Packs section

    private func packsSection(packs: [ResourcePackItemDTO], isJava: Bool) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: isJava ? "Java Packs" : "Resource Packs")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if packs.isEmpty {
                Text(isJava ? "No .zip packs installed. Add one using the link button above." : "No packs installed.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .padding(.vertical, MSCRemoteStyle.spaceMD)
            } else {
                ForEach(Array(packs.enumerated()), id: \.element.id) { idx, pack in
                    packRow(pack, packKind: isJava ? "java" : "bedrock")
                    if idx < packs.count - 1 {
                        Divider().background(MSCRemoteStyle.borderSubtle)
                    }
                }
            }
        }
        .mscCard()
    }

    @ViewBuilder
    private func packRow(_ pack: ResourcePackItemDTO, packKind: String) -> some View {
        let isActing = actingOnPackId == pack.id

        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: packKind == "java" ? "archivebox.fill" : "cube.fill")
                .font(.system(size: 18))
                .foregroundStyle(pack.isActive ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                .frame(width: 28)

            VStack(alignment: .leading, spacing: 3) {
                Text(pack.name)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .lineLimit(1)
                HStack(spacing: 6) {
                    Text(pack.fileSizeDisplay)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                    if packKind == "java" {
                        Text(pack.isActive ? "Active" : "Off")
                            .font(.system(size: 10, weight: .semibold))
                            .foregroundStyle(pack.isActive ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                            .padding(.horizontal, 5)
                            .padding(.vertical, 2)
                            .background(Capsule().fill((pack.isActive ? MSCRemoteStyle.success : Color.gray).opacity(0.15)))
                    } else {
                        Text(pack.typeLabel)
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
            }

            Spacer()

            if isAdmin {
                if isActing {
                    ProgressView().scaleEffect(0.8).frame(width: 44)
                } else {
                    HStack(spacing: MSCRemoteStyle.spaceSM) {
                        if packKind == "java" {
                            Button {
                                Task { await applyActivate(packId: pack.isActive ? nil : pack.id, require: false) }
                            } label: {
                                Image(systemName: pack.isActive ? "checkmark.circle.fill" : "circle")
                                    .font(.system(size: 20))
                                    .foregroundStyle(pack.isActive ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                            }
                            .buttonStyle(.plain)
                        }

                        Button {
                            packToRemove = pack
                            showRemoveConfirm = true
                        } label: {
                            Image(systemName: "trash")
                                .font(.system(size: 15))
                                .foregroundStyle(MSCRemoteStyle.danger.opacity(0.8))
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
    }

    // MARK: - Geyser section

    private func geyserSection(packs: [ResourcePackItemDTO]) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 4) {
                MSCSectionHeader(title: "Bedrock Players (Geyser)")
                    .padding(.bottom, 2)
                Text("Served to Bedrock/Xbox players via Geyser. Restart the server after changes.")
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            if packs.isEmpty {
                Text("No Bedrock packs added for Geyser.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .padding(.vertical, MSCRemoteStyle.spaceMD)
            } else {
                ForEach(Array(packs.enumerated()), id: \.element.id) { idx, pack in
                    geyserRow(pack)
                    if idx < packs.count - 1 {
                        Divider().background(MSCRemoteStyle.borderSubtle)
                    }
                }
            }
        }
        .mscCard()
    }

    @ViewBuilder
    private func geyserRow(_ pack: ResourcePackItemDTO) -> some View {
        let isActing = actingOnPackId == pack.id

        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "cube.transparent.fill")
                .font(.system(size: 18))
                .foregroundStyle(pack.isActive ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                .frame(width: 28)

            VStack(alignment: .leading, spacing: 3) {
                Text(pack.name)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .lineLimit(1)
                HStack(spacing: 6) {
                    Text(pack.fileSizeDisplay)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                    Text(pack.isActive ? "Enabled" : "Disabled")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(pack.isActive ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 2)
                        .background(Capsule().fill((pack.isActive ? MSCRemoteStyle.success : Color.gray).opacity(0.15)))
                }
            }

            Spacer()

            if isAdmin {
                if isActing {
                    ProgressView().scaleEffect(0.8).frame(width: 72)
                } else {
                    HStack(spacing: MSCRemoteStyle.spaceSM) {
                        Toggle("", isOn: Binding(
                            get: { pack.isActive },
                            set: { newVal in Task { await applyGeyserToggle(pack: pack, enabled: newVal) } }
                        ))
                        .toggleStyle(.switch)
                        .labelsHidden()
                        .scaleEffect(0.8)

                        Button {
                            packToRemove = pack
                            showRemoveConfirm = true
                        } label: {
                            Image(systemName: "trash")
                                .font(.system(size: 15))
                                .foregroundStyle(MSCRemoteStyle.danger.opacity(0.8))
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
    }

    // MARK: - Empty state

    private func emptyState(isJava: Bool) -> some View {
        VStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "shippingbox")
                .font(.system(size: 32, weight: .light))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Text("No resource packs installed")
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
            Text(isJava
                 ? "Use the link button above to add a Java resource pack by URL."
                 : "Add resource packs directly on the Mac server."
            )
            .font(.system(size: 13))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .multilineTextAlignment(.center)
            .padding(.horizontal, MSCRemoteStyle.spaceLG)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 48)
    }

    // MARK: - Actions

    private func loadPacks() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else {
            errorText = "Not paired. Set Base URL + Token in Settings."
            return
        }
        isLoading = true
        errorText = nil
        let result = await vm.fetchResourcePacks(baseURL: baseURL, token: token)
        if let result {
            response = result
        } else {
            errorText = "Couldn't load packs. Check your connection and try again."
        }
        isLoading = false
    }

    private func applyActivate(packId: String?, require: Bool) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let sentinelId = packId ?? "__clear__"
        actingOnPackId = sentinelId
        let result = await vm.activateResourcePack(baseURL: baseURL, token: token, packId: packId, require: require)
        actingOnPackId = nil
        handleMutationResult(result, successMessage: packId == nil ? "Pack deactivated." : "Pack activated — players apply it on next join.")
    }

    private func applySetURL(url: String, sha1: String?, require: Bool) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let result = await vm.setResourcePackURL(baseURL: baseURL, token: token, url: url, sha1: sha1, require: require)
        handleMutationResult(result, successMessage: "URL set — players apply it on next join.")
    }

    private func applyGeyserToggle(pack: ResourcePackItemDTO, enabled: Bool) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        actingOnPackId = pack.id
        let result = await vm.toggleGeyserPack(baseURL: baseURL, token: token, packId: pack.id, enabled: enabled)
        actingOnPackId = nil
        handleMutationResult(result, successMessage: "\(pack.name) \(enabled ? "enabled" : "disabled") — restart to apply.")
    }

    private func applyRemove(pack: ResourcePackItemDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        actingOnPackId = pack.id
        let result = await vm.removeResourcePack(baseURL: baseURL, token: token, packId: pack.id, packKind: pack.packKind)
        actingOnPackId = nil
        handleMutationResult(result, successMessage: "\(pack.name) removed.")
    }

    private func handleMutationResult(_ result: ResourcePackMutationResultDTO?, successMessage: String) {
        if let result {
            if result.success {
                if let updated = result.updated { response = updated }
                didChange = true
                showToast(successMessage)
            } else {
                switch result.message {
                case "java_only":       showToast("This action is only available for Java servers.")
                case "no_active_server": showToast("No active server selected.")
                case "pack_not_found":  showToast("Pack not found — it may have already been removed.")
                case "no_host_address": showToast("No host address configured. Set a DuckDNS name on the Mac first.")
                default:               showToast(result.message)
                }
            }
        } else {
            showToast("Request failed. Check your connection and try again.")
        }
    }

    private func showToast(_ message: String) {
        withAnimation { toast = message }
        Task {
            try? await Task.sleep(nanoseconds: 3_500_000_000)
            withAnimation { toast = nil }
        }
    }
}

// MARK: - Add Pack by URL sheet

private struct AddPackByURLView: View {
    @Environment(\.dismiss) private var dismiss

    // Must NOT be an `async` closure: a stored `async` closure property is typed
    // `nonisolated(nonsending)` and its function-type metadata accessor is null, which
    // crashes AttributeGraph (jump to 0x0) when this view is presented. Caller wraps in a Task.
    let onAdd: (String, String?, Bool) -> Void

    @State private var urlText: String = ""
    @State private var sha1Text: String = ""
    @State private var require: Bool = true

    private var urlIsValid: Bool {
        let trimmed = urlText.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.hasPrefix("http://") || trimmed.hasPrefix("https://")
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                        MSCSectionHeader(title: "Pack URL")
                            .padding(.bottom, 4)

                        VStack(spacing: MSCRemoteStyle.spaceSM) {
                            urlField
                            sha1Field
                            requireToggleRow
                        }

                        Text("The URL must be publicly reachable by the players joining your server (not a local LAN address). For best results, use a direct link to a .zip file.")
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .mscCard()

                    MSCActionButton(title: "Set Pack URL", icon: "link",
                                    style: .primary, isEnabled: urlIsValid) {
                        submit()
                    }

                    Spacer()
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.top, MSCRemoteStyle.spaceMD)
            }
            .navigationTitle("Add by URL")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Cancel") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
    }

    private var urlField: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("URL")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
            TextField("https://example.com/pack.zip", text: $urlText)
                .textFieldStyle(.plain)
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
                .keyboardType(.URL)
                .padding(MSCRemoteStyle.spaceSM)
                .background(MSCRemoteStyle.bgBase)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM - 2, style: .continuous))
        }
    }

    private var sha1Field: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("SHA1 (optional but recommended)")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
            TextField("hex SHA1 of the .zip file", text: $sha1Text)
                .textFieldStyle(.plain)
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
                .padding(MSCRemoteStyle.spaceSM)
                .background(MSCRemoteStyle.bgBase)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM - 2, style: .continuous))
        }
    }

    private var requireToggleRow: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text("Require pack")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                Text("Players who decline are disconnected.")
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
            Spacer()
            Toggle("", isOn: $require)
                .labelsHidden()
        }
    }

    private func submit() {
        let url = urlText.trimmingCharacters(in: .whitespacesAndNewlines)
        let sha1 = sha1Text.trimmingCharacters(in: .whitespacesAndNewlines)
        onAdd(url, sha1.isEmpty ? nil : sha1, require)
        dismiss()
    }
}
