import SwiftUI

// MARK: - AllowlistView
//
// Bedrock-only screen for viewing and editing the server allowlist (whitelist).
// Pushed from PlayersView via a NavigationLink that only appears for Bedrock
// servers, so this view intentionally does NOT wrap itself in a NavigationStack
// — it inherits the pushing view's navigation chrome.
//
// Mutations (add / remove) are admin-gated: the controls only render when the
// connected token is an admin, mirroring the server-side adminOnlyPOSTPaths gate.

struct AllowlistView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var newEntry: String = ""
    @State private var isLoading: Bool = false
    @State private var isMutating: Bool = false
    @State private var entryToRemove: AllowlistEntryDTO? = nil
    @State private var toastMessage: String? = nil
    @State private var localError: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }
    private var isAdmin: Bool { vm.connectedRole == "admin" }

    private var trimmedNewEntry: String {
        newEntry.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            ScrollView(showsIndicators: false) {
                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    allowlistCard
                    infoCard
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.top, MSCRemoteStyle.spaceMD)
                .padding(.bottom, MSCRemoteStyle.spaceLG)
                .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                .frame(maxWidth: .infinity)
            }
            .refreshable { await refresh() }

            if let toast = toastMessage {
                VStack {
                    Spacer()
                    Text(toast)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(.white)
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        .padding(.vertical, MSCRemoteStyle.spaceMD)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(Capsule())
                        .padding(.bottom, MSCRemoteStyle.spaceLG)
                }
                .transition(.move(edge: .bottom).combined(with: .opacity))
                .animation(.spring(response: 0.4, dampingFraction: 0.8), value: toast)
            }
        }
        .navigationTitle("Allowlist")
        .navigationBarTitleDisplayMode(.large)
        .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
        .toolbarColorScheme(.dark, for: .navigationBar)
        .task(id: isPaired) {
            guard isPaired else { return }
            await refresh()
        }
        .alert("Remove Player", isPresented: Binding(
            get: { entryToRemove != nil },
            set: { if !$0 { entryToRemove = nil } }
        )) {
            Button("Cancel", role: .cancel) { entryToRemove = nil }
            Button("Remove", role: .destructive) {
                guard let entry = entryToRemove else { return }
                entryToRemove = nil
                Task { await performMutation(action: "remove", name: entry.name) }
            }
        } message: {
            if let entry = entryToRemove {
                Text("Remove \"\(entry.name)\" from the allowlist?")
            }
        }
    }

    // MARK: - Allowlist Card

    private var allowlistCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(
                title: "Allowlist",
                trailing: vm.allowlistResponse.map { "\($0.entries.count)" }
            )
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            if isAdmin {
                addRow
                    .padding(.bottom, MSCRemoteStyle.spaceMD)
            }

            if let error = localError {
                Text(error)
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.danger)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.bottom, MSCRemoteStyle.spaceSM)
            }

            if let response = vm.allowlistResponse {
                if response.entries.isEmpty {
                    emptyState(icon: "person.crop.circle.badge.checkmark",
                               message: "Allowlist is empty. All players can join.")
                } else {
                    VStack(spacing: 0) {
                        ForEach(Array(response.entries.enumerated()), id: \.element.id) { index, entry in
                            entryRow(entry)
                            if index < response.entries.count - 1 {
                                Divider().background(MSCRemoteStyle.borderSubtle)
                            }
                        }
                    }
                }
            } else if isLoading {
                ProgressView()
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, MSCRemoteStyle.spaceLG)
            } else {
                emptyState(icon: "list.bullet.clipboard", message: "No data — pull to refresh.")
            }
        }
        .mscCard()
    }

    private var addRow: some View {
        HStack(spacing: MSCRemoteStyle.spaceSM) {
            TextField("", text: $newEntry, prompt: Text("Gamertag").foregroundColor(MSCRemoteStyle.textTertiary))
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .submitLabel(.done)
                .onSubmit { commitAdd() }
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .padding(.horizontal, MSCRemoteStyle.spaceMD)
                .frame(height: 40)
                .background(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .fill(MSCRemoteStyle.bgElevated)
                )
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1)
                )

            Button {
                commitAdd()
            } label: {
                Text(isMutating ? "…" : "Add")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(canAdd ? MSCRemoteStyle.bgBase : MSCRemoteStyle.textTertiary)
                    .frame(width: 60, height: 40)
                    .background(
                        RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                            .fill(canAdd ? MSCRemoteStyle.accent : MSCRemoteStyle.bgElevated)
                    )
            }
            .disabled(!canAdd)
        }
    }

    private var canAdd: Bool {
        isPaired && !isMutating && !trimmedNewEntry.isEmpty
    }

    private func entryRow(_ entry: AllowlistEntryDTO) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "checkmark.seal.fill")
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.success)

            VStack(alignment: .leading, spacing: 2) {
                Text(entry.name)
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                if let xuid = entry.xuid, !xuid.isEmpty {
                    Text("XUID \(xuid)")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            Spacer()
            if isAdmin {
                Button {
                    entryToRemove = entry
                } label: {
                    Image(systemName: "minus.circle.fill")
                        .font(.system(size: 18))
                        .foregroundStyle(isMutating ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.danger)
                }
                .disabled(isMutating)
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
    }

    // MARK: - Info Card

    private var infoCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                Image(systemName: "info.circle")
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                Text("How the allowlist works")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
            }
            Text("Only enforced when online-mode is enabled in server properties. Changes apply live if the server is running.")
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .mscCard()
    }

    // MARK: - Helpers

    private func emptyState(icon: String, message: String) -> some View {
        VStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: icon)
                .font(.system(size: 28, weight: .light))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Text(message)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, MSCRemoteStyle.spaceLG)
    }

    // MARK: - Actions

    private func refresh() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isLoading = true
        await vm.fetchAllowlist(baseURL: baseURL, token: token)
        isLoading = false
    }

    private func commitAdd() {
        let name = trimmedNewEntry
        guard canAdd else { return }
        Task { await performMutation(action: "add", name: name) }
    }

    private func performMutation(action: String, name: String) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        localError = nil
        isMutating = true
        let failure = await vm.mutateAllowlist(baseURL: baseURL, token: token, action: action, name: name)
        isMutating = false
        if let failure {
            localError = friendlyError(failure)
        } else {
            if action == "add" {
                newEntry = ""
                showToast("Added \(name)")
            } else {
                showToast("Removed \(name)")
            }
        }
    }

    private func friendlyError(_ raw: String) -> String {
        if raw.contains("not_bedrock") { return "The active server isn't a Bedrock server." }
        if raw.contains("no_active_server") { return "No active server selected on the host." }
        return raw
    }

    private func showToast(_ message: String) {
        withAnimation { toastMessage = message }
        Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            withAnimation { toastMessage = nil }
        }
    }
}
