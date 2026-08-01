import SwiftUI
import Combine

struct DashboardView: View {
    var navigateToConnectivity: (() -> Void)? = nil

    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

    @State private var selectedServerId: String = ""
    @State private var showDangerConfirm: Bool = false
    @State private var dangerCommandToConfirm: String = ""
    @State private var showSetActiveConfirm: Bool = false
    @State private var pendingActiveServerId: String = ""
    @State private var suppressServerSelectionPrompt: Bool = false
    @State private var performancePollTask: Task<Void, Never>? = nil
    @State private var connectivityPollTask: Task<Void, Never>? = nil
    @State private var showQuickGuide: Bool = false
    @State private var showManageServers: Bool = false

    @State private var now: Date = Date()
    @State private var serverStartedAt: Date? = nil
    private let ticker = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    private var isIPad: Bool { horizontalSizeClass == .regular }
    private var metricColumnCount: Int { isIPad ? 3 : 2 }
    private var horizontalPadding: CGFloat { isIPad ? MSCRemoteStyle.iPadContentPadding : MSCRemoteStyle.spaceLG }

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }
    private var isRunning: Bool { vm.status?.running == true }

    private var perfAgeLabel: String? {
        guard let ts = vm.performanceLatest?.timestamp else { return nil }
        let secs = Int(now.timeIntervalSince(ts))
        if secs < 5  { return "just now" }
        if secs < 100 { return "\(secs)s ago" }
        return ">99s ago"
    }

    private var activeServerNameText: String {
        guard isPaired else { return "—" }
        if let activeId = vm.status?.activeServerId,
           let name = vm.servers.first(where: { $0.id == activeId })?.name { return name }
        if !selectedServerId.isEmpty,
           let name = vm.servers.first(where: { $0.id == selectedServerId })?.name { return name }
        return vm.servers.isEmpty ? "Loading…" : "Unknown"
    }

    private var activeServerType: ServerType {
        let resolveId = vm.status?.activeServerId ?? selectedServerId
        if let fromServers = vm.servers.first(where: { $0.id == resolveId })?.resolvedServerType {
            return fromServers
        }
        return vm.status?.resolvedServerType ?? .java
    }

    private var pendingActiveServerDisplayName: String {
        guard !pendingActiveServerId.isEmpty else { return "Unknown" }
        return vm.servers.first(where: { $0.id == pendingActiveServerId })?.name ?? "Unknown"
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: 0) {
                    ScrollView(showsIndicators: false) {
                        scrollContent
                            .padding(.horizontal, horizontalPadding)
                            .padding(.top, MSCRemoteStyle.spaceMD)
                            .padding(.bottom, MSCRemoteStyle.spaceLG)
                            .frame(maxWidth: isIPad ? MSCRemoteStyle.contentMaxWidth : .infinity)
                            .frame(maxWidth: .infinity)
                    }
                    .frame(maxWidth: .infinity)
                    .refreshable { await refreshAll() }
                    footerText.padding(.vertical, MSCRemoteStyle.spaceMD)
                }
            }
            .navigationTitle("Home")
            .navigationBarTitleDisplayMode(.large)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button { showQuickGuide = true } label: {
                        AppIconMark(size: 26)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Quick Guide")
                }
            }
            .sheet(isPresented: $showQuickGuide) { QuickGuideView() }
            .alert("Confirm Command", isPresented: $showDangerConfirm) {
                Button("Cancel", role: .cancel) { }
                Button("Send", role: .destructive) {
                    Task { await sendCommandString(dangerCommandToConfirm) }
                }
            } message: {
                Text("This command can disrupt the server or players:\n\n\(dangerCommandToConfirm)\n\nAre you sure?")
            }
            .alert("Set Active Server", isPresented: $showSetActiveConfirm) {
                Button("Cancel", role: .cancel) { revertSelectionToActive() }
                Button("Set Active") { Task { await confirmSetActiveServer() } }
            } message: {
                Text("Set \"\(pendingActiveServerDisplayName)\" as the active server?")
            }
            .task { await initialRefreshIfPossible() }
            .task(id: isPaired) {
                performancePollTask?.cancel()
                connectivityPollTask?.cancel()
                guard isPaired else { return }
                startPerformancePolling()
                startConnectivityPolling()
            }
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.willResignActiveNotification)) { _ in
                guard isPaired else { return }
                Task { await refreshAll() }
            }
            .onDisappear {
                performancePollTask?.cancel()
                performancePollTask = nil
                connectivityPollTask?.cancel()
                connectivityPollTask = nil
            }
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.willEnterForegroundNotification)) { _ in
                guard isPaired else { return }
                Task { await refreshAll() }
            }
            .onReceive(ticker) { date in
                now = date
            }
            .onChange(of: vm.servers) { _, _ in syncSelectionFromStatusOrFirst() }
            .onChange(of: vm.status?.activeServerId) { _, _ in syncSelectionFromStatusOrFirst() }
            .onChange(of: selectedServerId) { _, newValue in handleServerSelectionChanged(to: newValue) }
            .onChange(of: vm.status?.running) { _, running in
                if running == true, serverStartedAt == nil {
                    serverStartedAt = Date()
                } else if running == false {
                    serverStartedAt = nil
                }
            }
            .background(manageServersSheetAnchor)
        }
    }

    private var manageServersSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showManageServers) {
                ManageServersSheet()
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
    }

    @ViewBuilder
    private var scrollContent: some View {
        VStack(spacing: MSCRemoteStyle.spaceLG) {
            if isIPad {
                HStack(alignment: .top, spacing: MSCRemoteStyle.spaceLG) {
                    DashboardStatusCard(
                        isRunning: isRunning,
                        isPaired: isPaired,
                        activeServerNameText: activeServerNameText,
                        serverStartedAt: serverStartedAt,
                        now: now,
                        refreshAction: { Task { await refreshAll() } },
                        connectivity: vm.connectivityResponse,
                        connectivityAction: navigateToConnectivity
                    )
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    DashboardServerCard(
                        servers: vm.servers,
                        activeServerId: vm.status?.activeServerId ?? selectedServerId,
                        activeServerNameText: activeServerNameText,
                        activeServerType: activeServerType,
                        isPaired: isPaired,
                        isRunning: isRunning,
                        selectedServerId: $selectedServerId,
                        manageAction: { showManageServers = true },
                        startAction: { Task { await startServer() } },
                        stopAction: { Task { await stopServer() } }
                    )
                    .frame(maxWidth: .infinity)
                }
                .fixedSize(horizontal: false, vertical: true)
            } else {
                DashboardStatusCard(
                    isRunning: isRunning,
                    isPaired: isPaired,
                    activeServerNameText: activeServerNameText,
                    serverStartedAt: serverStartedAt,
                    now: now,
                    refreshAction: { Task { await refreshAll() } },
                    connectivity: vm.connectivityResponse,
                    connectivityAction: navigateToConnectivity
                )
                DashboardServerCard(
                    servers: vm.servers,
                    activeServerId: vm.status?.activeServerId ?? selectedServerId,
                    activeServerNameText: activeServerNameText,
                    activeServerType: activeServerType,
                    isPaired: isPaired,
                    isRunning: isRunning,
                    selectedServerId: $selectedServerId,
                    manageAction: { showManageServers = true },
                    startAction: { Task { await startServer() } },
                    stopAction: { Task { await stopServer() } }
                )
            }

            if isPaired { serverSettingsLinkCard }

            DashboardPlayersCard(players: vm.players?.players ?? [], isRunning: isRunning)

            performanceLinkCard

            if settings.showJoinCard { JoinCardView() }
            if let err = vm.errorMessage, !err.isEmpty { errorBanner(err) }
        }
    }

    private var performanceLinkCard: some View {
        NavigationLink {
            PerformanceDetailView(isIPad: isIPad)
                .environmentObject(vm)
        } label: {
            DashboardPerformanceCard(
                activeServerType: activeServerType,
                performanceLatest: vm.performanceLatest,
                performanceHistory: vm.performanceHistory,
                performanceErrorMessage: vm.performanceErrorMessage,
                errorMessage: vm.errorMessage,
                isRunning: isRunning,
                now: now,
                metricColumnCount: metricColumnCount,
                perfAgeLabel: perfAgeLabel
            )
        }
        .buttonStyle(.plain)
        .accessibilityHint("Shows performance history")
    }

    private var serverSettingsLinkCard: some View {
        NavigationLink {
            ServerSettingsView()
                .environmentObject(settings)
                .environmentObject(vm)
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Image(systemName: "slider.horizontal.3")
                    .font(.system(size: 18))
                    .foregroundStyle(MSCRemoteStyle.accent)
                    .frame(width: 28)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text("Server Settings")
                        .font(.system(size: 15, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("Edit difficulty, players, MOTD, ports & more")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .accessibilityHidden(true)
            }
            .mscCard()
        }
        .buttonStyle(.plain)
        .accessibilityElement(children: .combine)
    }

    private func errorBanner(_ message: String) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.danger)
                .accessibilityHidden(true)
            Text(message)
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.danger)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .mscCard(padding: MSCRemoteStyle.spaceMD)
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
                .strokeBorder(MSCRemoteStyle.danger.opacity(0.3), lineWidth: 1)
        )
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Error: \(message)")
    }

    private var footerText: some View {
        Text("TempleTech · MSC REMOTE")
            .font(.system(size: 10, weight: .regular, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .frame(maxWidth: .infinity, alignment: .center)
    }

    private func initialRefreshIfPossible() async {
        guard isPaired else { return }
        await refreshAll()
        syncSelectionFromStatusOrFirst()
    }
    private func refreshAll() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else {
            vm.errorMessage = "Set Base URL + Token in Settings first."
            return
        }
        await vm.refreshAll(baseURL: baseURL, token: token, tailN: 200)
        syncSelectionFromStatusOrFirst()
    }
    private func refreshUntilRunningState(expectedRunning: Bool) async {
        for _ in 0..<6 {
            await refreshAll()
            if vm.status?.running == expectedRunning { return }
            try? await Task.sleep(nanoseconds: 700_000_000)
        }
    }
    private func startPerformancePolling() {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        performancePollTask = Task {
            while !Task.isCancelled {
                await vm.fetchPerformanceSnapshot(baseURL: baseURL, token: token)
                try? await Task.sleep(nanoseconds: 5_000_000_000)
            }
        }
    }
    /// Connectivity runs a live external reachability probe (~10s on the Mac), so it polls on a
    /// slower cadence than performance and stays out of the main refresh path.
    private func startConnectivityPolling() {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        connectivityPollTask = Task {
            while !Task.isCancelled {
                await vm.fetchConnectivity(baseURL: baseURL, token: token)
                try? await Task.sleep(nanoseconds: 20_000_000_000)
            }
        }
    }
    private func setSelectionSilently(_ serverId: String) {
        suppressServerSelectionPrompt = true
        selectedServerId = serverId
        DispatchQueue.main.async { suppressServerSelectionPrompt = false }
    }
    private func syncSelectionFromStatusOrFirst() {
        if let active = vm.status?.activeServerId, vm.servers.contains(where: { $0.id == active }) {
            setSelectionSilently(active); return
        }
        if selectedServerId.isEmpty, let first = vm.servers.first { setSelectionSilently(first.id) }
    }
    private func handleServerSelectionChanged(to newServerId: String) {
        guard !suppressServerSelectionPrompt, isPaired, !newServerId.isEmpty else { return }
        guard vm.connectedRole != "guest" else { revertSelectionToActive(); return }
        let activeId = vm.status?.activeServerId ?? ""
        guard newServerId != activeId else { return }
        pendingActiveServerId = newServerId
        showSetActiveConfirm = true
    }
    private func revertSelectionToActive() {
        let activeId = vm.status?.activeServerId ?? ""
        guard !activeId.isEmpty else { return }
        setSelectionSilently(activeId)
    }
    private func confirmSetActiveServer() async {
        let serverId = pendingActiveServerId
        guard !serverId.isEmpty, let baseURL = resolvedBaseURL, let token = resolvedToken else {
            revertSelectionToActive(); return
        }
        let ok = await vm.setActiveServer(baseURL: baseURL, token: token, serverId: serverId)
        if ok { await refreshAll() } else { revertSelectionToActive() }
    }
    private func startServer() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let ok = await vm.start(baseURL: baseURL, token: token)
        if ok { hapticSuccess(); await refreshUntilRunningState(expectedRunning: true) } else { hapticError() }
    }
    private func stopServer() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let ok = await vm.stop(baseURL: baseURL, token: token)
        if ok { hapticSuccess(); await refreshUntilRunningState(expectedRunning: false) } else { hapticError() }
    }
    private func sendCommandString(_ cmd: String) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let trimmed = cmd.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        let ok = await vm.sendCommand(baseURL: baseURL, token: token, command: trimmed)
        if ok { hapticSuccess(); await refreshAll() } else { hapticError() }
    }
}

