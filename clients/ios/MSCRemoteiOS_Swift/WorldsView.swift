import SwiftUI

struct WorldsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var isLoading: Bool = false
    @State private var isBackingUp: Bool = false
    @State private var isActivating: Bool = false
    @State private var isRestoring: Bool = false

    @State private var slotToActivate: WorldSlotDTO? = nil
    @State private var backupToRestore: BackupItemDTO? = nil
    @State private var toastMessage: String? = nil

    // World management (P9)
    @Binding var showCreateSheet: Bool
    @State private var slotToRename: WorldSlotDTO? = nil
    @State private var renameText: String = ""
    @State private var slotToReplace: WorldSlotDTO? = nil     // destination; source picked in a dialog
    @State private var slotToRepair: WorldSlotDTO? = nil
    @State private var isMutating: Bool = false

    // Backup schedule card draft state
    @State private var scheduleEnabled: Bool = false
    @State private var scheduleInterval: Int = 30
    @State private var scheduleMaxCount: Int = 12
    @State private var scheduleOriginalEnabled: Bool = false
    @State private var scheduleOriginalInterval: Int = 30
    @State private var scheduleOriginalMaxCount: Int = 12
    @State private var isSavingSchedule: Bool = false
    @State private var scheduleSaveMessage: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }
    private var isAdmin: Bool { vm.connectedRole == "admin" }

    private var activeServerType: ServerType {
        if let fromServers = vm.servers.first(where: { $0.id == (vm.status?.activeServerId ?? "") })?.resolvedServerType {
            return fromServers
        }
        return vm.status?.resolvedServerType ?? .java
    }

    private var isRepairing: Bool { vm.worldsResponse?.isRepairing == true }

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            ScrollView(showsIndicators: false) {
                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    worldSlotsCard
                    backupsCard
                    backupScheduleCard
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
        .task(id: isPaired) {
            guard isPaired else { return }
            await refresh()
        }
        // Each presentation is isolated onto its own hidden anchor view. SwiftUI reliably
        // supports only ~one presentation modifier per view; stacking a sheet + several
        // alerts/dialogs on the same node triggers an AttributeGraph re-evaluation loop that
        // freezes the main thread (and eventually crashes). One-per-anchor avoids that.
        .background(createSheetAnchor)
        .background(renameAlertAnchor)
        .background(replaceDialogAnchor)
        .background(repairAlertAnchor)
        .background(activateAlertAnchor)
        .background(restoreAlertAnchor)
    }

    // MARK: - Isolated presentation anchors

    private var createSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showCreateSheet) {
                CreateWorldView { name, seed in
                    Task { await performCreate(name: name, seed: seed) }
                }
            }
    }

    private var renameAlertAnchor: some View {
        Color.clear
            .alert("Rename World", isPresented: Binding(
                get: { slotToRename != nil },
                set: { if !$0 { slotToRename = nil } }
            )) {
                TextField("World name", text: $renameText)
                Button("Cancel", role: .cancel) { slotToRename = nil }
                Button("Rename") {
                    guard let slot = slotToRename else { return }
                    let newName = renameText
                    slotToRename = nil
                    Task { await performRename(slot: slot, newName: newName) }
                }
            } message: {
                Text("Enter a new name for this world slot.")
            }
    }

    private var replaceDialogAnchor: some View {
        Color.clear
            .confirmationDialog(
                "Replace with…",
                isPresented: Binding(
                    get: { slotToReplace != nil },
                    set: { if !$0 { slotToReplace = nil } }
                ),
                titleVisibility: .visible
            ) {
                if let dest = slotToReplace, let response = vm.worldsResponse {
                    ForEach(response.slots.filter { $0.id != dest.id }) { source in
                        Button("Copy \"\(source.name)\" → \"\(dest.name)\"", role: .destructive) {
                            slotToReplace = nil
                            Task { await performReplace(dest: dest, source: source) }
                        }
                    }
                }
                Button("Cancel", role: .cancel) { slotToReplace = nil }
            } message: {
                if let dest = slotToReplace {
                    Text("Overwrite the saved world in \"\(dest.name)\" with a copy of another slot's world. This cannot be undone.")
                }
            }
    }

    private var repairAlertAnchor: some View {
        Color.clear
            .alert("Repair World", isPresented: Binding(
                get: { slotToRepair != nil },
                set: { if !$0 { slotToRepair = nil } }
            )) {
                Button("Cancel", role: .cancel) { slotToRepair = nil }
                Button("Repair") {
                    guard let slot = slotToRepair else { return }
                    slotToRepair = nil
                    Task { await performRepair(slot: slot) }
                }
            } message: {
                Text("Regenerate this Bedrock world's level.dat to fix version-mismatch (\"Silverfish\") join errors.\n\nThe world data is preserved and a backup is taken first. The server will start briefly and then stop — this can take a couple of minutes.")
            }
    }

    private var activateAlertAnchor: some View {
        Color.clear
            .alert("Switch World", isPresented: Binding(
                get: { slotToActivate != nil },
                set: { if !$0 { slotToActivate = nil } }
            )) {
                Button("Cancel", role: .cancel) { slotToActivate = nil }
                Button("Switch") {
                    guard let slot = slotToActivate else { return }
                    slotToActivate = nil
                    Task { await performActivate(slot: slot) }
                }
            } message: {
                if let slot = slotToActivate {
                    Text("Switch to \"\(slot.name)\"?\n\nThe current world will be saved to its slot first. The server must be stopped.")
                }
            }
    }

    private var restoreAlertAnchor: some View {
        Color.clear
            .alert("Restore Backup", isPresented: Binding(
                get: { backupToRestore != nil },
                set: { if !$0 { backupToRestore = nil } }
            )) {
                Button("Cancel", role: .cancel) { backupToRestore = nil }
                Button("Restore", role: .destructive) {
                    guard let backup = backupToRestore else { return }
                    backupToRestore = nil
                    Task { await performRestore(backup: backup) }
                }
            } message: {
                if let backup = backupToRestore {
                    Text("Restore \"\(backup.displayName)\"?\n\nA safety backup of the current world will be taken first.")
                }
            }
    }

    // MARK: - World Slots Card

    private var worldSlotsCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "World Slots")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if let error = vm.errorMessage, vm.worldsResponse == nil {
                Text("Error: \(error)")
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.danger)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.bottom, MSCRemoteStyle.spaceSM)
            }

            if let response = vm.worldsResponse {
                if isRepairing {
                    repairingBanner
                        .padding(.bottom, MSCRemoteStyle.spaceMD)
                } else if response.serverRunning && !response.slots.isEmpty {
                    serverRunningBanner
                        .padding(.bottom, MSCRemoteStyle.spaceMD)
                }

                if response.slots.isEmpty {
                    emptyState(icon: "square.2.layers.3d", message: "No world slots found for the active server.")
                } else {
                    VStack(spacing: 0) {
                        ForEach(Array(response.slots.enumerated()), id: \.element.id) { index, slot in
                            slotRow(slot, serverRunning: response.serverRunning)
                            if index < response.slots.count - 1 {
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
                emptyState(icon: "globe", message: "No data — pull to refresh.")
            }
        }
        .mscCard()
    }

    private func slotRow(_ slot: WorldSlotDTO, serverRunning: Bool) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    Text(slot.name)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    if slot.isActive {
                        Text("Active")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundStyle(MSCRemoteStyle.success)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Capsule().fill(MSCRemoteStyle.success.opacity(0.12)))
                    }
                }
                HStack(spacing: 6) {
                    if let seed = slot.worldSeed, !seed.isEmpty {
                        Text("Seed: \(seed)")
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                        Text("·")
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .font(.system(size: 11))
                    }
                    if let bytes = slot.zipSizeBytes {
                        Text(formatBytes(bytes))
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                        Text("·")
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .font(.system(size: 11))
                    }
                    Text(shortDate(slot.createdAt))
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            Spacer()
            if isAdmin && !slot.isActive {
                Button {
                    slotToActivate = slot
                } label: {
                    Text("Set Active")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(serverRunning ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.accent)
                        .padding(.horizontal, MSCRemoteStyle.spaceMD)
                        .padding(.vertical, MSCRemoteStyle.spaceSM)
                        .background(
                            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                                .fill(serverRunning ? MSCRemoteStyle.bgElevated : MSCRemoteStyle.accentDim)
                        )
                }
                .disabled(serverRunning || isActivating)
            }
            if isAdmin {
                slotMenu(slot, serverRunning: serverRunning)
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
    }

    /// Per-slot admin action menu: rename / replace / repair.
    private func slotMenu(_ slot: WorldSlotDTO, serverRunning: Bool) -> some View {
        Menu {
            Button {
                renameText = slot.name
                slotToRename = slot
            } label: {
                Label("Rename", systemImage: "pencil")
            }

            // Replace requires at least one OTHER slot to copy from.
            if (vm.worldsResponse?.slots.count ?? 0) > 1 {
                Button(role: .destructive) {
                    slotToReplace = slot
                } label: {
                    Label("Replace With…", systemImage: "arrow.triangle.2.circlepath")
                }
            }

            // Repair only applies to the active world of a Bedrock server, while stopped.
            if activeServerType == .bedrock && slot.isActive {
                Button {
                    slotToRepair = slot
                } label: {
                    Label("Repair World", systemImage: "wrench.and.screwdriver")
                }
                .disabled(serverRunning || isRepairing)
            }
        } label: {
            Image(systemName: "ellipsis.circle")
                .font(.system(size: 18))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .padding(.leading, MSCRemoteStyle.spaceSM)
        }
        .disabled(isMutating || isRepairing)
    }

    private var repairingBanner: some View {
        HStack(spacing: MSCRemoteStyle.spaceSM) {
            ProgressView()
                .controlSize(.small)
                .tint(MSCRemoteStyle.warning)
            Text("Repairing world… the server will restart briefly. This can take a couple of minutes.")
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.warning)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var serverRunningBanner: some View {
        HStack(spacing: MSCRemoteStyle.spaceSM) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.warning)
            Text("Stop the server before switching worlds.")
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.warning)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Backups Card

    private var backupsCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Backups")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if isAdmin {
                MSCActionButton(
                    title: isBackingUp ? "Backing Up…" : "Back Up Now",
                    icon: "arrow.down.doc.fill",
                    style: .primary,
                    isEnabled: isPaired && !isBackingUp
                ) {
                    Task { await performBackupNow() }
                }
                .padding(.bottom, MSCRemoteStyle.spaceMD)
            }

            if let response = vm.backupsResponse {
                if response.backups.isEmpty {
                    emptyState(icon: "archivebox", message: "No backups yet for the active server.")
                } else {
                    VStack(spacing: 0) {
                        ForEach(Array(response.backups.enumerated()), id: \.element.id) { index, backup in
                            backupRow(backup)
                            if index < response.backups.count - 1 {
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
                emptyState(icon: "archivebox", message: "No data — pull to refresh.")
            }
        }
        .mscCard()
    }

    private func backupRow(_ backup: BackupItemDTO) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Text(backup.displayName)
                        .font(.system(size: 13, weight: .medium, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text(backup.isAutomatic ? "Auto" : "Manual")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(backup.isAutomatic ? .blue : MSCRemoteStyle.textSecondary)
                        .padding(.horizontal, 5)
                        .padding(.vertical, 2)
                        .background(
                            RoundedRectangle(cornerRadius: 4, style: .continuous)
                                .fill(backup.isAutomatic ? Color.blue.opacity(0.12) : MSCRemoteStyle.bgElevated)
                        )
                }
                HStack(spacing: 6) {
                    if let size = backup.fileSize {
                        Text(formatBytes(size))
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                    if let slotName = backup.slotName {
                        Text("·")
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .font(.system(size: 11))
                        Text(slotName)
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
            }
            Spacer()
            if isAdmin {
                Button {
                    backupToRestore = backup
                } label: {
                    Text("Restore")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(isRestoring ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.accent)
                        .padding(.horizontal, MSCRemoteStyle.spaceMD)
                        .padding(.vertical, MSCRemoteStyle.spaceSM)
                        .background(
                            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                                .fill(isRestoring ? MSCRemoteStyle.bgElevated : MSCRemoteStyle.accentDim)
                        )
                }
                .disabled(isRestoring)
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
    }

    // MARK: - Backup Schedule Card

    private var backupScheduleCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Backup Schedule")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if let _ = vm.backupConfigResponse {
                scheduleRows
            } else if isLoading {
                ProgressView()
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, MSCRemoteStyle.spaceLG)
            } else {
                emptyState(icon: "calendar.badge.clock", message: "No data — pull to refresh.")
            }
        }
        .mscCard()
        .onChange(of: vm.backupConfigResponse) { _, cfg in
            guard let cfg else { return }
            seedScheduleDraft(from: cfg)
        }
    }

    private var scheduleRows: some View {
        VStack(spacing: 0) {
            // Enabled toggle
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Auto-Backup Enabled")
                        .font(.system(size: 14))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("Snapshots while the server runs, pruned automatically")
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                Spacer()
                Toggle("", isOn: $scheduleEnabled)
                    .labelsHidden()
                    .disabled(!isAdmin)
                    .tint(MSCRemoteStyle.accent)
            }
            .padding(.vertical, MSCRemoteStyle.spaceSM + 2)

            if scheduleEnabled {
                Divider().background(MSCRemoteStyle.borderSubtle)

                // Interval picker
                HStack {
                    Text("Interval")
                        .font(.system(size: 14))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Spacer()
                    if isAdmin {
                        Picker("", selection: $scheduleInterval) {
                            ForEach(intervalOptions, id: \.self) { mins in
                                Text(formatInterval(mins)).tag(mins)
                            }
                        }
                        .labelsHidden()
                        .pickerStyle(.menu)
                        .tint(MSCRemoteStyle.accent)
                    } else {
                        Text(formatInterval(scheduleInterval))
                            .font(.system(size: 14))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                    }
                }
                .padding(.vertical, MSCRemoteStyle.spaceSM + 2)

                Divider().background(MSCRemoteStyle.borderSubtle)

                // Max stored stepper
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Max Stored")
                            .font(.system(size: 14))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                        Text("Oldest auto-backup pruned on each new backup")
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                    Spacer()
                    if isAdmin {
                        HStack(spacing: MSCRemoteStyle.spaceSM) {
                            Button {
                                if scheduleMaxCount > 3 { scheduleMaxCount -= 1 }
                            } label: {
                                Image(systemName: "minus.circle")
                                    .font(.system(size: 20))
                                    .foregroundStyle(scheduleMaxCount > 3 ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                            }
                            .buttonStyle(.plain)
                            Text("\(scheduleMaxCount)")
                                .font(.system(size: 14, weight: .semibold, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .frame(minWidth: 28)
                            Button {
                                if scheduleMaxCount < 50 { scheduleMaxCount += 1 }
                            } label: {
                                Image(systemName: "plus.circle")
                                    .font(.system(size: 20))
                                    .foregroundStyle(scheduleMaxCount < 50 ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                            }
                            .buttonStyle(.plain)
                        }
                    } else {
                        Text("\(scheduleMaxCount)")
                            .font(.system(size: 14))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                    }
                }
                .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
            }

            if isAdmin && scheduleDraftChanged {
                Divider().background(MSCRemoteStyle.borderSubtle)
                HStack {
                    if let msg = scheduleSaveMessage {
                        Text(msg)
                            .font(.system(size: 12))
                            .foregroundStyle(msg.hasPrefix("✓") ? MSCRemoteStyle.success : MSCRemoteStyle.danger)
                    }
                    Spacer()
                    Button {
                        Task { await saveSchedule() }
                    } label: {
                        if isSavingSchedule {
                            ProgressView().tint(.white)
                                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                                .padding(.vertical, MSCRemoteStyle.spaceSM)
                        } else {
                            Text("Save")
                                .font(.system(size: 13, weight: .semibold))
                                .foregroundStyle(.white)
                                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                                .padding(.vertical, MSCRemoteStyle.spaceSM)
                        }
                    }
                    .background(MSCRemoteStyle.accent)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    .disabled(isSavingSchedule)
                }
                .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
            }
        }
    }

    private var scheduleDraftChanged: Bool {
        scheduleEnabled != scheduleOriginalEnabled ||
        scheduleInterval != scheduleOriginalInterval ||
        scheduleMaxCount != scheduleOriginalMaxCount
    }

    private var intervalOptions: [Int] {
        vm.backupConfigResponse?.intervalOptions ?? [15, 30, 45, 60, 120, 240, 360]
    }

    private func formatInterval(_ minutes: Int) -> String {
        if minutes < 60 { return "\(minutes) min" }
        let h = minutes / 60
        return h == 1 ? "1 hour" : "\(h) hours"
    }

    private func seedScheduleDraft(from cfg: BackupConfigResponseDTO) {
        scheduleEnabled = cfg.autoBackupEnabled
        scheduleInterval = cfg.autoBackupIntervalMinutes
        scheduleMaxCount = cfg.autoBackupMaxCount
        scheduleOriginalEnabled = cfg.autoBackupEnabled
        scheduleOriginalInterval = cfg.autoBackupIntervalMinutes
        scheduleOriginalMaxCount = cfg.autoBackupMaxCount
        scheduleSaveMessage = nil
    }

    private func saveSchedule() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isSavingSchedule = true
        scheduleSaveMessage = nil
        let enabledChange:   Bool? = scheduleEnabled   != scheduleOriginalEnabled   ? scheduleEnabled   : nil
        let intervalChange:  Int?  = scheduleInterval  != scheduleOriginalInterval  ? scheduleInterval  : nil
        let maxCountChange:  Int?  = scheduleMaxCount  != scheduleOriginalMaxCount  ? scheduleMaxCount  : nil
        let error = await vm.updateBackupConfig(baseURL: baseURL, token: token,
                                                enabled: enabledChange,
                                                intervalMinutes: intervalChange,
                                                maxCount: maxCountChange)
        isSavingSchedule = false
        if error == nil {
            scheduleSaveMessage = "✓ Saved"
            // Re-seed so the original tracks the new saved state
            scheduleOriginalEnabled  = scheduleEnabled
            scheduleOriginalInterval = scheduleInterval
            scheduleOriginalMaxCount = scheduleMaxCount
        } else {
            scheduleSaveMessage = error ?? "Save failed"
        }
        Task {
            try? await Task.sleep(nanoseconds: 3_000_000_000)
            scheduleSaveMessage = nil
        }
    }

    // MARK: - Shared helpers

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
        async let w: () = vm.fetchWorlds(baseURL: baseURL, token: token)
        async let b: () = vm.fetchBackups(baseURL: baseURL, token: token)
        async let c: () = vm.fetchBackupConfig(baseURL: baseURL, token: token)
        _ = await (w, b, c)
        isLoading = false
    }

    private func performActivate(slot: WorldSlotDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isActivating = true
        let ok = await vm.activateWorldSlot(baseURL: baseURL, token: token, slotId: slot.id)
        isActivating = false
        if ok {
            showToast("Switching to \"\(slot.name)\"…")
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            await refresh()
        }
    }

    private func performBackupNow() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isBackingUp = true
        let ok = await vm.createBackupNow(baseURL: baseURL, token: token)
        isBackingUp = false
        if ok {
            showToast("Backup started")
            try? await Task.sleep(nanoseconds: 3_000_000_000)
            await refresh()
        }
    }

    private func performRestore(backup: BackupItemDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isRestoring = true
        let ok = await vm.restoreBackup(baseURL: baseURL, token: token, backupId: backup.id)
        isRestoring = false
        if ok {
            showToast("Restoring \"\(backup.displayName)\"…")
            try? await Task.sleep(nanoseconds: 3_000_000_000)
            await refresh()
        }
    }

    // MARK: - World management actions (P9)

    private func performCreate(name: String, seed: String?) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isMutating = true
        let err = await vm.createWorld(baseURL: baseURL, token: token, name: name, seed: seed)
        isMutating = false
        if let err {
            showToast(err)
        } else {
            showToast("Created \"\(name)\".")
            await refresh()
        }
    }

    private func performRename(slot: WorldSlotDTO, newName: String) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let trimmed = newName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { showToast("Enter a world name."); return }
        isMutating = true
        let err = await vm.renameWorld(baseURL: baseURL, token: token, slotId: slot.id, name: trimmed)
        isMutating = false
        if let err {
            showToast(err)
        } else {
            showToast("Renamed to \"\(trimmed)\".")
            await refresh()
        }
    }

    private func performReplace(dest: WorldSlotDTO, source: WorldSlotDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isMutating = true
        let err = await vm.replaceWorld(baseURL: baseURL, token: token, slotId: dest.id, sourceSlotId: source.id)
        isMutating = false
        if let err {
            showToast(err)
        } else {
            showToast("Replaced \"\(dest.name)\" with \"\(source.name)\".")
            await refresh()
        }
    }

    private func performRepair(slot: WorldSlotDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isMutating = true
        let err = await vm.repairWorld(baseURL: baseURL, token: token, slotId: slot.id)
        isMutating = false
        if let err {
            showToast(err)
            return
        }
        showToast("Repair started — the server will restart briefly.")
        await pollRepairUntilDone()
    }

    /// Polls GET /worlds until the server reports the repair has finished (isRepairing → false).
    private func pollRepairUntilDone() async {
        for _ in 0..<90 { // ~6 minutes max at 4s intervals
            try? await Task.sleep(nanoseconds: 4_000_000_000)
            await refresh()
            if vm.worldsResponse?.isRepairing != true { break }
        }
        if vm.worldsResponse?.isRepairing != true {
            showToast("World repair finished.")
        }
    }

    private func showToast(_ message: String) {
        withAnimation { toastMessage = message }
        Task {
            try? await Task.sleep(nanoseconds: 2_500_000_000)
            withAnimation { toastMessage = nil }
        }
    }

    // MARK: - Formatting

    private func formatBytes(_ bytes: Int64) -> String {
        let mb = Double(bytes) / (1024 * 1024)
        if mb >= 1000 { return String(format: "%.1f GB", mb / 1024) }
        if mb >= 1 { return String(format: "%.1f MB", mb) }
        return String(format: "%.0f KB", Double(bytes) / 1024)
    }

    private func shortDate(_ iso: String) -> String {
        let parser = ISO8601DateFormatter()
        parser.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = parser.date(from: iso) {
            let f = DateFormatter()
            f.dateStyle = .medium
            f.timeStyle = .none
            return f.string(from: date)
        }
        return iso
    }
}

