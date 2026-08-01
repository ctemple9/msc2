import SwiftUI

enum HealthSection {
    case connectivity
    case maintenance

    var title: String {
        switch self {
        case .connectivity: return "Connectivity"
        case .maintenance:  return "Diagnostics & Maintenance"
        }
    }
}

struct HealthView: View {
    let section: HealthSection

    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.openURL) private var openURL

    @State private var isRefreshing: Bool = false
    @State private var isBroadcastActioning: Bool = false
    @State private var isTogglingAutoStart: Bool = false
    @State private var showPlayitTunnel: Bool = false
    @State private var showServerFiles: Bool = false
    @State private var showClientExport: Bool = false
    @State private var showComponents: Bool = false

    // Startup-repair action feedback (performRepair) — was previously
    // piggybacked on componentsCard's toast before U2 moved that card out;
    // now rendered on startupProblemsCard, its actual owner.
    @State private var updateToast: String? = nil

    // DuckDNS inline form (P13)
    @State private var duckdnsHostnameInput: String = ""
    @State private var isDuckDNSSaving: Bool = false
    @State private var duckdnsToast: String? = nil

    // Geyser inline form (P13)
    @State private var geyserAddressInput: String = ""
    @State private var geyserPortInput: String = ""
    @State private var isGeyserSaving: Bool = false
    @State private var geyserToast: String? = nil

    // Memory (RAM) inline editor
    @State private var ramMinInput: Double = 0
    @State private var ramMaxInput: Double = 0
    @State private var isRAMSaving: Bool = false
    @State private var ramToast: String? = nil

    // Diagnostics (P10)
    @State private var expandedHealthCards: Set<String> = []
    @State private var expandedProblems: Set<String> = []
    @State private var actingProblemId: String? = nil
    @State private var problemToDelete: StartupProblemDTO? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }
    private var activeServer: ServerDTO? {
        if let activeId = vm.status?.activeServerId,
           let server = vm.servers.first(where: { $0.id == activeId }) {
            return server
        }
        return vm.servers.first
    }
    private var activeServerType: ServerType {
        activeServer?.resolvedServerType ?? vm.status?.resolvedServerType ?? .java
    }
    private var supportsJavaBedrockCrossPlay: Bool {
        activeServerType == .java && (activeServer?.resolvedJavaFlavor ?? .paper).supportsCrossPlay
    }
    private var supportsXboxBroadcastSettings: Bool {
        activeServerType == .bedrock || supportsJavaBedrockCrossPlay
    }

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            VStack(spacing: 0) {
                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        switch section {
                        case .connectivity:
                            playitCard
                            if supportsXboxBroadcastSettings {
                                broadcastCard
                            }
                            duckDNSCard
                            if supportsJavaBedrockCrossPlay {
                                geyserCard
                            }
                        case .maintenance:
                            diagnosticsCard
                            startupProblemsCard
                            memoryCard
                        }
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                }
                .refreshable { await refreshForSection(includeHeavyDiagnostics: true) }
                footerText.padding(.vertical, MSCRemoteStyle.spaceMD)
            }
        }
        .navigationTitle(section.title)
        .navigationBarTitleDisplayMode(.large)
        .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
        .toolbarColorScheme(.dark, for: .navigationBar)
        .task(id: isPaired) {
            guard isPaired else { return }
            switch section {
            case .connectivity:
                await refreshConnectivity()
                while !Task.isCancelled {
                    try? await Task.sleep(nanoseconds: 5_000_000_000)
                    guard !Task.isCancelled else { break }
                    await refreshConnectivity()
                }
            case .maintenance:
                await refreshMaintenance()
                await refreshHealthCards()
            }
        }
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button { Task { await refreshForSection(includeHeavyDiagnostics: true) } } label: {
                    Image(systemName: "arrow.clockwise")
                        .rotationEffect(.degrees(isRefreshing ? 360 : 0))
                        .animation(isRefreshing ? .linear(duration: 0.8).repeatForever(autoreverses: false) : .default, value: isRefreshing)
                }
                .disabled(isRefreshing)
            }
        }
        .background(playitSheetAnchor)
        .background(problemDeleteDialogAnchor)
    }

    // MARK: - Components Nav Card (U2)

    private var componentsNavCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Server Components")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            Button {
                showComponents = true
            } label: {
                HStack(spacing: MSCRemoteStyle.spaceMD) {
                    Image(systemName: "puzzlepiece.extension")
                        .font(.system(size: 22))
                        .foregroundStyle(MSCRemoteStyle.accent.opacity(0.8))
                        .frame(width: 32)

                    VStack(alignment: .leading, spacing: 3) {
                        Text("Manage Components")
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                        Text("Mods, server version, and resource packs.")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                    }
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            .buttonStyle(.plain)
            .disabled(!isPaired)
        }
        .mscCard()
    }

    private var componentsSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showComponents) {
                ComponentsView()
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
    }

    // MARK: - Server Tools Card (P16)

    private var serverToolsCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Server Tools")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            // Server Files is admin-only (S1): the server-side GET /files endpoints are
            // gated to the owner admin token, so only surface the browser to admins —
            // non-admin roles would just receive 403s.
            if vm.connectedRole == "admin" {
                serverToolRow(
                    title: "Server Files",
                    subtitle: "Browse and preview files from the active server directory.",
                    icon: "folder",
                    action: {
                        hapticLight()
                        showServerFiles = true
                    }
                )

                Divider()
                    .background(MSCRemoteStyle.borderSubtle)
                    .padding(.vertical, MSCRemoteStyle.spaceSM)
            }

            serverToolRow(
                title: "Client Export",
                subtitle: "Share required client mods or Modrinth links with players.",
                icon: "square.and.arrow.up",
                action: {
                    hapticLight()
                    showClientExport = true
                }
            )
        }
        .mscCard()
    }

    private func serverToolRow(title: String, subtitle: String, icon: String, action: @escaping () -> Void) -> some View {
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
                    Text(subtitle)
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .fixedSize(horizontal: false, vertical: true)
                }

                Spacer()

                Image(systemName: "chevron.right")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private var serverFilesSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showServerFiles) {
                ServerFilesRemoteSheet()
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
    }

    private var clientExportSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showClientExport) {
                ClientExportRemoteSheet()
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
    }

    // MARK: - Playit Card (P12)

    private var playitCard: some View {
        let s = vm.playitStatusResponse
        let isRunning = s?.isRunning ?? false
        let dotColor: Color = s == nil ? MSCRemoteStyle.textTertiary : (isRunning ? MSCRemoteStyle.success : MSCRemoteStyle.danger)

        return VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "playit.gg Tunnel")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(s == nil ? "Unknown" : (isRunning ? "Running" : "Stopped"))
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(dotColor)
                    if let addr = s?.javaAddress ?? s?.bedrockAddress {
                        Text(addr)
                            .font(.system(size: 12, design: .monospaced))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .lineLimit(1)
                    } else if s?.playitEnabled == true && isRunning {
                        Text("Resolving address…")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    } else if s?.playitEnabled == false {
                        Text("Not enabled for this server")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
                Spacer()
                Circle()
                    .fill(dotColor)
                    .frame(width: 10, height: 10)
                    .shadow(color: dotColor.opacity(0.6), radius: 4)
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            Button { showPlayitTunnel = true } label: {
                HStack(spacing: 6) {
                    Image(systemName: "antenna.radiowaves.left.and.right")
                        .font(.system(size: 12, weight: .semibold))
                    Text("Manage Tunnel")
                        .font(.system(size: 14, weight: .semibold))
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.system(size: 12, weight: .semibold))
                }
                .frame(maxWidth: .infinity)
                .frame(height: 40)
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .padding(.horizontal, MSCRemoteStyle.spaceMD)
                .background(MSCRemoteStyle.bgElevated)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                )
            }
            .buttonStyle(.plain)
            .disabled(!isPaired)
        }
        .mscCard()
    }

    private var playitSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showPlayitTunnel, onDismiss: {
                if let baseURL = resolvedBaseURL, let token = resolvedToken {
                    Task { await vm.fetchPlayitStatus(baseURL: baseURL, token: token) }
                }
            }) {
                PlayitView()
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
    }

    private var problemDeleteDialogAnchor: some View {
        Color.clear
            .confirmationDialog(
                problemToDelete.map { "Delete \($0.offenderName)?" } ?? "Delete add-on?",
                isPresented: Binding(get: { problemToDelete != nil }, set: { if !$0 { problemToDelete = nil } }),
                titleVisibility: .visible
            ) {
                Button("Delete", role: .destructive) {
                    if let p = problemToDelete { Task { await performRepair(p, action: "delete") } }
                    problemToDelete = nil
                }
                Button("Cancel", role: .cancel) { problemToDelete = nil }
            } message: {
                Text("The add-on's JAR will be permanently removed from the server.")
            }
    }

    private func refreshForSection(includeHeavyDiagnostics: Bool) async {
        switch section {
        case .connectivity:
            await refreshConnectivity()
        case .maintenance:
            await refreshMaintenance()
            if includeHeavyDiagnostics {
                await refreshHealthCards()
            }
        }
    }

    private func refreshConnectivity() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isRefreshing = true
        async let c: () = vm.fetchComponentsAndBroadcast(baseURL: baseURL, token: token)
        async let t: () = vm.fetchPlayitStatus(baseURL: baseURL, token: token)
        async let d: () = vm.fetchDuckDNS(baseURL: baseURL, token: token)
        async let g: () = vm.fetchGeyserConfig(baseURL: baseURL, token: token)
        _ = await (c, t, d, g)
        isRefreshing = false
    }

    private func refreshMaintenance() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isRefreshing = true
        async let p: () = vm.fetchHealthProblems(baseURL: baseURL, token: token)
        async let r: () = vm.fetchRAMConfig(baseURL: baseURL, token: token)
        _ = await (p, r)
        isRefreshing = false
    }

    // MARK: - Broadcast Card

    private var broadcastCard: some View {
        let running = vm.broadcastStatus.map { $0.xboxBroadcastRunning || $0.bedrockBroadcastRunning } ?? false
        let statusColor: Color = vm.broadcastStatus == nil ? MSCRemoteStyle.textTertiary : (running ? MSCRemoteStyle.success : MSCRemoteStyle.danger)
        let statusLabel = vm.broadcastStatus == nil ? "Unknown" : (running ? "Running" : "Stopped")

        return VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Xbox Broadcast")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            // Status row
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Broadcast Status")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text(statusLabel)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(statusColor)
                }
                Spacer()
                Circle()
                    .fill(statusColor)
                    .frame(width: 10, height: 10)
                    .shadow(color: statusColor.opacity(0.6), radius: 4)
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            Divider().background(MSCRemoteStyle.borderSubtle).padding(.bottom, MSCRemoteStyle.spaceMD)

            // Auto-start toggle
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Auto-start with server")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("Start broadcast when the server starts")
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                Spacer()
                if isTogglingAutoStart {
                    ProgressView().scaleEffect(0.8)
                } else {
                    Toggle("", isOn: Binding(
                        get: { vm.broadcastAutoStart ?? false },
                        set: { newValue in
                            Task { await setAutoStart(newValue) }
                        }
                    ))
                    .labelsHidden()
                    .tint(MSCRemoteStyle.accent)
                    .disabled(!isPaired || isTogglingAutoStart)
                }
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            // Start / Stop button
            Button {
                Task { await toggleBroadcast(wasRunning: running) }
            } label: {
                HStack(spacing: 6) {
                    if isBroadcastActioning {
                        ProgressView().scaleEffect(0.8)
                    } else {
                        Image(systemName: running ? "stop.fill" : "play.fill")
                            .font(.system(size: 12, weight: .semibold))
                    }
                    Text(running ? "Stop Broadcast" : "Start Broadcast")
                        .font(.system(size: 14, weight: .semibold))
                }
                .frame(maxWidth: .infinity)
                .frame(height: 40)
                .foregroundStyle(running ? .white : MSCRemoteStyle.bgBase)
                .background(running ? MSCRemoteStyle.danger : MSCRemoteStyle.accent)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            }
            .buttonStyle(.plain)
            .disabled(!isPaired || isBroadcastActioning)
            .padding(.bottom, MSCRemoteStyle.spaceSM)

            // Re-authenticate button (restarts broadcast to trigger a new device code)
            Button {
                Task { await reAuthenticate() }
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "person.badge.key")
                        .font(.system(size: 12, weight: .semibold))
                    Text("Re-authenticate with Microsoft")
                        .font(.system(size: 13, weight: .semibold))
                }
                .frame(maxWidth: .infinity)
                .frame(height: 38)
                .foregroundStyle(MSCRemoteStyle.textSecondary)
                .background(MSCRemoteStyle.bgElevated)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                )
            }
            .buttonStyle(.plain)
            .disabled(!isPaired || isBroadcastActioning)
        }
        .mscCard()
    }

    // MARK: - Actions

    private func refresh() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isRefreshing = true
        async let c: () = vm.fetchComponentsAndBroadcast(baseURL: baseURL, token: token)
        async let p: () = vm.fetchHealthProblems(baseURL: baseURL, token: token)
        async let t: () = vm.fetchPlayitStatus(baseURL: baseURL, token: token)
        async let d: () = vm.fetchDuckDNS(baseURL: baseURL, token: token)
        async let g: () = vm.fetchGeyserConfig(baseURL: baseURL, token: token)
        async let r: () = vm.fetchRAMConfig(baseURL: baseURL, token: token)
        _ = await (c, p, t, d, g, r)
        isRefreshing = false
    }

    /// Runs the heavy diagnostic health checks (live subprocess + reachability probes on the
    /// Mac). Kept out of the 5s poll loop; called on appear and pull-to-refresh only.
    private func refreshHealthCards() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        vm.isLoadingHealth = true
        await vm.fetchHealth(baseURL: baseURL, token: token)
        vm.isLoadingHealth = false
    }

    // MARK: - Diagnostics Card (P10)

    private var diagnosticsCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                MSCSectionHeader(title: "Diagnostics")
                Spacer()
                if let overall = vm.healthResponse?.overallSeverity {
                    Circle().fill(severityColor(overall)).frame(width: 9, height: 9)
                }
                if vm.isLoadingHealth {
                    ProgressView().controlSize(.mini)
                } else {
                    Button { Task { await refreshHealthCards() } } label: {
                        Image(systemName: "arrow.clockwise")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.accent)
                    }
                    .disabled(!isPaired)
                }
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            if let health = vm.healthResponse, !health.cards.isEmpty {
                VStack(spacing: 0) {
                    ForEach(Array(health.cards.enumerated()), id: \.element.id) { index, card in
                        healthCardRow(card)
                        if index < health.cards.count - 1 {
                            Divider().background(MSCRemoteStyle.borderSubtle)
                        }
                    }
                }
            } else if vm.isLoadingHealth {
                ProgressView()
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, MSCRemoteStyle.spaceLG)
            } else {
                Text("No diagnostic data — pull to refresh.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
        }
        .mscCard()
    }

    private func healthCardRow(_ card: HealthCardDTO) -> some View {
        let expanded = expandedHealthCards.contains(card.id)
        return VStack(alignment: .leading, spacing: 6) {
            Button {
                if expanded { expandedHealthCards.remove(card.id) } else { expandedHealthCards.insert(card.id) }
            } label: {
                HStack(spacing: MSCRemoteStyle.spaceMD) {
                    ZStack {
                        RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                            .fill(severityColor(card.severity).opacity(0.14))
                            .frame(width: 30, height: 30)
                        Image(systemName: card.iconSystemName)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(severityColor(card.severity))
                    }
                    VStack(alignment: .leading, spacing: 2) {
                        Text(card.title)
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                        if let detail = card.detail, !expanded {
                            Text(firstLine(detail))
                                .font(.system(size: 11))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                                .lineLimit(1)
                        }
                    }
                    Spacer()
                    Circle().fill(severityColor(card.severity)).frame(width: 8, height: 8)
                    if card.detail != nil {
                        Image(systemName: expanded ? "chevron.up" : "chevron.down")
                            .font(.system(size: 10, weight: .medium))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                }
            }
            .buttonStyle(.plain)

            if expanded, let detail = card.detail {
                Text(detail)
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.leading, 42)
                if let label = card.actionLabel, let code = card.actionCode, code.hasPrefix("openURL:") {
                    let urlStr = String(code.dropFirst("openURL:".count))
                    Button {
                        if let url = URL(string: urlStr) { openURL(url) }
                    } label: {
                        Text(label)
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.accent)
                    }
                    .padding(.leading, 42)
                }
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
    }

    // MARK: - Startup Problems Card (P10)

    @ViewBuilder
    private var startupProblemsCard: some View {
        if let resp = vm.healthProblemsResponse, !resp.problems.isEmpty {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: 6) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundStyle(MSCRemoteStyle.warning)
                        .font(.system(size: 13))
                    MSCSectionHeader(title: resp.isSoftFail ? "Mods Failed to Load" : "Server Couldn't Start")
                }
                .padding(.bottom, 4)

                Text(resp.isSoftFail
                     ? "The server started, but \(resp.problems.count) mod\(resp.problems.count == 1 ? "" : "s") didn't load. Fix them, then restart."
                     : "\(resp.problems.count) \(resp.problems.count == 1 ? "problem" : "problems") stopped this server from starting.")
                    .font(.system(size: 12))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                if resp.serverRunning {
                    HStack(spacing: 6) {
                        Image(systemName: "info.circle").font(.system(size: 11))
                        Text("Stop the server to enable repairs.").font(.system(size: 11))
                    }
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .padding(.bottom, MSCRemoteStyle.spaceSM)
                }

                VStack(spacing: 0) {
                    ForEach(Array(resp.problems.enumerated()), id: \.element.id) { index, p in
                        problemRow(p, serverRunning: resp.serverRunning)
                        if index < resp.problems.count - 1 {
                            Divider().background(MSCRemoteStyle.borderSubtle)
                        }
                    }
                }

                if let toast = updateToast {
                    Text(toast)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.success)
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.top, MSCRemoteStyle.spaceSM)
                        .transition(.opacity)
                }
            }
            .mscCard()
            .animation(.easeInOut(duration: 0.2), value: updateToast)
        }
    }

    private func problemRow(_ p: StartupProblemDTO, serverRunning: Bool) -> some View {
        let expanded = expandedProblems.contains(p.id)
        let isAdmin = vm.connectedRole != "guest"
        let busy = actingProblemId == p.id || p.isRepairing
        return VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
                ZStack {
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .fill(problemTint(p.kind).opacity(0.14))
                        .frame(width: 30, height: 30)
                    Image(systemName: p.iconSystemName)
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(problemTint(p.kind))
                }
                VStack(alignment: .leading, spacing: 2) {
                    Text(p.offenderName)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    if let req = p.requirement {
                        Text(req)
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    Text(p.kindTitle)
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(problemTint(p.kind))
                }
                Spacer()
                if busy { ProgressView().controlSize(.small) }
            }

            if isAdmin && !busy {
                problemActions(p, serverRunning: serverRunning)
            }
            problemFooter(p, expanded: expanded)

            if expanded {
                Text(p.rawExcerpt)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(8)
                    .background(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM).fill(Color.black.opacity(0.25)))
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
    }

    private func problemActions(_ p: StartupProblemDTO, serverRunning: Bool) -> some View {
        HStack(spacing: 8) {
            if p.availableActions.contains("update") {
                problemActionButton("Update", tint: MSCRemoteStyle.accent, disabled: serverRunning) {
                    Task { await performRepair(p, action: "update") }
                }
            }
            if p.availableActions.contains("install") {
                problemActionButton("Install", tint: MSCRemoteStyle.accent, disabled: serverRunning) {
                    Task { await performRepair(p, action: "install") }
                }
            }
            if p.availableActions.contains("disable") {
                problemActionButton("Disable", tint: MSCRemoteStyle.textSecondary, disabled: serverRunning) {
                    Task { await performRepair(p, action: "disable") }
                }
            }
            if p.availableActions.contains("delete") {
                problemActionButton("Delete", tint: MSCRemoteStyle.danger, disabled: serverRunning) {
                    problemToDelete = p
                }
            }
        }
    }

    @ViewBuilder
    private func problemFooter(_ p: StartupProblemDTO, expanded: Bool) -> some View {
        HStack(spacing: 14) {
            if let urlStr = p.modrinthURL, let url = URL(string: urlStr) {
                Button { openURL(url) } label: {
                    HStack(spacing: 3) {
                        Image(systemName: "arrow.up.forward.square")
                        Text("Modrinth")
                    }
                    .font(.system(size: 10.5))
                    .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            Button {
                if expanded { expandedProblems.remove(p.id) } else { expandedProblems.insert(p.id) }
            } label: {
                HStack(spacing: 3) {
                    Image(systemName: expanded ? "chevron.down" : "chevron.right")
                    Text("Log detail")
                }
                .font(.system(size: 10.5))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
            Spacer()
        }
    }

    private func problemActionButton(_ title: String, tint: Color, disabled: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(disabled ? MSCRemoteStyle.textTertiary : tint)
                .padding(.horizontal, MSCRemoteStyle.spaceMD)
                .padding(.vertical, 5)
                .background(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                        .fill(disabled ? MSCRemoteStyle.bgElevated : tint.opacity(0.12))
                )
        }
        .disabled(disabled)
    }

    // MARK: - Diagnostics actions + helpers

    private func performRepair(_ p: StartupProblemDTO, action: String) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        actingProblemId = p.id
        let err = await vm.repairHealthProblem(baseURL: baseURL, token: token, problemId: p.id, action: action)
        actingProblemId = nil
        if let err { showUpdateToast(err); return }
        switch action {
        case "update":  showUpdateToast("Updating \(p.offenderName)…");   await pollProblems()
        case "install": showUpdateToast("Installing dependency…");        await pollProblems()
        case "disable": showUpdateToast("Disabled \(p.offenderName)")
        case "delete":  showUpdateToast("Deleted \(p.offenderName)")
        default: break
        }
        await vm.fetchHealthProblems(baseURL: baseURL, token: token)
    }

    /// After an async repair (update/install), polls problems until none are still repairing.
    private func pollProblems() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        for _ in 0..<20 { // ~80s at 4s intervals
            try? await Task.sleep(nanoseconds: 4_000_000_000)
            await vm.fetchHealthProblems(baseURL: baseURL, token: token)
            let stillRepairing = vm.healthProblemsResponse?.problems.contains { $0.isRepairing } ?? false
            if !stillRepairing { break }
        }
    }

    private func showUpdateToast(_ text: String) {
        withAnimation { updateToast = text }
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
            withAnimation { updateToast = nil }
        }
    }

    private func severityColor(_ severity: String) -> Color {
        switch severity {
        case "red":    return MSCRemoteStyle.danger
        case "yellow": return MSCRemoteStyle.warning
        case "green":  return MSCRemoteStyle.success
        default:       return MSCRemoteStyle.textTertiary
        }
    }

    private func problemTint(_ kind: String) -> Color {
        switch kind {
        case "missingDependency":   return MSCRemoteStyle.accent
        case "incompatibleVersion": return MSCRemoteStyle.warning
        default:                    return MSCRemoteStyle.danger
        }
    }

    private func firstLine(_ s: String) -> String {
        s.split(whereSeparator: \.isNewline).first.map(String.init) ?? s
    }

    private func toggleBroadcast(wasRunning: Bool) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isBroadcastActioning = true
        vm.updateCredentials(baseURL: baseURL, token: token)
        if wasRunning {
            _ = try? await vm.requireClient().stopBroadcast()
        } else {
            _ = try? await vm.requireClient().startBroadcast()
        }
        try? await Task.sleep(nanoseconds: 1_500_000_000)
        await vm.fetchComponentsAndBroadcast(baseURL: baseURL, token: token)
        isBroadcastActioning = false
    }

    private func setAutoStart(_ enabled: Bool) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isTogglingAutoStart = true
        vm.updateCredentials(baseURL: baseURL, token: token)
        _ = try? await vm.requireClient().setBroadcastAutoStart(enabled: enabled)
        vm.broadcastAutoStart = enabled  // optimistic update
        isTogglingAutoStart = false
    }

    private func reAuthenticate() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isBroadcastActioning = true
        vm.updateCredentials(baseURL: baseURL, token: token)
        _ = try? await vm.requireClient().stopBroadcast()
        try? await Task.sleep(nanoseconds: 500_000_000)
        _ = try? await vm.requireClient().startBroadcast()
        isBroadcastActioning = false
        // The polling loop will pick up the new auth prompt within a few seconds
    }

    // MARK: - DuckDNS Card (P13)

    @ViewBuilder
    private var duckDNSCard: some View {
        if isPaired {
            VStack(alignment: .leading, spacing: 0) {
                MSCSectionHeader(title: "DuckDNS")
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                if let response = vm.duckdnsResponse {
                    // Status row
                    HStack(spacing: 8) {
                        Circle()
                            .fill(response.isConfigured ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                            .frame(width: 8, height: 8)
                        Text(response.isConfigured ? (response.hostname ?? "") : "Not configured")
                            .font(.system(size: 12, design: .monospaced))
                            .foregroundStyle(response.isConfigured ? MSCRemoteStyle.textPrimary : MSCRemoteStyle.textTertiary)
                            .lineLimit(1)
                    }
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                    if vm.connectedRole == "admin" {
                        HStack(spacing: MSCRemoteStyle.spaceSM) {
                            TextField("yourname.duckdns.org", text: $duckdnsHostnameInput)
                                .font(.system(size: 13, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .textInputAutocapitalization(.never)
                                .autocorrectionDisabled()
                                .keyboardType(.URL)
                                .padding(.horizontal, MSCRemoteStyle.spaceMD)
                                .padding(.vertical, 10)
                                .background(MSCRemoteStyle.bgElevated)
                                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))

                            Button {
                                Task { await saveDuckDNS() }
                            } label: {
                                if isDuckDNSSaving {
                                    ProgressView().scaleEffect(0.8).frame(width: 60, height: 36)
                                } else {
                                    Text("Save")
                                        .font(.system(size: 13, weight: .semibold))
                                        .foregroundStyle(.white)
                                        .frame(width: 60, height: 36)
                                        .background(MSCRemoteStyle.accent)
                                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                                }
                            }
                            .buttonStyle(.plain)
                            .disabled(isDuckDNSSaving || !isPaired)
                        }

                        if let toast = duckdnsToast {
                            Text(toast)
                                .font(.system(size: 11))
                                .foregroundStyle(MSCRemoteStyle.success)
                                .frame(maxWidth: .infinity, alignment: .center)
                                .padding(.top, MSCRemoteStyle.spaceSM)
                                .transition(.opacity)
                        }
                    }
                } else {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, MSCRemoteStyle.spaceSM)
                }
            }
            .mscCard()
            .task(id: vm.duckdnsResponse?.hostname) {
                duckdnsHostnameInput = vm.duckdnsResponse?.hostname ?? ""
            }
            .animation(.easeInOut(duration: 0.2), value: duckdnsToast)
        }
    }

    // MARK: - Geyser Card (P13)

    @ViewBuilder
    private var geyserCard: some View {
        if isPaired, let response = vm.geyserConfigResponse, response.isGeyserInstalled {
            VStack(alignment: .leading, spacing: 0) {
                MSCSectionHeader(title: "Geyser Bedrock Config")
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                if !response.configFileExists {
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .font(.system(size: 11))
                            .foregroundStyle(Color.orange)
                        Text("config.yml not yet generated. Start the server once, then come back to set the address.")
                            .font(.system(size: 12))
                            .foregroundStyle(Color.orange.opacity(0.9))
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .padding(.bottom, MSCRemoteStyle.spaceMD)
                }

                if vm.connectedRole == "admin" && response.configFileExists {
                    VStack(spacing: MSCRemoteStyle.spaceSM) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Listen Address")
                                .font(.system(size: 11, weight: .medium))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                            TextField("0.0.0.0", text: $geyserAddressInput)
                                .font(.system(size: 13, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .textInputAutocapitalization(.never)
                                .autocorrectionDisabled()
                                .keyboardType(.URL)
                                .padding(.horizontal, MSCRemoteStyle.spaceMD)
                                .padding(.vertical, 10)
                                .background(MSCRemoteStyle.bgElevated)
                                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Bedrock Port")
                                .font(.system(size: 11, weight: .medium))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                            TextField("19132", text: $geyserPortInput)
                                .font(.system(size: 13, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                                .keyboardType(.numberPad)
                                .padding(.horizontal, MSCRemoteStyle.spaceMD)
                                .padding(.vertical, 10)
                                .background(MSCRemoteStyle.bgElevated)
                                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        }

                        Button {
                            Task { await saveGeyserConfig() }
                        } label: {
                            HStack(spacing: 6) {
                                if isGeyserSaving {
                                    ProgressView().scaleEffect(0.8)
                                } else {
                                    Image(systemName: "checkmark")
                                        .font(.system(size: 11, weight: .semibold))
                                }
                                Text(isGeyserSaving ? "Saving…" : "Save Geyser Config")
                                    .font(.system(size: 13, weight: .semibold))
                            }
                            .frame(maxWidth: .infinity)
                            .frame(height: 38)
                            .foregroundStyle(isGeyserSaving ? MSCRemoteStyle.textTertiary : .white)
                            .background(isGeyserSaving ? MSCRemoteStyle.bgElevated : MSCRemoteStyle.accent)
                            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        }
                        .buttonStyle(.plain)
                        .disabled(isGeyserSaving || !isPaired)
                        .padding(.top, MSCRemoteStyle.spaceSM)
                    }
                } else {
                    VStack(spacing: 4) {
                        if let addr = response.address {
                            HStack {
                                Text("Address")
                                    .font(.system(size: 12))
                                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                                Spacer()
                                Text(addr)
                                    .font(.system(size: 12, design: .monospaced))
                                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                            }
                        }
                        if let port = response.port {
                            HStack {
                                Text("Port")
                                    .font(.system(size: 12))
                                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                                Spacer()
                                Text(String(port))
                                    .font(.system(size: 12, design: .monospaced))
                                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                            }
                        }
                        if response.address == nil && response.port == nil {
                            Text("No config values set.")
                                .font(.system(size: 12))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                        }
                    }
                }

                if let toast = geyserToast {
                    Text(toast)
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.success)
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.top, MSCRemoteStyle.spaceMD)
                        .transition(.opacity)
                }
            }
            .mscCard()
            .task(id: response.address) {
                geyserAddressInput = response.address ?? ""
                geyserPortInput = response.port.map(String.init) ?? ""
            }
            .animation(.easeInOut(duration: 0.2), value: geyserToast)
        }
    }

    // MARK: - Memory (RAM) Card

    @ViewBuilder
    private var memoryCard: some View {
        if isPaired, let ram = vm.ramConfigResponse, ram.hasActiveServer {
            let isAdmin = vm.connectedRole != "guest"
            let physCap = Double(max(1, ram.physicalRAMGB))

            VStack(alignment: .leading, spacing: 0) {
                MSCSectionHeader(title: "Memory")
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                // Host RAM context
                HStack(spacing: 6) {
                    Image(systemName: "memorychip")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                    Text(ram.physicalRAMGB > 0
                         ? "\(ram.physicalRAMGB) GB installed · \(ram.recommendedMaxGB) GB recommended max"
                         : "Adjust the RAM allocated to this server.")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .padding(.bottom, MSCRemoteStyle.spaceMD)

                if isAdmin {
                    if ram.isBedrock {
                        ramStepperRow(label: "Memory Limit",
                                      subtitle: ramMaxInput == 0 ? "Default (2 GB)" : "Fixed VM memory",
                                      value: $ramMaxInput, range: 0...physCap, step: 0.5)
                    } else {
                        ramStepperRow(label: "Minimum", subtitle: "-Xms",
                                      value: $ramMinInput, range: 0.5...physCap, step: 0.5)
                        Divider().background(MSCRemoteStyle.borderSubtle)
                            .padding(.vertical, MSCRemoteStyle.spaceSM)
                        ramStepperRow(label: "Maximum", subtitle: "-Xmx",
                                      value: $ramMaxInput, range: 0.5...physCap, step: 0.5)
                    }

                    // Guidance / warnings
                    if !ram.isBedrock && ramMaxInput < ramMinInput {
                        ramNote("Maximum will be raised to match the minimum on save.",
                                icon: "arrow.up.circle", color: MSCRemoteStyle.warning)
                    }
                    if ram.recommendedMaxGB > 0 && ramMaxInput > Double(ram.recommendedMaxGB) {
                        ramNote("Above \(ram.recommendedMaxGB) GB can starve macOS — use with care.",
                                icon: "exclamationmark.triangle", color: MSCRemoteStyle.warning)
                    }
                    if ram.serverRunning {
                        ramNote("Server is running — changes apply after the next restart.",
                                icon: "arrow.clockwise", color: MSCRemoteStyle.textTertiary)
                    }

                    Button {
                        Task { await saveRAM(ram) }
                    } label: {
                        HStack(spacing: 6) {
                            if isRAMSaving {
                                ProgressView().scaleEffect(0.8)
                            } else {
                                Image(systemName: "checkmark")
                                    .font(.system(size: 11, weight: .semibold))
                            }
                            Text(isRAMSaving ? "Saving…" : "Save Memory")
                                .font(.system(size: 13, weight: .semibold))
                        }
                        .frame(maxWidth: .infinity)
                        .frame(height: 38)
                        .foregroundStyle((isRAMSaving || !ramHasChanges(ram)) ? MSCRemoteStyle.textTertiary : .white)
                        .background((isRAMSaving || !ramHasChanges(ram)) ? MSCRemoteStyle.bgElevated : MSCRemoteStyle.accent)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    }
                    .buttonStyle(.plain)
                    .disabled(isRAMSaving || !isPaired || !ramHasChanges(ram))
                    .padding(.top, MSCRemoteStyle.spaceMD)
                } else {
                    // Read-only for guests
                    if ram.isBedrock {
                        ramReadRow("Memory Limit", ram.maxRamGB == 0 ? "Default (2 GB)" : "\(gbLabel(ram.maxRamGB)) GB")
                    } else {
                        ramReadRow("Minimum", "\(gbLabel(ram.minRamGB)) GB")
                        Divider().background(MSCRemoteStyle.borderSubtle).padding(.vertical, MSCRemoteStyle.spaceSM)
                        ramReadRow("Maximum", "\(gbLabel(ram.maxRamGB)) GB")
                    }
                }

                if let toast = ramToast {
                    Text(toast)
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.success)
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.top, MSCRemoteStyle.spaceMD)
                        .transition(.opacity)
                }
            }
            .mscCard()
            .task(id: ramSeedKey(ram)) {
                ramMinInput = ram.minRamGB
                ramMaxInput = ram.maxRamGB
            }
            .animation(.easeInOut(duration: 0.2), value: ramToast)
        }
    }

    private func ramStepperRow(label: String, subtitle: String?,
                               value: Binding<Double>, range: ClosedRange<Double>, step: Double) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            VStack(alignment: .leading, spacing: 2) {
                Text(label)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                if let subtitle {
                    Text(subtitle)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            Spacer()
            Text("\(gbLabel(value.wrappedValue)) GB")
                .font(.system(size: 15, weight: .semibold, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.accent)
                .frame(minWidth: 62, alignment: .trailing)
            Stepper("", value: value, in: range, step: step)
                .labelsHidden()
        }
    }

    private func ramReadRow(_ label: String, _ value: String) -> some View {
        HStack {
            Text(label)
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.textSecondary)
            Spacer()
            Text(value)
                .font(.system(size: 14, weight: .semibold, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
        }
    }

    private func ramNote(_ text: String, icon: String, color: Color) -> some View {
        HStack(alignment: .top, spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 11))
                .foregroundStyle(color)
            Text(text)
                .font(.system(size: 11))
                .foregroundStyle(color)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.top, MSCRemoteStyle.spaceMD)
    }

    /// Formats a GB value, dropping the decimal for whole numbers (4.0 → "4", 4.5 → "4.5").
    private func gbLabel(_ value: Double) -> String { String(format: "%g", value) }

    /// `.task(id:)` key — re-seeds the steppers only when the server or its stored values change,
    /// so the 5s poll can't stomp an in-progress edit.
    private func ramSeedKey(_ ram: RAMConfigResponseDTO) -> String {
        "\(ram.serverName)|\(ram.serverType)|\(ram.minRamGB)|\(ram.maxRamGB)"
    }

    private func ramHasChanges(_ ram: RAMConfigResponseDTO) -> Bool {
        if ram.isBedrock { return ramMaxInput != ram.maxRamGB }
        return ramMinInput != ram.minRamGB || ramMaxInput != ram.maxRamGB
    }

    private func saveRAM(_ ram: RAMConfigResponseDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isRAMSaving = true
        let result = await vm.updateRAMConfig(
            minRamGB: ram.isBedrock ? nil : ramMinInput,
            maxRamGB: ramMaxInput,
            baseURL: baseURL, token: token
        )
        isRAMSaving = false
        let ok = result?.success == true
        if ok { hapticSuccess() } else { hapticError() }
        withAnimation {
            ramToast = ok
                ? (result?.restartRequired == true ? "Saved — restart to apply" : "Saved")
                : "Save failed"
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { ramToast = nil }
        }
    }

    // MARK: - Save actions (P13)

    private func saveDuckDNS() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isDuckDNSSaving = true
        let trimmed = duckdnsHostnameInput.trimmingCharacters(in: .whitespacesAndNewlines)
        let ok = await vm.updateDuckDNS(hostname: trimmed.isEmpty ? nil : trimmed, baseURL: baseURL, token: token)
        isDuckDNSSaving = false
        withAnimation { duckdnsToast = ok ? "Saved" : "Save failed" }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { duckdnsToast = nil }
        }
    }

    private func saveGeyserConfig() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isGeyserSaving = true
        let addr = geyserAddressInput.trimmingCharacters(in: .whitespacesAndNewlines)
        let portInt = Int(geyserPortInput.trimmingCharacters(in: .whitespacesAndNewlines))
        let result = await vm.updateGeyserConfig(
            address: addr.isEmpty ? nil : addr,
            port: portInt,
            baseURL: baseURL,
            token: token
        )
        isGeyserSaving = false
        let ok = result?.success == true
        withAnimation { geyserToast = ok ? "Saved" : (result?.message == "not_installed" ? "Geyser not installed" : "Save failed") }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { geyserToast = nil }
        }
    }

    private var footerText: some View {
        Text("TempleTech · MSC REMOTE")
            .font(.system(size: 10, weight: .regular, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .frame(maxWidth: .infinity, alignment: .center)
    }
}

// MARK: - Server Files Sheet (P16)

struct ServerFilesRemoteSheet: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    @State private var currentPath: String? = nil
    @State private var isLoading: Bool = false
    @State private var readingPath: String? = nil
    @State private var preview: ServerFileReadResponseDTO? = nil
    @State private var toast: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var response: ServerFilesResponseDTO? { vm.serverFilesResponse }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        locationCard
                        filesCard
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                    .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                    .frame(maxWidth: .infinity)
                }
                .refreshable { await refresh() }

                if let toast {
                    VStack {
                        Spacer()
                        Text(toast)
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
            .navigationTitle("Server Files")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        Task { await refresh() }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                            .foregroundStyle(MSCRemoteStyle.accent)
                    }
                    .disabled(isLoading)
                }
            }
            .task { await refresh() }
            .background(filePreviewAnchor)
        }
    }

    private var locationCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Location")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Button {
                    Task { await navigateBack() }
                } label: {
                    Image(systemName: "chevron.left")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle((response?.parentPath != nil) ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                        .frame(width: 32, height: 32)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                }
                .buttonStyle(.plain)
                .disabled(response?.parentPath == nil || isLoading)

                VStack(alignment: .leading, spacing: 3) {
                    Text(response?.serverName ?? "Active Server")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text((response?.path.isEmpty == false) ? response!.path : "Server Root")
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .lineLimit(1)
                }

                Spacer()
            }
        }
        .mscCard()
    }

    private var filesCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Files", trailing: response.map { "\($0.items.count)" })
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if isLoading {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    ProgressView().scaleEffect(0.8)
                    Text("Loading files...")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, MSCRemoteStyle.spaceLG)
            } else if let response, response.items.isEmpty {
                Text(response.note == nil ? "No files found." : noteText(response.note))
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, MSCRemoteStyle.spaceLG)
            } else if let items = response?.items {
                VStack(spacing: 0) {
                    ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                        fileRow(item)
                        if index < items.count - 1 {
                            Divider().background(MSCRemoteStyle.borderSubtle)
                        }
                    }
                }
            } else {
                Text("Pull to refresh files.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, MSCRemoteStyle.spaceLG)
            }
        }
        .mscCard()
    }

    private func fileRow(_ item: ServerFileItemDTO) -> some View {
        Button {
            Task {
                if item.isDirectory {
                    await openDirectory(item.path)
                } else {
                    await readFile(item)
                }
            }
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Image(systemName: fileIcon(item))
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(item.isDirectory ? MSCRemoteStyle.accent : MSCRemoteStyle.textSecondary)
                    .frame(width: 24)

                VStack(alignment: .leading, spacing: 3) {
                    Text(item.name)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                        .lineLimit(1)
                    Text(fileDetail(item))
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .lineLimit(1)
                }

                Spacer()

                if readingPath == item.path {
                    ProgressView().scaleEffect(0.75)
                } else if item.isDirectory {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                } else if item.isPreviewable == true {
                    Image(systemName: "doc.text.magnifyingglass")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .padding(.vertical, MSCRemoteStyle.spaceSM)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(readingPath != nil || (!item.isDirectory && item.isPreviewable != true))
    }

    private var filePreviewAnchor: some View {
        Color.clear
            .sheet(isPresented: Binding(
                get: { preview != nil },
                set: { if !$0 { preview = nil } }
            )) {
                if let preview {
                    ServerFilePreviewSheet(preview: preview)
                }
            }
    }

    private func refresh() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isLoading = true
        let error = await vm.fetchServerFiles(baseURL: baseURL, token: token, path: currentPath)
        isLoading = false
        if let error { showToast(error) }
    }

    private func navigateBack() async {
        currentPath = response?.parentPath
        await refresh()
    }

    private func openDirectory(_ path: String) async {
        hapticLight()
        currentPath = path
        await refresh()
    }

    private func readFile(_ item: ServerFileItemDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        hapticLight()
        readingPath = item.path
        let error = await vm.readServerFile(baseURL: baseURL, token: token, path: item.path)
        readingPath = nil
        if let error {
            hapticError()
            showToast(error)
        } else if let read = vm.serverFileReadResponse {
            hapticSuccess()
            preview = read
        }
    }

    private func showToast(_ message: String) {
        withAnimation { toast = message }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { toast = nil }
        }
    }

    private func noteText(_ note: String?) -> String {
        switch note {
        case "no_active_server": return "No active server is selected on the Mac."
        case "invalid_path": return "That path is outside the active server folder."
        case "directory_not_found": return "That folder no longer exists."
        default: return note ?? "No files found."
        }
    }

    private func fileIcon(_ item: ServerFileItemDTO) -> String {
        if item.isDirectory { return "folder.fill" }
        switch item.fileExtension ?? "" {
        case "json", "txt", "log": return "doc.text"
        case "yml", "yaml", "properties", "toml", "cfg", "conf": return "slider.horizontal.3"
        case "jar": return "archivebox"
        case "zip": return "doc.zipper"
        case "png", "jpg", "jpeg", "gif": return "photo"
        case "sh": return "terminal"
        default: return "doc"
        }
    }

    private func fileDetail(_ item: ServerFileItemDTO) -> String {
        if item.isDirectory { return "folder" }
        if let size = item.sizeBytes { return formatRemoteBytes(size) }
        return item.fileExtension?.isEmpty == false ? item.fileExtension! : "file"
    }
}

