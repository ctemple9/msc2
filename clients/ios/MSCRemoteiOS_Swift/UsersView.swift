import SwiftUI

struct UsersView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var isLoading = false
    @State private var showInviteSheet = false
    @State private var userToEdit: UserSummaryDTO? = nil
    @State private var userToRevoke: UserSummaryDTO? = nil
    @State private var toastMessage: String? = nil
    @State private var newTokenResult: UserCreateResultDTO? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            ScrollView(showsIndicators: false) {
                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    usersCard
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
                .allowsHitTesting(false)
            }
        }
        .navigationTitle("Users & Access")
        .navigationBarTitleDisplayMode(.large)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button { showInviteSheet = true } label: {
                    Image(systemName: "person.badge.plus")
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
        .task { await refresh() }
        .sheet(isPresented: $showInviteSheet, onDismiss: { Task { await refresh() } }) {
            InviteUserSheet(onCreated: { result in
                newTokenResult = result
                showInviteSheet = false
            })
            .environmentObject(settings)
            .environmentObject(vm)
        }
        .sheet(item: $userToEdit, onDismiss: { Task { await refresh() } }) { user in
            EditUserSheet(user: user)
                .environmentObject(settings)
                .environmentObject(vm)
        }
        .sheet(item: $newTokenResult) { result in
            NewTokenDisplaySheet(result: result)
        }
        .confirmationDialog(
            "Revoke \"\(userToRevoke?.label ?? "")\"?",
            isPresented: Binding(get: { userToRevoke != nil }, set: { if !$0 { userToRevoke = nil } }),
            titleVisibility: .visible
        ) {
            Button("Revoke Access", role: .destructive) {
                guard let user = userToRevoke else { return }
                Task { await doRevoke(user: user) }
            }
            Button("Cancel", role: .cancel) { userToRevoke = nil }
        } message: {
            Text("This token will be invalidated immediately. This cannot be undone.")
        }
    }

    // MARK: - Cards

    private var usersCard: some View {
        VStack(spacing: 0) {
            MSCSectionHeader(title: "Shared Access Tokens")
                .padding(.bottom, MSCRemoteStyle.spaceSM)

            VStack(spacing: 0) {
                if isLoading && vm.usersResponse == nil {
                    HStack {
                        Spacer()
                        ProgressView()
                        Spacer()
                    }
                    .padding(MSCRemoteStyle.spaceLG)
                } else if let users = vm.usersResponse?.users, !users.isEmpty {
                    ForEach(Array(users.enumerated()), id: \.element.id) { idx, user in
                        userRow(user)
                        if idx < users.count - 1 {
                            Divider()
                                .padding(.leading, MSCRemoteStyle.spaceLG)
                        }
                    }
                } else {
                    Text("No shared access tokens yet.\nTap + to invite someone.")
                        .font(.system(size: 14))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                        .multilineTextAlignment(.center)
                        .padding(MSCRemoteStyle.spaceLG)
                }
            }
            .mscCard()
        }
    }

    private func userRow(_ user: UserSummaryDTO) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: roleIcon(user.role))
                .foregroundStyle(user.isExpired ? MSCRemoteStyle.textTertiary : roleColor(user.role))
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text(user.label)
                        .font(.system(size: 15, weight: .medium))
                        .foregroundStyle(user.isExpired ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.textPrimary)
                    if user.isExpired {
                        Text("EXPIRED")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundStyle(MSCRemoteStyle.danger)
                            .padding(.horizontal, 5)
                            .padding(.vertical, 2)
                            .background(MSCRemoteStyle.danger.opacity(0.15))
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                    }
                }
                Text(roleDescription(user))
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            Menu {
                Button { userToEdit = user } label: {
                    Label("Edit", systemImage: "pencil")
                }
                Button(role: .destructive) { userToRevoke = user } label: {
                    Label("Revoke", systemImage: "trash")
                }
            } label: {
                Image(systemName: "ellipsis.circle")
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                    .frame(width: 32, height: 32)
            }
        }
        .padding(.horizontal, MSCRemoteStyle.spaceLG)
        .padding(.vertical, MSCRemoteStyle.spaceMD)
    }

    private var infoCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            MSCSectionHeader(title: "About Access Tokens")
                .padding(.bottom, MSCRemoteStyle.spaceXS)

            VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceXS) {
                infoRow(icon: "person.badge.shield.checkmark", text: "Admin tokens can perform any action, including managing other users.")
                infoRow(icon: "eye", text: "Guest tokens can view status but cannot change anything.")
                infoRow(icon: "slider.horizontal.3", text: "Named tokens have custom permission categories — ideal for automation or limited access.")
                infoRow(icon: "exclamationmark.triangle", text: "Token strings are shown once at creation. Store them securely — they cannot be recovered.")
            }
            .padding(MSCRemoteStyle.spaceLG)
            .mscCard()
        }
    }

    private func infoRow(icon: String, text: String) -> some View {
        HStack(alignment: .top, spacing: MSCRemoteStyle.spaceSM) {
            Image(systemName: icon)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.accent)
                .frame(width: 20)
            Text(text)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    // MARK: - Helpers

    private func refresh() async {
        guard let url = resolvedBaseURL, let tok = resolvedToken else { return }
        isLoading = true
        await vm.fetchUsers(baseURL: url, token: tok)
        isLoading = false
    }

    private func doRevoke(user: UserSummaryDTO) async {
        guard let url = resolvedBaseURL, let tok = resolvedToken else { return }
        let ok = await vm.revokeUser(baseURL: url, token: tok, userId: user.id)
        userToRevoke = nil
        if ok {
            await vm.fetchUsers(baseURL: url, token: tok)
            showToast("\"\(user.label)\" revoked.")
        } else {
            showToast("Failed to revoke access.")
        }
    }

    private func showToast(_ msg: String) {
        withAnimation { toastMessage = msg }
        Task {
            try? await Task.sleep(nanoseconds: 2_500_000_000)
            withAnimation { toastMessage = nil }
        }
    }

    private func roleIcon(_ role: String) -> String {
        switch role {
        case "admin": return "person.badge.shield.checkmark.fill"
        case "guest": return "eye.fill"
        default:      return "slider.horizontal.3"
        }
    }

    private func roleColor(_ role: String) -> Color {
        switch role {
        case "admin": return MSCRemoteStyle.accent
        case "guest": return MSCRemoteStyle.warning
        default:      return MSCRemoteStyle.accent
        }
    }

    private func roleDescription(_ user: UserSummaryDTO) -> String {
        if let perms = user.permissions, !perms.isEmpty {
            return perms.joined(separator: ", ")
        }
        return user.role.capitalized
    }
}

