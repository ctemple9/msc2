import SwiftUI

/// Converts a saved slot to another registered server's edition/format via
/// Chunker. Always operation-backed (`phase6-api.md` §3) -- there is no
/// synchronous result, so this view's own job is showing live progress,
/// letting the user cancel mid-flight, and offering a retry after a
/// failure, per P6.24's own "show progress/cancel/failure/recovery
/// states" requirement.
///
/// Scoped narrower than the full oracle picker: MSC 1's wizard can also
/// place a conversion into an *existing* slot on the target server, but
/// this agent's `/v1/worlds*` routes only ever operate on "the active
/// server" implicitly — there is no route to browse another server's
/// slots without switching this app's own active-server context first.
/// Placement here is always a fresh, named slot (`targetSlotId` left
/// `nil`); overwriting an existing target slot isn't exposed in this UI.
struct ConvertWorldView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    let sourceSlot: WorldSlotDTO
    let sourceServerType: ServerType

    @State private var targetServerId: String? = nil
    @State private var targetFormatText: String = ""
    @State private var targetName: String = ""
    @State private var conversionTask: Task<Void, Never>? = nil
    @State private var isRunning: Bool = false
    @State private var isCancelling: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }

    /// MSC 1's own target-server picker: any *other*, opposite-edition
    /// registered server (`WorldConversionWizardView`'s `s.id !=
    /// sourceServer.id && (sourceServer.isBedrock ? s.isJava :
    /// s.isBedrock)`, cited in `routes/worlds.rs`'s own P6.21 convert doc).
    private var eligibleTargetServers: [ServerDTO] {
        vm.servers.filter { $0.resolvedServerType != sourceServerType }
    }

    private var canSubmit: Bool {
        targetServerId != nil
            && !targetFormatText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !targetName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !isRunning
    }

    private var showingProgress: Bool { isRunning || vm.activeOperation != nil }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    if showingProgress {
                        progressCard
                    } else {
                        formCard
                        MSCActionButton(title: "Start Conversion", icon: "arrow.triangle.2.circlepath",
                                        style: .primary, isEnabled: canSubmit) {
                            start()
                        }
                    }
                    Spacer()
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.top, MSCRemoteStyle.spaceMD)
            }
            .navigationTitle("Convert World")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button(showingProgress ? "Close" : "Cancel") {
                        // Leaving the screen -- stop polling locally. Any
                        // already-sent server-side cancel keeps taking
                        // effect whether or not this view is still open.
                        conversionTask?.cancel()
                        dismiss()
                    }
                    .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
        .onDisappear { vm.activeOperation = nil }
    }

    private var formCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            MSCSectionHeader(title: "Convert \"\(sourceSlot.name)\"")
                .padding(.bottom, 4)

            VStack(alignment: .leading, spacing: 4) {
                Text("Target Server")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                if eligibleTargetServers.isEmpty {
                    Text("No opposite-edition server is registered on this agent yet.")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                } else {
                    Picker("", selection: $targetServerId) {
                        Text("Choose…").tag(String?.none)
                        ForEach(eligibleTargetServers) { server in
                            Text(server.name).tag(server.id as String?)
                        }
                    }
                    .labelsHidden()
                    .pickerStyle(.menu)
                    .tint(MSCRemoteStyle.accent)
                }
            }

            VStack(alignment: .leading, spacing: 4) {
                Text("Target Format")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                TextField("e.g. bedrock, java_1_21", text: $targetFormatText)
                    .textFieldStyle(.plain)
                    .font(.system(size: 13, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .autocorrectionDisabled()
                    .textInputAutocapitalization(.never)
                    .padding(MSCRemoteStyle.spaceSM)
                    .background(MSCRemoteStyle.bgBase)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM - 2, style: .continuous))
                Text("Exact Chunker format string — ask the target server's operator if you're not sure.")
                    .font(.system(size: 10))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }

            VStack(alignment: .leading, spacing: 4) {
                Text("New Slot Name")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                TextField("Converted World", text: $targetName)
                    .textFieldStyle(.plain)
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .padding(MSCRemoteStyle.spaceSM)
                    .background(MSCRemoteStyle.bgBase)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM - 2, style: .continuous))
            }
        }
        .mscCard()
    }

    private var progressCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            MSCSectionHeader(title: "Converting")

            switch vm.activeOperation?.state {
            case .succeeded:
                statusRow(icon: "checkmark.circle.fill", color: MSCRemoteStyle.success,
                          text: "Conversion complete. The new slot is ready on the target server.")
            case .failed:
                statusRow(icon: "xmark.circle.fill", color: MSCRemoteStyle.danger,
                          text: vm.activeOperation?.error?.message ?? "Conversion failed.")
                retryButton
            case .cancelled:
                statusRow(icon: "slash.circle.fill", color: MSCRemoteStyle.warning,
                          text: "Cancelled.")
                retryButton
            default:
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    ProgressView().tint(MSCRemoteStyle.accent)
                    Text(vm.activeOperation?.statusLine ?? "Starting…")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                }
                Button(isCancelling ? "Cancelling…" : "Cancel Conversion") {
                    cancelInFlight()
                }
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(isCancelling ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.danger)
                .disabled(isCancelling)
            }
        }
        .mscCard()
    }

    private func statusRow(icon: String, color: Color, text: String) -> some View {
        HStack(alignment: .top, spacing: MSCRemoteStyle.spaceSM) {
            Image(systemName: icon).foregroundStyle(color)
            Text(text)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var retryButton: some View {
        Button("Try Again") {
            isCancelling = false
            vm.activeOperation = nil
            start()
        }
        .font(.system(size: 13, weight: .semibold))
        .foregroundStyle(MSCRemoteStyle.accent)
    }

    private func start() {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken,
              let targetServerId else { return }
        let format = targetFormatText.trimmingCharacters(in: .whitespacesAndNewlines)
        let name = targetName.trimmingCharacters(in: .whitespacesAndNewlines)
        isRunning = true
        isCancelling = false
        vm.activeOperation = nil
        conversionTask = Task {
            _ = await vm.convertWorld(baseURL: baseURL, token: token,
                                      sourceSlotId: sourceSlot.id, targetServerId: targetServerId,
                                      targetFormat: format, targetName: name, targetSlotId: nil)
            isRunning = false
        }
    }

    /// Sends the server-side cancel request but deliberately does *not*
    /// stop the local polling `Task` — it needs to keep running so the
    /// next poll can observe the real terminal state the server settles
    /// on (`cancelled`, or `succeeded`/`failed` if cancellation loses the
    /// race), matching `RemoteAPIClient.pollOperationToTerminal`'s own
    /// P6.22 CLI precedent rather than freezing the UI on a stale
    /// "running" snapshot.
    private func cancelInFlight() {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken,
              let operationId = vm.activeOperation?.id else { return }
        isCancelling = true
        Task {
            _ = await vm.cancelOperation(baseURL: baseURL, token: token, operationId: operationId)
        }
    }
}