private struct ServerFilePreviewSheet: View {
    let preview: ServerFileReadResponseDTO
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    Text(preview.content ?? "")
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(MSCRemoteStyle.spaceLG)
                        .background(MSCRemoteStyle.bgCard)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous))
                        .overlay(
                            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
                                .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1)
                        )
                        .padding(MSCRemoteStyle.spaceLG)
                }
            }
            .navigationTitle(preview.name ?? "Preview")
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
}

// MARK: - Client Export Sheet (P16)

struct ClientExportSharePayload: Identifiable {
    let id = UUID()
    let items: [Any]
}

struct ClientExportRemoteSheet: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel
    @Environment(\.dismiss) private var dismiss

    @State private var selectedIds: Set<String> = []
    @State private var didSeedSelection: Bool = false
    @State private var isLoading: Bool = false
    @State private var isSharing: Bool = false
    @State private var sharePayload: ClientExportSharePayload? = nil
    @State private var toast: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var response: ClientExportResponseDTO? { vm.clientExportResponse }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        summaryCard
                        itemsCard
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                    .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                    .frame(maxWidth: .infinity)
                }
                .refreshable { await refresh(resetSelection: true) }

                if let toast {
                    VStack {
                        Spacer()
                        Text(toast)
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
            .navigationTitle("Client Export")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        Task { await refresh(resetSelection: true) }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                            .foregroundStyle(MSCRemoteStyle.accent)
                    }
                    .disabled(isLoading)
                }
            }
            .task { await refresh(resetSelection: false) }
            .background(shareSheetAnchor)
        }
    }

    private var summaryCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Share Package")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if isLoading && response == nil {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    ProgressView().scaleEffect(0.8)
                    Text("Loading client requirements...")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, MSCRemoteStyle.spaceLG)
            } else {
                Text(summaryText)
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                remotePrimaryButton(
                    title: isSharing ? "Preparing..." : "Share",
                    icon: "square.and.arrow.up",
                    enabled: canShare && !isSharing,
                    isLoading: isSharing
                ) {
                    Task { await buildSharePayload() }
                }
            }
        }
        .mscCard()
    }

    private var itemsCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Client Items", trailing: response.map { "\($0.items.count)" })
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if let response, response.items.isEmpty {
                Text(emptyText)
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, MSCRemoteStyle.spaceLG)
            } else if let items = response?.items {
                VStack(spacing: 0) {
                    ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                        exportItemRow(item)
                        if index < items.count - 1 {
                            Divider().background(MSCRemoteStyle.borderSubtle)
                        }
                    }
                }
            } else {
                Text("Pull to refresh client export items.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, MSCRemoteStyle.spaceLG)
            }
        }
        .mscCard()
    }

    private func exportItemRow(_ item: ClientExportItemDTO) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Toggle("", isOn: Binding(
                get: { selectedIds.contains(item.id) },
                set: { selected in
                    hapticLight()
                    if selected { selectedIds.insert(item.id) }
                    else { selectedIds.remove(item.id) }
                }
            ))
            .labelsHidden()
            .tint(MSCRemoteStyle.accent)

            VStack(alignment: .leading, spacing: 3) {
                Text(item.displayName)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .lineLimit(1)
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    Text(statusLabel(item.clientStatus))
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(statusColor(item.clientStatus))
                    Text(item.statusSource)
                        .font(.system(size: 10))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }

            Spacer()
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM)
    }

    private var shareSheetAnchor: some View {
        Color.clear
            .sheet(item: $sharePayload) { payload in
                ShareSheet(items: payload.items)
            }
    }

    private var canShare: Bool {
        guard response?.items.isEmpty == false else { return false }
        return !selectedIds.isEmpty
    }

    private var summaryText: String {
        guard let response else { return "Share the client-side files or links players need for this server." }
        switch response.note {
        case "java_addons_only": return "Client export is available for Java servers with mods or plugins."
        case "empty": return response.isPaperLike ? "No client-side plugin components were found." : "No mods were found for this server."
        case "nothing_selected": return "Select at least one item to share."
        default:
            if response.isPaperLike {
                return "Paper-style servers share Modrinth links for client-side components."
            }
            return "Modded servers share a ZIP of the selected client-required JAR files."
        }
    }

    private var emptyText: String {
        switch response?.note {
        case "java_addons_only": return "Switch to a Java mod/plugin server to export client requirements."
        case "empty": return "No client-side items found."
        default: return "No client export items found."
        }
    }

    private func refresh(resetSelection: Bool) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isLoading = true
        let error = await vm.fetchClientExport(baseURL: baseURL, token: token)
        isLoading = false
        if resetSelection { didSeedSelection = false }
        seedSelectionIfNeeded()
        if let error { showToast(error) }
    }

    private func seedSelectionIfNeeded() {
        guard !didSeedSelection, let items = response?.items else { return }
        selectedIds = Set(items.filter(\.selectedByDefault).map(\.id))
        didSeedSelection = true
    }

    private func buildSharePayload() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        hapticLight()
        isSharing = true
        let error = await vm.fetchClientExport(baseURL: baseURL, token: token, selectedIds: Array(selectedIds))
        isSharing = false
        if let error {
            hapticError()
            showToast(error)
            return
        }
        guard let response = vm.clientExportResponse else {
            hapticError()
            showToast("Export failed.")
            return
        }
        if let text = response.shareText, !text.isEmpty {
            hapticSuccess()
            sharePayload = ClientExportSharePayload(items: [text])
            return
        }
        if let zipBase64 = response.zipBase64,
           let data = Data(base64Encoded: zipBase64),
           let filename = response.zipFileName {
            let url = FileManager.default.temporaryDirectory.appendingPathComponent(filename)
            do {
                try data.write(to: url, options: .atomic)
                hapticSuccess()
                sharePayload = ClientExportSharePayload(items: [url])
            } catch {
                hapticError()
                showToast("Could not prepare share file.")
            }
            return
        }
        hapticError()
        showToast(response.note == "nothing_selected" ? "Select at least one item." : "Nothing to share.")
    }

    private func showToast(_ message: String) {
        withAnimation { toast = message }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { toast = nil }
        }
    }

    private func statusLabel(_ status: String) -> String {
        switch status {
        case "required": return "Required"
        case "optional": return "Optional"
        case "server_only": return "Server-only"
        default: return "Unknown"
        }
    }

    private func statusColor(_ status: String) -> Color {
        switch status {
        case "required": return MSCRemoteStyle.warning
        case "optional": return MSCRemoteStyle.accent
        case "server_only": return MSCRemoteStyle.textTertiary
        default: return MSCRemoteStyle.textSecondary
        }
    }
}

func remotePrimaryButton(title: String, icon: String, enabled: Bool, isLoading: Bool, action: @escaping () -> Void) -> some View {
    Button(action: action) {
        HStack(spacing: MSCRemoteStyle.spaceSM) {
            if isLoading {
                ProgressView().scaleEffect(0.8)
            } else {
                Image(systemName: icon)
                    .font(.system(size: 12, weight: .semibold))
            }
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

private func formatRemoteBytes(_ bytes: Int64) -> String {
    let kb = Double(bytes) / 1024
    let mb = kb / 1024
    if mb >= 1 { return String(format: "%.1f MB", mb) }
    if kb >= 1 { return String(format: "%.0f KB", kb) }
    return "\(bytes) B"
}