// MARK: - InviteUserSheet

private struct InviteUserSheet: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    var onCreated: (UserCreateResultDTO) -> Void

    @Environment(\.dismiss) private var dismiss

    @State private var label = ""
    @State private var selectedPreset = AccessPreset.admin
    @State private var expiryDays: Int? = nil
    @State private var isSaving = false
    @State private var errorMsg: String? = nil

    enum AccessPreset: String, CaseIterable, Identifiable {
        case admin   = "Admin"
        case guest   = "View only"
        case control = "Server control"
        case worlds  = "Worlds"
        case custom  = "Custom"

        var id: String { rawValue }

        var role: String {
            switch self {
            case .admin:   return "admin"
            case .guest:   return "guest"
            default:       return "named"
            }
        }

        var permissions: [String]? {
            switch self {
            case .admin:   return nil
            case .guest:   return nil
            case .control: return ["serverControl"]
            case .worlds:  return ["worlds"]
            case .custom:  return []
            }
        }

        var description: String {
            switch self {
            case .admin:   return "Full access to everything"
            case .guest:   return "Read-only — status and console"
            case .control: return "Start, stop, and run commands"
            case .worlds:  return "Manage world slots and backups"
            case .custom:  return "Choose specific permissions below"
            }
        }
    }

    static let allPermissions = ["serverControl", "players", "settings", "addons", "worlds", "broadcast", "networking", "fleet"]
    @State private var customPermissions: Set<String> = []

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        labelCard
                        presetCard
                        if selectedPreset == .custom { permissionsCard }
                        expiryCard
                        if let err = errorMsg {
                            Text(err)
                                .font(.system(size: 13))
                                .foregroundStyle(MSCRemoteStyle.danger)
                                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        }
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                    .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                    .frame(maxWidth: .infinity)
                }
            }
            .navigationTitle("Invite Person")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    if isSaving {
                        ProgressView()
                    } else {
                        Button("Create") { Task { await doCreate() } }
                            .foregroundStyle(MSCRemoteStyle.accent)
                            .disabled(label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    }
                }
            }
        }
    }

    private var labelCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            MSCSectionHeader(title: "Label")
            VStack(spacing: 0) {
                TextField("e.g. \"GitHub Actions\" or \"Friend's phone\"", text: $label)
                    .font(.system(size: 15))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .padding(MSCRemoteStyle.spaceLG)
            }
            .mscCard()
        }
    }

    private var presetCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            MSCSectionHeader(title: "Access Level")
            VStack(spacing: 0) {
                ForEach(Array(AccessPreset.allCases.enumerated()), id: \.element.id) { idx, preset in
                    Button {
                        selectedPreset = preset
                        if preset != .custom { customPermissions = [] }
                    } label: {
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(preset.rawValue)
                                    .font(.system(size: 15, weight: .medium))
                                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                                Text(preset.description)
                                    .font(.system(size: 12))
                                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            if selectedPreset == preset {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundStyle(MSCRemoteStyle.accent)
                            }
                        }
                        .padding(MSCRemoteStyle.spaceLG)
                    }
                    if idx < AccessPreset.allCases.count - 1 {
                        Divider().padding(.leading, MSCRemoteStyle.spaceLG)
                    }
                }
            }
            .mscCard()
        }
    }

    private var permissionsCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            MSCSectionHeader(title: "Permissions")
            VStack(spacing: 0) {
                ForEach(Array(Self.allPermissions.enumerated()), id: \.element) { idx, perm in
                    Button {
                        if customPermissions.contains(perm) { customPermissions.remove(perm) }
                        else { customPermissions.insert(perm) }
                    } label: {
                        HStack {
                            Text(permissionLabel(perm))
                                .font(.system(size: 15))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            if customPermissions.contains(perm) {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundStyle(MSCRemoteStyle.accent)
                            } else {
                                Image(systemName: "circle")
                                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                            }
                        }
                        .padding(MSCRemoteStyle.spaceLG)
                    }
                    if idx < Self.allPermissions.count - 1 {
                        Divider().padding(.leading, MSCRemoteStyle.spaceLG)
                    }
                }
            }
            .mscCard()
        }
    }

    private struct ExpiryOption: Identifiable {
        let days: Int?
        let label: String
        var id: String { label }
    }
    private static let expiryOptions: [ExpiryOption] = [
        .init(days: nil, label: "Never"),
        .init(days: 7,   label: "7 days"),
        .init(days: 30,  label: "30 days"),
        .init(days: 90,  label: "90 days")
    ]

    private var expiryCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            MSCSectionHeader(title: "Expiry")
            VStack(spacing: 0) {
                ForEach(Array(Self.expiryOptions.enumerated()), id: \.element.id) { idx, opt in
                    Button { expiryDays = opt.days } label: {
                        HStack {
                            Text(opt.label)
                                .font(.system(size: 15))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            if expiryDays == opt.days {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundStyle(MSCRemoteStyle.accent)
                            }
                        }
                        .padding(MSCRemoteStyle.spaceLG)
                    }
                    if idx < Self.expiryOptions.count - 1 {
                        Divider().padding(.leading, MSCRemoteStyle.spaceLG)
                    }
                }
            }
            .mscCard()
        }
    }

    private func doCreate() async {
        let trimmed = label.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        guard let url = settings.resolvedBaseURL(), let tok = settings.resolvedToken() else { return }

        isSaving = true
        errorMsg = nil

        let perms: [String]? = selectedPreset == .custom ? Array(customPermissions) : selectedPreset.permissions

        let result = await vm.createUser(
            baseURL: url, token: tok,
            label: trimmed,
            role: selectedPreset.role,
            permissions: perms,
            expiresInDays: expiryDays
        )

        isSaving = false

        if let r = result, r.success {
            onCreated(r)
        } else {
            errorMsg = result?.message ?? "Failed to create token."
        }
    }

    private func permissionLabel(_ key: String) -> String {
        switch key {
        case "serverControl": return "Server control"
        case "players":       return "Players & allowlist"
        case "settings":      return "Settings & config"
        case "addons":        return "Mods & plugins"
        case "worlds":        return "Worlds & backups"
        case "broadcast":     return "Xbox broadcast"
        case "networking":    return "Networking (playit)"
        case "fleet":         return "Server fleet"
        default:              return key
        }
    }
}