private struct ManageServersSheet: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    @State private var serverToRename: ServerDTO? = nil
    @State private var renameText: String = ""
    @State private var serverToDelete: ServerDTO? = nil
    @State private var mutatingServerId: String? = nil
    @State private var toastMessage: String? = nil
    @State private var showCreateServer: Bool = false
    @State private var showImport: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isAdmin: Bool { vm.connectedRole == "admin" }
    private var activeServerId: String { vm.status?.activeServerId ?? "" }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        serversCard
                        lifecycleCard
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                    .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                    .frame(maxWidth: .infinity)
                }
                .refreshable { await refreshServers() }

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
                    .animation(.spring(response: 0.4, dampingFraction: 0.8), value: toastMessage)
                }
            }
            .navigationTitle("Manage Servers")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button { Task { await refreshServers() } } label: {
                        Image(systemName: "arrow.clockwise")
                            .foregroundStyle(MSCRemoteStyle.accent)
                    }
                    .disabled(resolvedBaseURL == nil || resolvedToken == nil)
                    .accessibilityLabel("Refresh servers")
                }
            }
            .task { await refreshServers() }
            .background(renameAlertAnchor)
            .background(deleteAlertAnchor)
            .background(createServerSheetAnchor)
            .background(importSheetAnchor)
        }
    }

    private var serversCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "SERVERS", trailing: "\(vm.servers.count)")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if vm.servers.isEmpty {
                Text("No servers found.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, MSCRemoteStyle.spaceLG)
            } else {
                ForEach(vm.servers) { server in
                    serverRow(server)
                    if server.id != vm.servers.last?.id {
                        Divider().background(MSCRemoteStyle.borderSubtle)
                    }
                }
            }
        }
        .mscCard()
    }

    private var lifecycleCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "LIFECYCLE")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            openSheetRow(icon: "plus.square.on.square", title: "Create Server",
                         detail: "Create a fresh Java or Bedrock server on the Mac.") {
                hapticLight()
                showCreateServer = true
            }

            Divider().background(MSCRemoteStyle.borderSubtle)

            openSheetRow(icon: "square.and.arrow.down.on.square", title: "Import / Transfer",
                         detail: "Import a server folder, zip archive, or transfer package.") {
                hapticLight()
                showImport = true
            }
        }
        .mscCard()
    }

    private func openSheetRow(icon: String, title: String, detail: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Image(systemName: icon)
                    .font(.system(size: 20, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.accent)
                    .frame(width: 30)
                VStack(alignment: .leading, spacing: 3) {
                    Text(title)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text(detail)
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
            .padding(.vertical, MSCRemoteStyle.spaceSM)
        }
        .buttonStyle(.plain)
    }

    private func serverRow(_ server: ServerDTO) -> some View {
        let isActive = server.id == activeServerId
        let isBusy = mutatingServerId == server.id

        return HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: server.resolvedServerType.iconName)
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(isActive ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                .frame(width: 28)

            Button {
                guard isAdmin, !isBusy else { return }
                hapticLight()
                renameText = server.name
                serverToRename = server
            } label: {
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: MSCRemoteStyle.spaceSM) {
                        Text(server.name)
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                            .lineLimit(1)
                        if isActive {
                            Text("ACTIVE")
                                .font(.system(size: 10, weight: .semibold))
                                .foregroundStyle(MSCRemoteStyle.accent)
                                .padding(.horizontal, MSCRemoteStyle.spaceSM)
                                .padding(.vertical, 3)
                                .background(MSCRemoteStyle.accentDim)
                                .clipShape(Capsule())
                        }
                    }
                    Text(server.resolvedServerType.displayName)
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .disabled(!isAdmin || isBusy)

            if isBusy {
                ProgressView().scaleEffect(0.8)
            } else if isAdmin {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    if server.resolvedServerType == .java {
                        Button {
                            hapticLight()
                            Task { await performAcceptEULA(server: server) }
                        } label: {
                            Image(systemName: "checkmark.seal")
                                .font(.system(size: 13, weight: .semibold))
                                .foregroundStyle(MSCRemoteStyle.success)
                                .frame(width: 32, height: 32)
                                .background(MSCRemoteStyle.bgElevated)
                                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Accept EULA for \(server.name)")
                    }

                    Button {
                        hapticLight()
                        renameText = server.name
                        serverToRename = server
                    } label: {
                        Image(systemName: "pencil")
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.accent)
                            .frame(width: 32, height: 32)
                            .background(MSCRemoteStyle.bgElevated)
                            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Rename \(server.name)")

                    Button {
                        hapticLight()
                        serverToDelete = server
                    } label: {
                        Image(systemName: "trash")
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.danger)
                            .frame(width: 32, height: 32)
                            .background(MSCRemoteStyle.bgElevated)
                            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Delete \(server.name)")
                }
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM)
    }

    private var renameAlertAnchor: some View {
        Color.clear
            .alert("Rename Server", isPresented: Binding(
                get: { serverToRename != nil },
                set: { if !$0 { serverToRename = nil } }
            )) {
                TextField("Server name", text: $renameText)
                Button("Cancel", role: .cancel) { serverToRename = nil }
                Button("Rename") {
                    guard let server = serverToRename else { return }
                    let newName = renameText
                    serverToRename = nil
                    Task { await performRename(server: server, name: newName) }
                }
            } message: {
                Text("Enter a new name for this server.")
            }
    }

    private var deleteAlertAnchor: some View {
        Color.clear
            .alert("Delete Server", isPresented: Binding(
                get: { serverToDelete != nil },
                set: { if !$0 { serverToDelete = nil } }
            )) {
                Button("Cancel", role: .cancel) { serverToDelete = nil }
                Button("Delete from Disk", role: .destructive) {
                    guard let server = serverToDelete else { return }
                    serverToDelete = nil
                    Task { await performDelete(server: server) }
                }
            } message: {
                if let server = serverToDelete {
                    Text("Delete \(server.name)? This removes it from MSC and deletes its server folder from the Mac.")
                }
            }
    }

    private var createServerSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showCreateServer) {
                ServerCreateSheet()
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
    }

    private var importSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showImport) {
                ServerImportSheet()
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
    }

    private func refreshServers() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        await vm.refreshAll(baseURL: baseURL, token: token, tailN: 200)
    }

    private func performRename(server: ServerDTO, name: String) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            hapticError()
            showToast("Enter a server name.")
            return
        }
        mutatingServerId = server.id
        let error = await vm.renameServer(baseURL: baseURL, token: token, serverId: server.id, name: trimmed)
        mutatingServerId = nil
        if let error {
            hapticError()
            showToast(error)
        } else {
            hapticSuccess()
            showToast("Renamed to \"\(trimmed)\".")
        }
    }

    private func performDelete(server: ServerDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        mutatingServerId = server.id
        let error = await vm.deleteServer(baseURL: baseURL, token: token, serverId: server.id)
        mutatingServerId = nil
        if let error {
            hapticError()
            showToast(error)
        } else {
            hapticSuccess()
            showToast("Removed \"\(server.name)\".")
            await refreshServers()
        }
    }

    private func performAcceptEULA(server: ServerDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        mutatingServerId = server.id
        let error = await vm.acceptServerEULA(baseURL: baseURL, token: token, serverId: server.id)
        mutatingServerId = nil
        if let error {
            hapticError()
            showToast(error)
        } else {
            hapticSuccess()
            showToast("Accepted EULA for \"\(server.name)\".")
        }
    }

    private func showToast(_ message: String) {
        withAnimation { toastMessage = message }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { toastMessage = nil }
        }
    }
}

