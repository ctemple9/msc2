import SwiftUI

// MARK: - ServerSettingsView
//
// A schema-driven form. The macOS server sends a typed settings schema
// (sections → fields, each carrying its control type + constraints); this
// screen renders the right control per field and validates locally, then
// POSTs only the changed keys. Because nothing about the field set is
// hardcoded, the same screen serves Java, Bedrock (P4), and future config
// surfaces — the server owns the schema.
//
// Pushed from the Dashboard via NavigationLink, so it does NOT wrap its own
// NavigationStack (mirrors AllowlistView).

struct ServerSettingsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    /// Local editable copy: key → string value. Seeded from the schema.
    @State private var draft: [String: String] = [:]
    /// The values as last loaded/saved — the diff baseline.
    @State private var original: [String: String] = [:]

    @State private var isSaving = false
    @State private var isLoading = false
    @State private var resultBanner: String? = nil
    @State private var resultIsError = false
    @State private var rejections: [SettingRejectionDTO] = []

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }
    private var canEdit: Bool { vm.connectedRole != "guest" }

    private var response: SettingsResponseDTO? { vm.settingsResponse }
    private var sections: [SettingsSectionDTO] { response?.sections ?? [] }
    private var allFields: [SettingFieldDTO] { sections.flatMap { $0.fields } }

    /// Only the keys whose value differs from the loaded baseline.
    private var changes: [String: String] {
        var out: [String: String] = [:]
        for (k, v) in draft where original[k] != v { out[k] = v }
        return out
    }

    private var hasValidationErrors: Bool {
        allFields.contains { validationError($0) != nil }
    }

    private var canSave: Bool {
        canEdit && !changes.isEmpty && !hasValidationErrors && !isSaving
    }

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            ScrollView(showsIndicators: false) {
                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    if let response, !response.editable {
                        unavailableCard(note: response.note)
                    } else if response == nil {
                        loadingCard
                    } else {
                        if response?.serverRunning == true { restartHintBanner }
                        if !canEdit { readOnlyBanner }
                        if let resultBanner { resultBannerView(resultBanner) }
                        if !rejections.isEmpty { rejectionsCard }

                        ForEach(sections) { section in
                            sectionCard(section)
                        }
                    }
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                .frame(maxWidth: .infinity)
                .padding(.top, MSCRemoteStyle.spaceMD)
                .padding(.bottom, MSCRemoteStyle.space2XL)
            }
            .refreshable { await load(reseed: true) }
        }
        .navigationTitle("Server Settings")
        .navigationBarTitleDisplayMode(.inline)
        .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
        .toolbarColorScheme(.dark, for: .navigationBar)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                if canEdit && response?.editable == true {
                    Button {
                        Task { await save() }
                    } label: {
                        if isSaving {
                            ProgressView().tint(MSCRemoteStyle.accent)
                        } else {
                            Text("Save").fontWeight(.semibold)
                        }
                    }
                    .disabled(!canSave)
                    .foregroundStyle(canSave ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                }
            }
        }
        .task(id: isPaired) {
            guard isPaired else { return }
            await load(reseed: false)
        }
    }

    // MARK: - Load / Save

    private func load(reseed: Bool) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isLoading = true
        await vm.fetchSettings(baseURL: baseURL, token: token)
        isLoading = false
        // Seed local state when we have no baseline yet, on an explicit refresh,
        // or whenever there are no pending edits (safe to adopt fresh values).
        if reseed || original.isEmpty || changes.isEmpty {
            seed(from: sections)
        }
    }

    private func save() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken, !changes.isEmpty else { return }
        hapticLight()
        isSaving = true
        let result = await vm.updateSettings(baseURL: baseURL, token: token, changes: changes)
        isSaving = false

        guard let result else {
            resultIsError = true
            resultBanner = "Couldn't reach the server."
            hapticError()
            return
        }

        rejections = result.rejected ?? []
        if let fresh = result.sections { seed(from: fresh) }

        if result.success {
            hapticSuccess()
            resultIsError = false
            var msg = "Saved \(result.appliedKeys.count) change\(result.appliedKeys.count == 1 ? "" : "s")."
            if result.restartRequired { msg += " Restart the server to apply." }
            if !rejections.isEmpty { msg += " \(rejections.count) not applied." }
            resultBanner = msg
        } else {
            hapticError()
            resultIsError = true
            resultBanner = friendlyMessage(result.message)
        }
    }

    private func seed(from sections: [SettingsSectionDTO]) {
        var d: [String: String] = [:]
        for s in sections { for f in s.fields { d[f.key] = f.value } }
        draft = d
        original = d
    }

    // MARK: - Section & field rendering

    private func sectionCard(_ section: SettingsSectionDTO) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Image(systemName: section.icon)
                    .font(.system(size: 14))
                    .foregroundStyle(MSCRemoteStyle.accent)
                MSCSectionHeader(title: section.title)
            }
            .padding(.bottom, MSCRemoteStyle.spaceSM)

            VStack(spacing: 0) {
                ForEach(Array(section.fields.enumerated()), id: \.element.id) { idx, field in
                    fieldRow(field)
                    if idx < section.fields.count - 1 {
                        Divider().background(MSCRemoteStyle.borderSubtle)
                    }
                }
            }
        }
        .mscCard()
    }

    @ViewBuilder
    private func fieldRow(_ field: SettingFieldDTO) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .center, spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(field.label)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    if let help = field.help {
                        Text(help)
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                Spacer(minLength: MSCRemoteStyle.spaceMD)
                control(for: field)
            }

            if let err = validationError(field) {
                Text(err)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.danger)
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 1)
    }

    @ViewBuilder
    private func control(for field: SettingFieldDTO) -> some View {
        switch field.type {
        case "bool":
            Toggle("", isOn: boolBinding(field.key))
                .labelsHidden()
                .tint(MSCRemoteStyle.accent)
                .disabled(!canEdit)

        case "enum":
            Picker("", selection: stringBinding(field.key)) {
                ForEach(field.options ?? []) { opt in
                    Text(opt.label).tag(opt.value)
                }
            }
            .pickerStyle(.menu)
            .tint(MSCRemoteStyle.accent)
            .disabled(!canEdit)

        case "int":
            HStack(spacing: 4) {
                TextField("", text: stringBinding(field.key))
                    .keyboardType(.numbersAndPunctuation)
                    .multilineTextAlignment(.trailing)
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .frame(width: 68)
                    .textFieldStyle(.plain)
                    .disabled(!canEdit)
                if let unit = field.unit {
                    Text(unit)
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 5)
            .background(MSCRemoteStyle.bgDeep)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(validationError(field) == nil ? MSCRemoteStyle.borderMid : MSCRemoteStyle.danger, lineWidth: 1)
            )

        default: // "string"
            TextField("", text: stringBinding(field.key, maxLength: field.maxLength))
                .multilineTextAlignment(.trailing)
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .frame(maxWidth: 180)
                .textFieldStyle(.plain)
                .padding(.horizontal, 8)
                .padding(.vertical, 5)
                .background(MSCRemoteStyle.bgDeep)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                )
                .disabled(!canEdit)
        }
    }

    // MARK: - Bindings

    private func stringBinding(_ key: String, maxLength: Int? = nil) -> Binding<String> {
        Binding(
            get: { draft[key] ?? "" },
            set: { newValue in
                if let maxLength, newValue.count > maxLength {
                    draft[key] = String(newValue.prefix(maxLength))
                } else {
                    draft[key] = newValue
                }
            }
        )
    }

    private func boolBinding(_ key: String) -> Binding<Bool> {
        Binding(
            get: { (draft[key] ?? "false") == "true" },
            set: { draft[key] = $0 ? "true" : "false" }
        )
    }

    // MARK: - Validation

    private func validationError(_ field: SettingFieldDTO) -> String? {
        guard field.type == "int" else { return nil }
        let raw = (draft[field.key] ?? "").trimmingCharacters(in: .whitespaces)
        guard let v = Int(raw) else { return "Enter a whole number" }
        if let lo = field.minInt, v < lo { return "Must be at least \(lo)" }
        if let hi = field.maxInt, v > hi { return "Must be at most \(hi)" }
        return nil
    }

    private func friendlyMessage(_ raw: String) -> String {
        switch raw {
        case "no_active_server": return "No active server selected on the Mac."
        case "not_supported":    return "This server type isn't supported yet."
        case "no_valid_changes": return "No valid changes to apply."
        default:                 return raw
        }
    }

    // MARK: - Banners & cards

    private var restartHintBanner: some View {
        HStack(spacing: 8) {
            Image(systemName: "exclamationmark.arrow.circlepath")
                .font(.system(size: 13))
                .foregroundStyle(Color.orange)
            Text("Server is running — changes apply after the next restart.")
                .font(.system(size: 12))
                .foregroundStyle(Color.orange)
            Spacer(minLength: 0)
        }
        .mscCard(padding: MSCRemoteStyle.spaceMD)
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
                .strokeBorder(Color.orange.opacity(0.3), lineWidth: 1)
        )
    }

    private var readOnlyBanner: some View {
        HStack(spacing: 8) {
            Image(systemName: "lock.fill")
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Text("Read-only — connect with an admin token to edit.")
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Spacer(minLength: 0)
        }
        .mscCard(padding: MSCRemoteStyle.spaceMD)
    }

    private func resultBannerView(_ text: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: resultIsError ? "xmark.octagon.fill" : "checkmark.circle.fill")
                .font(.system(size: 13))
                .foregroundStyle(resultIsError ? MSCRemoteStyle.danger : MSCRemoteStyle.success)
            Text(text)
                .font(.system(size: 12))
                .foregroundStyle(resultIsError ? MSCRemoteStyle.danger : MSCRemoteStyle.success)
            Spacer(minLength: 0)
        }
        .mscCard(padding: MSCRemoteStyle.spaceMD)
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
                .strokeBorder((resultIsError ? MSCRemoteStyle.danger : MSCRemoteStyle.success).opacity(0.3), lineWidth: 1)
        )
    }

    private var rejectionsCard: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Not applied")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.danger)
            ForEach(rejections) { r in
                Text("• \(r.key): \(r.reason)")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .mscCard(padding: MSCRemoteStyle.spaceMD)
    }

    private var loadingCard: some View {
        HStack(spacing: 8) {
            ProgressView().scaleEffect(0.8)
            Text("Loading settings…")
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.vertical, MSCRemoteStyle.spaceLG)
        .mscCard()
    }

    private func unavailableCard(note: String?) -> some View {
        VStack(spacing: MSCRemoteStyle.spaceSM) {
            Image(systemName: "slider.horizontal.3")
                .font(.system(size: 26))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            Text(unavailableText(note: note))
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, MSCRemoteStyle.spaceLG)
        .mscCard()
    }

    private func unavailableText(note: String?) -> String {
        switch note {
        case "no_active_server": return "No active server is selected on your Mac. Pick one on the Dashboard first."
        case "bedrock_not_supported_yet": return "Editable settings for Bedrock servers are coming in a later update."
        default: return "Settings aren't available for this server."
        }
    }
}