// MARK: - EditUserSheet

private struct EditUserSheet: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    let user: UserSummaryDTO
    @Environment(\.dismiss) private var dismiss

    @State private var label: String
    @State private var isSaving = false
    @State private var errorMsg: String? = nil

    init(user: UserSummaryDTO) {
        self.user = user
        _label = State(initialValue: user.label)
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
                        MSCSectionHeader(title: "Label")
                        VStack(spacing: 0) {
                            TextField("Label", text: $label)
                                .font(.system(size: 15))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .padding(MSCRemoteStyle.spaceLG)
                        }
                        .mscCard()
                    }
                    if let err = errorMsg {
                        Text(err)
                            .font(.system(size: 13))
                            .foregroundStyle(MSCRemoteStyle.danger)
                            .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    }
                    Spacer()
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.top, MSCRemoteStyle.spaceMD)
                .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                .frame(maxWidth: .infinity)
            }
            .navigationTitle("Edit Access")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    if isSaving {
                        ProgressView()
                    } else {
                        Button("Save") { Task { await doSave() } }
                            .foregroundStyle(MSCRemoteStyle.accent)
                            .disabled(label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    }
                }
            }
        }
    }

    private func doSave() async {
        guard let url = settings.resolvedBaseURL(), let tok = settings.resolvedToken() else { return }
        isSaving = true
        errorMsg = nil
        let result = await vm.updateUser(
            baseURL: url, token: tok,
            userId: user.id,
            label: label.trimmingCharacters(in: .whitespacesAndNewlines),
            role: nil, permissions: nil, expiresInDays: nil
        )
        isSaving = false
        if result?.success == true {
            dismiss()
        } else {
            errorMsg = result?.message ?? "Failed to save."
        }
    }
}