// MARK: - Create World Sheet (P9)

/// Creates a fresh (empty) named world slot. The world generates the first time
/// the slot is activated, so this is safe to do while the server is running.
private struct CreateWorldView: View {
    @Environment(\.dismiss) private var dismiss

    // NOTE: must NOT be an `async` closure. A stored `async` closure property is typed
    // `nonisolated(nonsending)` under the current Swift concurrency default, and its
    // function-type metadata accessor is null — so AttributeGraph crashes (jump to 0x0)
    // the moment it reflects this view's fields on presentation. The caller wraps the
    // async work in a Task instead.
    let onCreate: (String, String?) -> Void

    @State private var nameText: String = ""
    @State private var seedText: String = ""

    private var nameIsValid: Bool {
        !nameText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: MSCRemoteStyle.spaceLG) {
                    VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
                        MSCSectionHeader(title: "New World")
                            .padding(.bottom, 4)

                        VStack(spacing: MSCRemoteStyle.spaceSM) {
                            nameField
                            seedField
                        }

                        Text("A fresh, empty world slot is created now. The world itself is generated the first time you set the slot as active.")
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .mscCard()

                    MSCActionButton(title: "Create World", icon: "plus",
                                    style: .primary, isEnabled: nameIsValid) {
                        submit()
                    }

                    Spacer()
                }
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .padding(.top, MSCRemoteStyle.spaceMD)
            }
            .navigationTitle("Create World")
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

    private var nameField: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Name")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
            TextField("My New World", text: $nameText)
                .textFieldStyle(.plain)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .padding(MSCRemoteStyle.spaceSM)
                .background(MSCRemoteStyle.bgBase)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM - 2, style: .continuous))
        }
    }

    private var seedField: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Seed (optional)")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
            TextField("Leave blank for a random seed", text: $seedText)
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

    private func submit() {
        let name = nameText.trimmingCharacters(in: .whitespacesAndNewlines)
        let seed = seedText.trimmingCharacters(in: .whitespacesAndNewlines)
        onCreate(name, seed.isEmpty ? nil : seed)
        dismiss()
    }
}