private struct ServerCreateSheet: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    @State private var serverType: ServerType = .java
    @State private var javaFlavor: RemoteJavaServerFlavor = .paper
    @State private var serverName: String = ""
    @State private var port: String = "25565"
    @State private var maxPlayers: String = "20"
    @State private var enableCrossPlay: Bool = false
    @State private var crossPlayPort: String = "19132"
    @State private var enablePlayit: Bool = false
    @State private var enableXboxBroadcast: Bool = false
    @State private var acceptEula: Bool = false
    @State private var difficulty: String = "normal"
    @State private var gamemode: String = "survival"
    @State private var worldName: String = ""
    @State private var worldSeed: String = ""
    @State private var selectedVersion: VersionEntryDTO? = nil
    @State private var createVersionsResponse: VersionsResponseDTO? = nil
    @State private var isLoadingVersions: Bool = false
    @State private var showVersionPicker: Bool = false
    @State private var selectedJavaRuntime: JavaRuntimeDTO? = nil
    @State private var javaRuntimesResponse: JavaRuntimesResponseDTO? = nil
    @State private var isLoadingRuntimes: Bool = false
    @State private var showRuntimePicker: Bool = false
    @State private var isWorking: Bool = false
    @State private var toastMessage: String? = nil
    @State private var createWarningMessage: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isAdmin: Bool { vm.connectedRole == "admin" }
    private var effectivePort: Int {
        Int(port) ?? (serverType == .bedrock ? 19132 : 25565)
    }
    private var effectiveMaxPlayers: Int {
        Int(maxPlayers) ?? (serverType == .bedrock ? 10 : 20)
    }
    private var supportsJavaCrossPlay: Bool {
        serverType == .java && javaFlavor.supportsCrossPlay
    }
    private var supportsXboxBroadcast: Bool {
        serverType == .bedrock || (supportsJavaCrossPlay && enableCrossPlay)
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        createCard
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                    .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                    .frame(maxWidth: .infinity)
                }
                toastOverlay
            }
            .navigationTitle("Create Server")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .background(versionPickerSheetAnchor)
            .background(javaRuntimePickerSheetAnchor)
            .alert("Xbox Broadcast", isPresented: Binding(
                get: { createWarningMessage != nil },
                set: { if !$0 { createWarningMessage = nil } }
            )) {
                Button("OK") { createWarningMessage = nil }
            } message: {
                Text(createWarningMessage ?? "")
            }
        }
    }

    private var createCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            MSCSectionHeader(title: "NEW SERVER")

            Picker("", selection: $serverType) {
                Text("Java").tag(ServerType.java)
                Text("Bedrock").tag(ServerType.bedrock)
            }
            .pickerStyle(.segmented)
            .tint(MSCRemoteStyle.accent)
            .onChange(of: serverType) { old, new in
                if old == .java, port == "25565" { port = "19132" }
                if old == .bedrock, port == "19132" { port = "25565" }
                if old == .java, maxPlayers == "20" { maxPlayers = "10" }
                if old == .bedrock, maxPlayers == "10" { maxPlayers = "20" }
                if new == .bedrock { enableCrossPlay = false }
                if new == .java { enableXboxBroadcast = false }
                resetVersionSelection()
            }

            if serverType == .java {
                javaSoftwareSection
                javaRuntimeRow
            }

            versionPickerRow

            labeledField("Server Name", text: $serverName, placeholder: serverType == .bedrock ? "New Bedrock Server" : "New \(javaFlavor.displayName) Server")
            labeledField(serverType == .bedrock ? "Bedrock Port" : "Java Port", text: $port, placeholder: serverType == .bedrock ? "19132" : "25565", keyboard: .numberPad)
            labeledField("Max Players", text: $maxPlayers, placeholder: serverType == .bedrock ? "10" : "20", keyboard: .numberPad)
            labeledField("World Name", text: $worldName, placeholder: "Optional")
            labeledField("World Seed", text: $worldSeed, placeholder: "Optional")

            Picker("", selection: $difficulty) {
                Text("Peaceful").tag("peaceful")
                Text("Easy").tag("easy")
                Text("Normal").tag("normal")
                Text("Hard").tag("hard")
            }
            .pickerStyle(.segmented)
            .tint(MSCRemoteStyle.accent)

            Picker("", selection: $gamemode) {
                Text("Survival").tag("survival")
                Text("Creative").tag("creative")
                Text("Adventure").tag("adventure")
            }
            .pickerStyle(.segmented)
            .tint(MSCRemoteStyle.accent)

            if serverType == .java {
                if supportsJavaCrossPlay {
                    toggleRow(title: "Bedrock Cross-play", detail: "Copy Geyser and Floodgate into the new \(javaFlavor.displayName) server.", isOn: $enableCrossPlay)
                }
                if supportsJavaCrossPlay && enableCrossPlay {
                    labeledField("Cross-play Port", text: $crossPlayPort, placeholder: "19132", keyboard: .numberPad)
                }
                toggleRow(title: "Accept EULA", detail: "Write eula=true after creation.", isOn: $acceptEula)
            }
            toggleRow(title: "Enable playit", detail: "Start with playit enabled in the new server config.", isOn: $enablePlayit)
            if supportsXboxBroadcast {
                toggleRow(title: "Xbox Broadcast", detail: "Let console players discover this server from Friends.", isOn: $enableXboxBroadcast)
            }

            actionButton(title: isWorking ? "Creating…" : "Create Server",
                         icon: "plus",
                         enabled: isAdmin && !isWorking) {
                Task { await performCreate() }
            }
        }
        .mscCard()
    }

    private var javaSoftwareSection: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            MSCSectionHeader(title: "STANDARD")
            VStack(spacing: MSCRemoteStyle.spaceSM) {
                ForEach(RemoteJavaServerFlavor.choices(in: .standard), id: \.self) { flavor in
                    javaFlavorRow(flavor)
                }
            }
            MSCSectionHeader(title: "MODDED")
                .padding(.top, MSCRemoteStyle.spaceXS)
            VStack(spacing: MSCRemoteStyle.spaceSM) {
                ForEach(RemoteJavaServerFlavor.choices(in: .modded), id: \.self) { flavor in
                    javaFlavorRow(flavor)
                }
            }
        }
    }

    private func javaFlavorRow(_ flavor: RemoteJavaServerFlavor) -> some View {
        Button {
            javaFlavor = flavor
            if !flavor.supportsCrossPlay {
                enableCrossPlay = false
                enableXboxBroadcast = false
            } else if !enableCrossPlay {
                enableXboxBroadcast = false
            }
            resetVersionSelection()
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Image(systemName: flavor.iconName)
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(javaFlavor == flavor ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                    .frame(width: 26)

                VStack(alignment: .leading, spacing: 3) {
                    Text(flavor.displayName)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text(flavor.shortDescription)
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                }

                Spacer()

                if javaFlavor == flavor {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .padding(MSCRemoteStyle.spaceMD)
            .background(javaFlavor == flavor ? MSCRemoteStyle.accentDim : MSCRemoteStyle.bgElevated)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .stroke(javaFlavor == flavor ? MSCRemoteStyle.accent.opacity(0.45) : MSCRemoteStyle.borderSubtle, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }

    private var versionPickerRow: some View {
        Button {
            Task { await openVersionPicker() }
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Image(systemName: "number.square")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(selectedVersion == nil ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.accent)
                    .frame(width: 26)

                VStack(alignment: .leading, spacing: 3) {
                    Text("Version")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text(selectedVersion.map(versionSummary) ?? (serverType == .bedrock ? "Latest (auto)" : "Latest (recommended)"))
                        .font(.system(size: 12))
                        .foregroundStyle(selectedVersion == nil ? MSCRemoteStyle.textSecondary : MSCRemoteStyle.accent)
                        .lineLimit(1)
                }

                Spacer()

                if isLoadingVersions {
                    ProgressView()
                        .scaleEffect(0.8)
                } else {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            .padding(MSCRemoteStyle.spaceMD)
            .background(MSCRemoteStyle.bgElevated)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .stroke(MSCRemoteStyle.borderSubtle, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .disabled(isLoadingVersions || !isAdmin)
    }

    private var versionPickerSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showVersionPicker) {
                CreateVersionPickerSheet(
                    response: createVersionsResponse,
                    serverType: serverType,
                    selectedVersion: $selectedVersion
                )
            }
    }

    private var javaRuntimeRow: some View {
        Button {
            Task { await openRuntimePicker() }
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Image(systemName: "cpu")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(selectedJavaRuntime == nil ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.accent)
                    .frame(width: 26)
                VStack(alignment: .leading, spacing: 3) {
                    Text("Java Runtime")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    if let runtime = selectedJavaRuntime {
                        Text("\(runtime.name)  ·  \(runtime.versionLabel)")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.accent)
                            .lineLimit(1)
                    } else {
                        Text("System default")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                    }
                }
                Spacer()
                if isLoadingRuntimes {
                    ProgressView().scaleEffect(0.8)
                } else {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            .padding(MSCRemoteStyle.spaceMD)
            .background(MSCRemoteStyle.bgElevated)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .stroke(MSCRemoteStyle.borderSubtle, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .disabled(isLoadingRuntimes || !isAdmin)
    }

    private var javaRuntimePickerSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showRuntimePicker) {
                CreateJavaRuntimePickerSheet(
                    response: javaRuntimesResponse,
                    selectedRuntime: $selectedJavaRuntime
                )
            }
    }

    private func openRuntimePicker() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else {
            hapticError()
            showToast("Pair with your Mac first.")
            return
        }
        if javaRuntimesResponse == nil {
            isLoadingRuntimes = true
            javaRuntimesResponse = await vm.fetchJavaRuntimes(baseURL: baseURL, token: token)
            isLoadingRuntimes = false
        }
        showRuntimePicker = true
    }

    private func openVersionPicker() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else {
            hapticError()
            showToast("Pair with your Mac first.")
            return
        }
        if createVersionsResponse == nil {
            isLoadingVersions = true
            let result = await vm.fetchCreateVersions(
                baseURL: baseURL,
                token: token,
                serverType: serverType,
                javaFlavor: serverType == .java ? javaFlavor : nil
            )
            isLoadingVersions = false
            guard let result else {
                hapticError()
                showToast("Couldn't load versions.")
                return
            }
            createVersionsResponse = result
        }
        showVersionPicker = true
    }

    private func resetVersionSelection() {
        selectedVersion = nil
        createVersionsResponse = nil
        showVersionPicker = false
    }

    private func versionSummary(_ entry: VersionEntryDTO) -> String {
        var text = entry.displayLabel
        if let build = entry.buildLabel, !build.isEmpty {
            text += " · \(build)"
        } else if let loader = entry.loaderVersion, !loader.isEmpty {
            text += " · loader \(loader)"
        }
        return text
    }

    private func performCreate() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let name = serverName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { hapticError(); showToast("Enter a server name."); return }
        hapticLight()
        isWorking = true
        let (error, warnings) = await vm.createFreshServer(
            baseURL: baseURL,
            token: token,
            name: name,
            serverType: serverType,
            javaFlavor: serverType == .java ? javaFlavor : nil,
            selectedVersion: selectedVersion,
            port: effectivePort,
            maxPlayers: effectiveMaxPlayers,
            enableCrossPlay: supportsJavaCrossPlay && enableCrossPlay,
            crossPlayBedrockPort: Int(crossPlayPort) ?? 19132,
            enablePlayit: enablePlayit,
            enableXboxBroadcast: supportsXboxBroadcast && enableXboxBroadcast,
            difficulty: difficulty,
            gamemode: gamemode,
            worldName: worldName.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty,
            worldSeed: worldSeed.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty,
            acceptEula: serverType == .java && acceptEula,
            bedrockVersion: nil,
            dockerImage: nil,
            javaPath: serverType == .java ? selectedJavaRuntime?.executablePath : nil
        )
        isWorking = false
        if let error {
            hapticError()
            showToast(error)
        } else {
            hapticSuccess()
            showToast("Server created.")
            if warnings?.contains("xbox_broadcast_jar_not_configured") == true {
                createWarningMessage = "Xbox Broadcast was enabled but MSC hasn't downloaded the MCXboxBroadcast JAR yet. Open MSC on your Mac → Edit Server → Broadcast to download it."
            }
        }
    }

    private var toastOverlay: some View {
        Group {
            if let toastMessage {
                VStack {
                    Spacer()
                    Text(toastMessage)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        .padding(.vertical, MSCRemoteStyle.spaceMD)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(Capsule())
                        .padding(.bottom, MSCRemoteStyle.spaceLG)
                }
                .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
    }

    private func showToast(_ message: String) {
        withAnimation { toastMessage = message }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { toastMessage = nil }
        }
    }
}

private struct CreateVersionPickerSheet: View {
    let response: VersionsResponseDTO?
    let serverType: ServerType
    @Binding var selectedVersion: VersionEntryDTO?
    @Environment(\.dismiss) private var dismiss

    private var concreteVersions: [VersionEntryDTO] {
        response?.versions.filter { entry in
            entry.id != "__latest__" && entry.id.uppercased() != "LATEST"
        } ?? []
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    LazyVStack(spacing: 0) {
                        latestRow
                        if !concreteVersions.isEmpty {
                            Divider()
                                .background(MSCRemoteStyle.borderSubtle)
                                .padding(.leading, MSCRemoteStyle.spaceLG)
                        }
                        ForEach(Array(concreteVersions.enumerated()), id: \.element.id) { index, entry in
                            versionRow(entry)
                            if index < concreteVersions.count - 1 {
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
            }
            .navigationTitle(response?.flavorName.isEmpty == false ? "\(response?.flavorName ?? "") Version" : "Choose Version")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
    }

    private var latestRow: some View {
        Button {
            selectedVersion = nil
            dismiss()
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(serverType == .bedrock ? "Latest (auto)" : "Latest (recommended)")
                        .font(.system(size: 14, weight: selectedVersion == nil ? .semibold : .regular))
                        .foregroundStyle(selectedVersion == nil ? MSCRemoteStyle.accent : MSCRemoteStyle.textPrimary)
                    Text(serverType == .bedrock ? "Let the Bedrock image use the newest available release." : "MSC will fetch the newest stable version for this server type.")
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 6)
                if selectedVersion == nil {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 18))
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func versionRow(_ entry: VersionEntryDTO) -> some View {
        let isSelected = selectedVersion?.id == entry.id
        return Button {
            selectedVersion = entry
            dismiss()
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        Text(entry.displayLabel)
                            .font(.system(size: 14, weight: isSelected ? .semibold : .regular))
                            .foregroundStyle(isSelected ? MSCRemoteStyle.accent : MSCRemoteStyle.textPrimary)
                        if !entry.isStable {
                            versionBadge("BETA", color: MSCRemoteStyle.warning)
                        }
                    }
                    if let build = entry.buildLabel, !build.isEmpty {
                        Text(build)
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    } else if let loader = entry.loaderVersion, !loader.isEmpty {
                        Text("Loader: \(loader)")
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
                Spacer(minLength: 6)
                if isSelected {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 18))
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func versionBadge(_ label: String, color: Color) -> some View {
        Text(label)
            .font(.system(size: 8, weight: .semibold))
            .padding(.horizontal, 5)
            .padding(.vertical, 2)
            .background(Capsule().fill(color.opacity(0.18)))
            .foregroundStyle(color)
    }
}

private struct CreateJavaRuntimePickerSheet: View {
    let response: JavaRuntimesResponseDTO?
    @Binding var selectedRuntime: JavaRuntimeDTO?
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                if let runtimes = response?.runtimes, !runtimes.isEmpty {
                    ScrollView(showsIndicators: false) {
                        LazyVStack(spacing: 0) {
                            systemDefaultRow
                            Divider()
                                .background(MSCRemoteStyle.borderSubtle)
                                .padding(.leading, MSCRemoteStyle.spaceLG)
                            ForEach(Array(runtimes.enumerated()), id: \.element.id) { index, runtime in
                                runtimeRow(runtime)
                                if index < runtimes.count - 1 {
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
                } else {
                    VStack(spacing: MSCRemoteStyle.spaceMD) {
                        Image(systemName: "cpu")
                            .font(.system(size: 32, weight: .light))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                        Text("No Java runtimes detected")
                            .font(.system(size: 15, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                        Text("MSC will use the system default. Install a JDK on your Mac to see options here.")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }
            .navigationTitle("Java Runtime")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
    }

    private var systemDefaultRow: some View {
        let isSelected = selectedRuntime == nil
        return Button {
            selectedRuntime = nil
            dismiss()
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("System default")
                        .font(.system(size: 14, weight: isSelected ? .semibold : .regular))
                        .foregroundStyle(isSelected ? MSCRemoteStyle.accent : MSCRemoteStyle.textPrimary)
                    Text("Uses the Java path from MSC settings")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                Spacer(minLength: 6)
                if isSelected {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 18))
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func runtimeRow(_ runtime: JavaRuntimeDTO) -> some View {
        let isSelected = selectedRuntime?.id == runtime.id
        return Button {
            selectedRuntime = runtime
            dismiss()
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        Text(runtime.name)
                            .font(.system(size: 14, weight: isSelected ? .semibold : .regular))
                            .foregroundStyle(isSelected ? MSCRemoteStyle.accent : MSCRemoteStyle.textPrimary)
                        if let major = runtime.majorVersion {
                            Text("Java \(major)")
                                .font(.system(size: 9, weight: .semibold))
                                .padding(.horizontal, 5)
                                .padding(.vertical, 2)
                                .background(Capsule().fill(MSCRemoteStyle.accent.opacity(0.15)))
                                .foregroundStyle(MSCRemoteStyle.accent)
                        }
                    }
                    Text(runtime.executablePath)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .lineLimit(1)
                }
                Spacer(minLength: 6)
                if isSelected {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 18))
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

private struct ServerImportSheet: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    @State private var sourcePath: String = ""
    @State private var importKind: String = "folder"
    @State private var displayName: String = ""
    @State private var portText: String = ""
    @State private var maxPlayersText: String = ""
    @State private var activeWorldName: String = ""
    @State private var acceptEula: Bool = false
    @State private var enablePlayit: Bool = false
    @State private var replaceAll: Bool = false
    @State private var backupPath: String = ""
    @State private var isWorking: Bool = false
    @State private var toastMessage: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isAdmin: Bool { vm.connectedRole == "admin" }
    private var scan: ServerImportScanResponseDTO? { vm.serverImportScanResponse }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        sourceCard
                        if importKind == "transfer" { transferCard }
                        else if let scan { scanCard(scan) }
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                    .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                    .frame(maxWidth: .infinity)
                }
                toastOverlay
            }
            .navigationTitle("Import / Transfer")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
        }
    }

    private var sourceCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            MSCSectionHeader(title: "SOURCE ON MAC")
            labeledField("Path", text: $sourcePath, placeholder: "/Users/me/Downloads/server.zip")
            Picker("", selection: $importKind) {
                Text("Folder").tag("folder")
                Text("Zip").tag("zip")
                Text("Transfer").tag("transfer")
            }
            .pickerStyle(.segmented)
            .tint(MSCRemoteStyle.accent)
            if importKind != "transfer" {
                actionButton(title: isWorking ? "Scanning…" : "Scan Server",
                             icon: "doc.text.magnifyingglass",
                             enabled: isAdmin && !isWorking && !sourcePath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) {
                    Task { await performScan() }
                }
            }
        }
        .mscCard()
    }

    private func scanCard(_ scan: ServerImportScanResponseDTO) -> some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            MSCSectionHeader(title: "IMPORT SERVER")
            HStack {
                MSCStatusDot(isActive: true)
                Text(scan.serverType?.displayName ?? "Server")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                Spacer()
                Text(scan.eulaAccepted == true ? "EULA accepted" : "EULA needed")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(scan.eulaAccepted == true ? MSCRemoteStyle.success : MSCRemoteStyle.warning)
            }
            labeledField("Display Name", text: $displayName, placeholder: "Imported Server")
            labeledField("Port", text: $portText, placeholder: "25565", keyboard: .numberPad)
            labeledField("Max Players", text: $maxPlayersText, placeholder: "20", keyboard: .numberPad)
            labeledField("Active World", text: $activeWorldName, placeholder: scan.defaultWorldName ?? "world")
            toggleRow(title: "Accept EULA", detail: "Write eula=true during import.", isOn: $acceptEula)
            toggleRow(title: "Enable playit", detail: "Turn on playit for this imported server.", isOn: $enablePlayit)
            if let worlds = scan.worlds, !worlds.isEmpty {
                Divider().background(MSCRemoteStyle.borderSubtle)
                ForEach(worlds.prefix(4)) { world in
                    HStack {
                        Text(world.name)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                        Spacer()
                        Text(world.dimensionsLabel)
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
            }
            actionButton(title: isWorking ? "Importing…" : "Import Server",
                         icon: "square.and.arrow.down",
                         enabled: isAdmin && !isWorking) {
                Task { await performExistingImport() }
            }
        }
        .mscCard()
    }

    private var transferCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            MSCSectionHeader(title: "TRANSFER PACKAGE")
            toggleRow(title: "Replace all servers", detail: "Requires a backup path before import.", isOn: $replaceAll)
            if replaceAll {
                labeledField("Backup Path", text: $backupPath, placeholder: "/Users/me/Desktop/MSC-backup.msctransfer")
            }
            actionButton(title: isWorking ? "Importing…" : "Import Transfer",
                         icon: "archivebox",
                         enabled: isAdmin && !isWorking && !sourcePath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) {
                Task { await performTransferImport() }
            }
        }
        .mscCard()
    }

    private func performScan() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        hapticLight()
        isWorking = true
        let error = await vm.scanServerImport(baseURL: baseURL, token: token, sourcePath: sourcePath, importKind: importKind)
        isWorking = false
        if let error {
            hapticError()
            showToast(error)
        } else if let scan = vm.serverImportScanResponse {
            hapticSuccess()
            displayName = displayName.isEmpty ? URL(fileURLWithPath: sourcePath).deletingPathExtension().lastPathComponent : displayName
            portText = String(scan.port ?? (scan.serverType == .bedrock ? 19132 : 25565))
            maxPlayersText = String(scan.maxPlayers ?? 20)
            activeWorldName = scan.defaultWorldName ?? ""
            acceptEula = scan.eulaAccepted ?? false
            showToast("Scan complete.")
        }
    }

    private func performExistingImport() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken, let scan else { return }
        let name = displayName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { hapticError(); showToast("Enter a display name."); return }
        hapticLight()
        isWorking = true
        let error = await vm.importExistingServer(
            baseURL: baseURL,
            token: token,
            sourcePath: sourcePath,
            importKind: importKind,
            displayName: name,
            serverType: scan.serverType ?? .java,
            activeWorldName: activeWorldName.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty,
            port: Int(portText),
            maxPlayers: Int(maxPlayersText),
            acceptEula: acceptEula,
            enablePlayit: enablePlayit
        )
        isWorking = false
        if let error {
            hapticError()
            showToast(error)
        } else {
            hapticSuccess()
            showToast("Server imported.")
        }
    }

    private func performTransferImport() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        hapticLight()
        isWorking = true
        let error = await vm.importTransferPackage(
            baseURL: baseURL,
            token: token,
            sourcePath: sourcePath,
            replaceAll: replaceAll,
            backupPath: replaceAll ? backupPath.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty : nil
        )
        isWorking = false
        if let error {
            hapticError()
            showToast(error)
        } else {
            hapticSuccess()
            showToast("Transfer imported.")
        }
    }

    private var toastOverlay: some View {
        Group {
            if let toastMessage {
                VStack {
                    Spacer()
                    Text(toastMessage)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        .padding(.vertical, MSCRemoteStyle.spaceMD)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(Capsule())
                        .padding(.bottom, MSCRemoteStyle.spaceLG)
                }
                .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
    }

    private func showToast(_ message: String) {
        withAnimation { toastMessage = message }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { toastMessage = nil }
        }
    }
}

private func labeledField(_ label: String, text: Binding<String>, placeholder: String, keyboard: UIKeyboardType = .default) -> some View {
    VStack(alignment: .leading, spacing: 4) {
        Text(label)
            .font(.system(size: 11, weight: .medium))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
        TextField(placeholder, text: text)
            .font(.system(size: 13))
            .foregroundStyle(MSCRemoteStyle.textPrimary)
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .keyboardType(keyboard)
            .padding(.horizontal, MSCRemoteStyle.spaceMD)
            .padding(.vertical, 10)
            .background(MSCRemoteStyle.bgElevated)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .accessibilityLabel(label)
    }
}

private func toggleRow(title: String, detail: String, isOn: Binding<Bool>) -> some View {
    HStack {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
            Text(detail)
                .font(.system(size: 11))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
        Spacer()
        Toggle("", isOn: isOn)
            .labelsHidden()
            .tint(MSCRemoteStyle.accent)
            .accessibilityLabel(title)
            .accessibilityHint(detail)
    }
}

private func actionButton(title: String, icon: String, enabled: Bool, action: @escaping () -> Void) -> some View {
    Button(action: action) {
        HStack(spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 12, weight: .semibold))
            Text(title)
                .font(.system(size: 14, weight: .semibold))
        }
        .frame(maxWidth: .infinity)
        .frame(height: 44)
        .foregroundStyle(enabled ? MSCRemoteStyle.bgBase : MSCRemoteStyle.textTertiary)
        .background(enabled ? MSCRemoteStyle.accent : MSCRemoteStyle.bgElevated)
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
    }
    .buttonStyle(.plain)
    .disabled(!enabled)
}

private func emptyText(_ message: String) -> some View {
    Text(message)
        .font(.system(size: 13))
        .foregroundStyle(MSCRemoteStyle.textTertiary)
        .frame(maxWidth: .infinity)
        .padding(.vertical, MSCRemoteStyle.spaceLG)
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}