// MARK: - NewTokenDisplaySheet

private struct NewTokenDisplaySheet: View {
    let result: UserCreateResultDTO
    @Environment(\.dismiss) private var dismiss

    @State private var didCopy = false

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    VStack(spacing: MSCRemoteStyle.spaceMD) {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.system(size: 48))
                            .foregroundStyle(MSCRemoteStyle.success)
                        Text("Token Created")
                            .font(.system(size: 20, weight: .bold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                        Text("Copy this token now. It will not be shown again.")
                            .font(.system(size: 14))
                            .foregroundStyle(MSCRemoteStyle.warning)
                            .multilineTextAlignment(.center)
                    }
                    .padding(.top, MSCRemoteStyle.spaceLG)

                    if let token = result.token {
                        VStack(spacing: MSCRemoteStyle.spaceSM) {
                            Text(token)
                                .font(.system(size: 12, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .padding(MSCRemoteStyle.spaceLG)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .mscCard()
                                .onTapGesture { copyToken(token) }

                            Button { copyToken(token) } label: {
                                Label(didCopy ? "Copied!" : "Copy Token", systemImage: didCopy ? "checkmark" : "doc.on.doc")
                                    .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(didCopy ? MSCRemoteStyle.success : MSCRemoteStyle.accent)
                        }
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    }

                    Spacer()
                }
                .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                .frame(maxWidth: .infinity)
            }
            .navigationTitle(result.user?.label ?? "New Token")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
    }

    private func copyToken(_ token: String) {
        UIPasteboard.general.string = token
        UIImpactFeedbackGenerator(style: .medium).impactOccurred()
        withAnimation { didCopy = true }
        Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            withAnimation { didCopy = false }
        }
    }
}
