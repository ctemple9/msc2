import SwiftUI

// MARK: - Inline level/tag inference
//
// The ConsoleLineDTO.level field from the server is unreliable — it is often
// nil, or set to a value that doesn't match what's visually in the text.
// The macOS ConsoleManager infers level and tag by scanning the raw text.
// We port that same logic here so iOS filtering matches macOS behaviour.

private func inferredLevel(from text: String) -> String {
    let upper = text.uppercased()
    if upper.contains(" ERROR") || upper.contains(" SEVERE") || upper.contains("EXCEPTION") || upper.contains("JAVA.LANG.") { return "ERROR" }
    if upper.contains(" WARN")  { return "WARN"  }
    if upper.contains(" INFO")  { return "INFO"  }
    return "OTHER"
}

/// Returns the plugin/component tag from a server line, or nil if none.
/// Mirrors ConsoleManager.inferTag + extractBracketTokens + isCoreServerTag.
private func inferredTag(from text: String) -> String? {
    // Extract all [token] groups
    var tokens: [String] = []
    var current = ""
    var inBracket = false
    for ch in text {
        if ch == "[" { inBracket = true; current = ""; continue }
        if ch == "]" { if inBracket { tokens.append(current) }; inBracket = false; current = ""; continue }
        if inBracket { current.append(ch) }
    }
    let trimmed = tokens.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }

    // Drop timestamp tokens (contain ":" and digits) and level words
    let filtered = trimmed.filter { tok in
        let hasColon = tok.contains(":")
        let hasDigit = tok.rangeOfCharacter(from: .decimalDigits) != nil
        if hasColon && hasDigit { return false }
        let lower = tok.lowercased()
        return lower != "info" && lower != "warn" && lower != "error"
    }

    return filtered.first
}

private let coreServerTags: Set<String> = ["bootstrap", "minecraftserver", "paper", "server", "main"]

private func isPlugin(_ text: String) -> Bool {
    guard let tag = inferredTag(from: text) else { return false }
    return !coreServerTags.contains(tag.lowercased())
}

private func isAutoLine(_ text: String) -> Bool {
    if text.contains("[Auto →") || text.contains("[Auto ->") { return true }
    let lower = text.lowercased()
    if lower.contains("tps from last 1m, 5m, 15m") { return true }
    // Forge/NeoForge `forge tps` reply: "Overall: … Mean tick time: X ms. Mean TPS: Y"
    if lower.contains("mean tick time") && lower.contains("mean tps") { return true }
    if lower.contains("there are") && lower.contains("players online") { return true }
    return false
}

// MARK: - ConsoleView

struct ConsoleView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var tailN: Int = 200
    @State private var showLive: Bool = false

    // MARK: Filter state

    @State private var activeSourceChips: Set<String> = []
    @State private var activeLevelChips: Set<String>  = []
    @State private var pluginsOnly: Bool  = false
    @State private var hideAuto:    Bool  = false
    @State private var searchText:  String = ""
    @State private var showSearch:  Bool   = false
    @FocusState private var searchFocused: Bool

    // MARK: Quick send

    private var quickSendChips: [CommandTemplate] {
        let allCommands = CommandCatalog.defaultGroups.flatMap { $0.commands }
        let commandByString = Dictionary(uniqueKeysWithValues: allCommands.map { ($0.command, $0) })
        var seen = Set<String>()
        var chips: [CommandTemplate] = []
        for cmd in settings.favoriteCommands {
            guard chips.count < 5 else { break }
            if let template = commandByString[cmd], seen.insert(cmd).inserted { chips.append(template) }
        }
        for cmd in settings.recentCommands {
            guard chips.count < 5 else { break }
            if let template = commandByString[cmd], seen.insert(cmd).inserted { chips.append(template) }
        }
        return chips
    }

    // MARK: Quick command state

    @State private var selectedDifficulty: String = "normal"
    @State private var selectedGamemode: String = "survival"

    // MARK: Command state

    @State private var commandText: String = ""
    @State private var showCommandPicker: Bool = false
    @State private var showDangerConfirm: Bool = false
    @State private var dangerCommandToConfirm: String = ""
    @State private var lastSentChip: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken:   URL? { nil }   // unused — kept for symmetry
    private var isPaired: Bool {
        settings.resolvedBaseURL() != nil && settings.resolvedToken() != nil
    }

    private var sourceLines: [ConsoleLineDTO] {
        showLive ? vm.consoleStream : vm.consoleTail
    }

    private var isAnyFilterActive: Bool {
        !activeSourceChips.isEmpty || !activeLevelChips.isEmpty ||
        pluginsOnly || hideAuto ||
        !searchText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    // The "app" source chip is only worth showing if there are app-source lines
    private var hasAppLines:    Bool { sourceLines.contains { $0.source == "app"    } }
    private var hasServerLines: Bool { sourceLines.contains { $0.source == "server" } }
    private var hasAutoLines:   Bool { sourceLines.contains { isAutoLine($0.text)   } }
    private var hasPluginLines: Bool { sourceLines.contains { $0.source == "server" && isPlugin($0.text) } }

    /// Final visible line set — derived inline, never cached.
    private var displayLines: [ConsoleLineDTO] {
        var lines = sourceLines

        // Source filter
        if !activeSourceChips.isEmpty {
            lines = lines.filter { activeSourceChips.contains($0.source) }
        }

        // Level filter — inferred from text, not the DTO field
        if !activeLevelChips.isEmpty {
            lines = lines.filter { activeLevelChips.contains(inferredLevel(from: $0.text)) }
        }

        // Plugins filter — server lines whose tag is NOT a core server component
        if pluginsOnly {
            lines = lines.filter { $0.source == "server" && isPlugin($0.text) }
        }

        // Hide auto — suppress polling noise and known auto-response patterns
        if hideAuto {
            lines = lines.filter { !isAutoLine($0.text) }
        }

        // Text search
        let term = searchText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if !term.isEmpty {
            lines = lines.filter { $0.text.lowercased().contains(term) }
        }

        return lines
    }

    // MARK: - Body

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                VStack(spacing: 0) {
                    ScrollView(showsIndicators: false) {
                        VStack(spacing: MSCRemoteStyle.spaceLG) {
                            quickActionsCard
                            consoleOutputCard
                        }
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        .padding(.top,    MSCRemoteStyle.spaceMD)
                        .padding(.bottom, MSCRemoteStyle.spaceLG)
                    }
                    .refreshable { await fetchTail() }

                    if vm.connectedRole != "guest" {
                        commandInputBar
                    }
                    footerText.padding(.vertical, 6)
                }
            }
            .navigationTitle("Console")
            .navigationBarTitleDisplayMode(.large)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .sheet(isPresented: $showCommandPicker) {
                CommandPickerSheet(commandText: $commandText)
                    .environmentObject(vm)
            }
            .alert("Confirm Command", isPresented: $showDangerConfirm) {
                Button("Cancel", role: .cancel) { }
                Button("Send", role: .destructive) {
                    Task { await sendCommandString(dangerCommandToConfirm) }
                }
            } message: {
                Text("This command can disrupt the server or players:\n\n\(dangerCommandToConfirm)\n\nAre you sure?")
            }
            .onDisappear { vm.disconnectConsoleStream(); showLive = false }
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.willEnterForegroundNotification)) { _ in
                guard isPaired, showLive else { return }
                Task { vm.disconnectConsoleStream(); await connectStream() }
            }
            .onChange(of: showLive) { _, _ in resetFilters() }
        }
    }

    // MARK: - Quick Actions Card

    private var quickActionsCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceLG) {
            timeWeatherSection
            Divider().background(MSCRemoteStyle.borderSubtle)
            gamemodeSection
            if !quickSendChips.isEmpty {
                Divider().background(MSCRemoteStyle.borderSubtle)
                quickSendSection
            }
        }
        .mscCard()
    }

    private var timeWeatherSection: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            MSCSectionHeader(title: "Time of Day")
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                quickIconButton(title: "Dawn",  icon: "sunrise.fill",       tint: MSCRemoteStyle.cmdGameplay,     bgTint: MSCRemoteStyle.cmdGameplay.opacity(0.12),     command: "/time set 1000")
                quickIconButton(title: "Dusk",  icon: "sunset.fill",        tint: MSCRemoteStyle.cmdEnvironment,  bgTint: MSCRemoteStyle.cmdEnvironment.opacity(0.12),  command: "/time set 13000")
                quickIconButton(title: "Night", icon: "moon.stars.fill",    tint: MSCRemoteStyle.cmdPlayer,       bgTint: MSCRemoteStyle.cmdPlayer.opacity(0.12),       command: "/time set 18000")
            }
            MSCSectionHeader(title: "Weather")
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                quickIconButton(title: "Clear", icon: "sun.max.fill",          tint: MSCRemoteStyle.cmdGameplay,    bgTint: MSCRemoteStyle.cmdGameplay.opacity(0.12),    command: "/weather clear")
                quickIconButton(title: "Rain",  icon: "cloud.rain.fill",       tint: MSCRemoteStyle.cmdModeration, bgTint: MSCRemoteStyle.cmdModeration.opacity(0.12), command: "/weather rain")
                quickIconButton(title: "Storm", icon: "cloud.bolt.rain.fill",  tint: MSCRemoteStyle.cmdServer,     bgTint: MSCRemoteStyle.cmdServer.opacity(0.12),     command: "/weather thunder")
            }
        }
    }

    private var gamemodeSection: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Settings")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            VStack(spacing: 0) {
                settingsRow {
                    Text("Difficulty")
                        .font(.system(size: 14))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Spacer()
                    Picker("Difficulty", selection: $selectedDifficulty) {
                        Text("Peaceful").tag("peaceful")
                        Text("Easy").tag("easy")
                        Text("Normal").tag("normal")
                        Text("Hard").tag("hard")
                    }
                    .pickerStyle(.menu)
                    .tint(MSCRemoteStyle.accent)
                    .onChange(of: selectedDifficulty) { _, new in
                        guard isPaired else { return }
                        hapticLight()
                        Task { await sendCommandString("/difficulty \(new)") }
                    }
                }

                Divider().background(MSCRemoteStyle.borderSubtle)

                settingsRow {
                    Text("Gamemode")
                        .font(.system(size: 14))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Spacer()
                    Picker("Gamemode", selection: $selectedGamemode) {
                        Text("Survival").tag("survival")
                        Text("Creative").tag("creative")
                        Text("Adventure").tag("adventure")
                        Text("Spectator").tag("spectator")
                    }
                    .pickerStyle(.menu)
                    .tint(MSCRemoteStyle.accent)
                    .onChange(of: selectedGamemode) { _, new in
                        guard isPaired else { return }
                        hapticLight()
                        Task { await sendCommandString("/defaultgamemode \(new)") }
                    }
                }
            }
        }
    }

    private func quickIconButton(title: String, icon: String, tint: Color, bgTint: Color, command: String) -> some View {
        let isSent = lastSentChip == command
        return Button {
            guard isPaired else { return }
            hapticLight()
            withAnimation(.easeInOut(duration: 0.12)) { lastSentChip = command }
            Task {
                await sendCommandString(command)
                try? await Task.sleep(nanoseconds: 600_000_000)
                await MainActor.run { withAnimation(.easeInOut(duration: 0.2)) { lastSentChip = nil } }
            }
        } label: {
            VStack(spacing: 6) {
                Image(systemName: icon)
                    .font(.system(size: 22))
                    .foregroundStyle(isSent ? MSCRemoteStyle.bgBase : tint)
                Text(title)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(isSent ? MSCRemoteStyle.bgBase : MSCRemoteStyle.textPrimary)
            }
            .frame(maxWidth: .infinity)
            .frame(height: 64)
            .background(isSent ? tint : bgTint)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                .strokeBorder(isSent ? tint : tint.opacity(0.25), lineWidth: 1))
            .animation(.easeInOut(duration: 0.15), value: isSent)
        }
        .disabled(!isPaired)
        .opacity(isPaired ? 1 : 0.45)
    }

    private func settingsRow<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceSM) { content() }
            .padding(.vertical, MSCRemoteStyle.spaceSM)
    }

    // MARK: - Quick Send

    private var quickSendSection: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                MSCSectionHeader(title: "Quick Send")
                Spacer()
                Text("★ Star to pin")
                    .font(.system(size: 9, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    ForEach(quickSendChips) { chip in
                        quickSendChip(chip)
                    }
                }
                .padding(.horizontal, 2)
                .padding(.vertical, 2)
            }
        }
    }

    private func quickSendChip(_ chip: CommandTemplate) -> some View {
        let isSent = lastSentChip == chip.command
        return Button {
            guard isPaired else { return }
            hapticLight()
            settings.recordRecent(command: chip.command)
            if isDangerousCommand(chip.command) {
                dangerCommandToConfirm = chip.command
                showDangerConfirm = true
                return
            }
            withAnimation(.easeInOut(duration: 0.12)) { lastSentChip = chip.command }
            Task {
                await sendCommandString(chip.command)
                try? await Task.sleep(nanoseconds: 800_000_000)
                await MainActor.run { withAnimation(.easeInOut(duration: 0.2)) { lastSentChip = nil } }
            }
        } label: {
            VStack(alignment: .leading, spacing: 4) {
                Text(chip.title)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(isSent ? MSCRemoteStyle.bgBase : (isPaired ? MSCRemoteStyle.textPrimary : MSCRemoteStyle.textTertiary))
                    .lineLimit(1)
                Text(chip.command)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(isSent ? MSCRemoteStyle.bgBase.opacity(0.7) : MSCRemoteStyle.textTertiary)
                    .lineLimit(1)
            }
            .padding(.horizontal, MSCRemoteStyle.spaceMD)
            .padding(.vertical, MSCRemoteStyle.spaceSM)
            .background(isSent ? MSCRemoteStyle.success : (isPaired ? MSCRemoteStyle.bgElevated : MSCRemoteStyle.bgElevated.opacity(0.5)))
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(isSent ? MSCRemoteStyle.success : (isPaired ? MSCRemoteStyle.borderMid : MSCRemoteStyle.borderSubtle), lineWidth: 1)
            )
            .animation(.easeInOut(duration: 0.15), value: isSent)
        }
        .disabled(!isPaired)
    }

    // MARK: - Command Input Bar

    private var commandInputBar: some View {
        VStack(spacing: 0) {
            Divider().background(MSCRemoteStyle.borderMid)

            HStack(spacing: MSCRemoteStyle.spaceSM) {
                Text(">")
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.accent)

                TextField("Command…", text: $commandText)
                    .font(.system(size: 14, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled(true)
                    .submitLabel(.send)
                    .onSubmit { onSendTapped() }

                Button {
                    hapticLight()
                    showCommandPicker = true
                } label: {
                    Image(systemName: "list.bullet")
                        .font(.system(size: 16, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                        .frame(width: 36, height: 36)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                }
                .buttonStyle(.plain)

                Button {
                    onSendTapped()
                } label: {
                    Image(systemName: "paperplane.fill")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(
                            isPaired && !commandText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                ? MSCRemoteStyle.bgBase : MSCRemoteStyle.textTertiary
                        )
                        .frame(width: 36, height: 36)
                        .background(
                            isPaired && !commandText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                ? MSCRemoteStyle.accent : MSCRemoteStyle.bgElevated
                        )
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                }
                .buttonStyle(.plain)
                .disabled(!isPaired || commandText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            .padding(.horizontal, MSCRemoteStyle.spaceLG)
            .padding(.vertical, MSCRemoteStyle.spaceMD)
            .background(MSCRemoteStyle.bgBase)
        }
    }

    // MARK: - Command Actions

    private func onSendTapped() {
        guard isPaired else { return }
        let cmd = commandText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !cmd.isEmpty else { return }
        hapticLight()
        if isDangerousCommand(cmd) {
            dangerCommandToConfirm = cmd
            showDangerConfirm = true
            return
        }
        Task { await sendCommandString(cmd) }
    }

    private func sendCommandString(_ cmd: String) async {
        guard let baseURL = settings.resolvedBaseURL(), let token = settings.resolvedToken() else { return }
        let trimmed = cmd.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        commandText = ""
        let ok = await vm.sendCommand(baseURL: baseURL, token: token, command: trimmed)
        if ok { hapticSuccess() } else { hapticError() }
    }

    private func isDangerousCommand(_ cmd: String) -> Bool {
        let s = cmd.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !s.isEmpty else { return false }
        let parts = s.split(whereSeparator: { $0.isWhitespace })
        guard let first = parts.first else { return false }
        let root = String(first)
        switch root {
        case "/stop","stop","/reload","reload","/ban","ban","/ban-ip","ban-ip",
             "/pardon","pardon","/pardon-ip","pardon-ip","/kick","kick","/op","op","/deop","deop":
            return true
        case "/whitelist","whitelist":
            if parts.count >= 2 { let sub = String(parts[1]); if sub == "off" || sub == "remove" { return true } }
            return false
        default: return false
        }
    }

    // MARK: - Console Output Card

    private var consoleOutputCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            let filtered = displayLines
            let total    = sourceLines.count

            // ── Header ─────────────────────────────────────────────────────
            HStack(alignment: .center) {
                Text(showLive ? "LIVE" : "TAIL")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(showLive && vm.isStreamingConsole ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                    .kerning(1.2)
                Spacer()
                if total > 0 {
                    Text(isAnyFilterActive ? "\(filtered.count) of \(total)" : "\(total)")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(isAnyFilterActive ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                }
                // Search toggle
                Button {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        showSearch.toggle()
                        if !showSearch { searchText = "" }
                    }
                } label: {
                    Image(systemName: showSearch ? "magnifyingglass.circle.fill" : "magnifyingglass")
                        .font(.system(size: 14))
                        .foregroundStyle(showSearch ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                }
                .buttonStyle(.plain)
                .padding(.leading, MSCRemoteStyle.spaceSM)
                // Copy visible
                Button { copyVisibleLines(filtered) } label: {
                    Image(systemName: "doc.on.doc")
                        .font(.system(size: 13))
                        .foregroundStyle(filtered.isEmpty ? MSCRemoteStyle.textTertiary.opacity(0.3) : MSCRemoteStyle.textTertiary)
                }
                .buttonStyle(.plain)
                .disabled(filtered.isEmpty)
                .padding(.leading, MSCRemoteStyle.spaceSM)
                // Reset — only when a filter is active
                if isAnyFilterActive {
                    Button { withAnimation(.easeInOut(duration: 0.15)) { resetFilters() } } label: {
                        Image(systemName: "xmark.circle.fill")
                            .font(.system(size: 14))
                            .foregroundStyle(MSCRemoteStyle.warning)
                    }
                    .buttonStyle(.plain)
                    .padding(.leading, MSCRemoteStyle.spaceSM)
                    .transition(.opacity.combined(with: .scale(scale: 0.8)))
                }
            }
            .padding(.horizontal, 2)
            .padding(.bottom, MSCRemoteStyle.spaceSM)

            terminalControlRow
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            // ── Search bar ─────────────────────────────────────────────────
            if showSearch {
                searchBar
                    .padding(.bottom, MSCRemoteStyle.spaceMD)
                    .transition(.move(edge: .top).combined(with: .opacity))
            }

            // ── Single chip row ────────────────────────────────────────────
            // All filter chips live on one horizontal scroll row, no labels.
            // Chips only appear when the data actually contains lines they
            // would act on, preventing phantom filters.
            if total > 0 {
                filterChipStrip
                    .padding(.bottom, MSCRemoteStyle.spaceMD)
            }

            // ── Terminal box ───────────────────────────────────────────────
            ZStack(alignment: .topLeading) {
                Color(hex: "#0A0C0E")
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))

                if filtered.isEmpty {
                    VStack(alignment: .leading, spacing: 6) {
                        if isAnyFilterActive && total > 0 {
                            Text("No lines match the active filters.")
                                .font(.system(size: 12, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                            Text("Tap \u{00D7} above to reset.")
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textTertiary.opacity(0.6))
                        } else {
                            Text(showLive ? "Waiting for live output…" : "No lines loaded yet.")
                                .font(.system(size: 12, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                            Text(showLive ? "Turn on the toggle above." : "Tap Fetch to load a snapshot.")
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundStyle(MSCRemoteStyle.textTertiary.opacity(0.6))
                        }
                    }
                    .padding(MSCRemoteStyle.spaceMD)
                } else {
                    ScrollViewReader { proxy in
                        ScrollView(showsIndicators: false) {
                            LazyVStack(alignment: .leading, spacing: 6) {
                                ForEach(filtered) { line in
                                    consoleLine(line).id(line.id)
                                }
                            }
                            .padding(MSCRemoteStyle.spaceMD)
                        }
                        .onChange(of: vm.consoleStream.count) { _, _ in
                            guard showLive, let last = filtered.last else { return }
                            withAnimation { proxy.scrollTo(last.id, anchor: .bottom) }
                        }
                    }
                }
            }
            .frame(maxHeight: 360)
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1)
            )
        }
        .mscCard()
    }

    private var terminalControlRow: some View {
        VStack(spacing: MSCRemoteStyle.spaceSM) {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    Circle()
                        .fill(vm.isStreamingConsole ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                        .frame(width: 7, height: 7)
                    Text(vm.isStreamingConsole ? "Connected" : "Disconnected")
                        .font(.system(size: 11, weight: .semibold, design: .monospaced))
                        .foregroundStyle(vm.isStreamingConsole ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                }

                Spacer()

                Text("Live")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                Toggle("", isOn: $showLive)
                    .labelsHidden()
                    .tint(MSCRemoteStyle.accent)
                    .onChange(of: showLive) { _, newValue in
                        if newValue { Task { await connectStream() } }
                        else { vm.disconnectConsoleStream() }
                    }
                    .disabled(!isPaired)
            }

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Stepper(value: $tailN, in: 20...500, step: 20) {
                    Text("Lines: \(tailN)")
                        .font(.system(size: 12, design: .rounded))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                }
                .tint(MSCRemoteStyle.accent)

                Spacer(minLength: MSCRemoteStyle.spaceSM)

                Button {
                    hapticLight()
                    if showLive { vm.trimConsoleStream(to: tailN) }
                    else { Task { await fetchTail() } }
                } label: {
                    Text(showLive ? "Trim" : "Fetch")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(isPaired ? MSCRemoteStyle.bgBase : MSCRemoteStyle.textTertiary)
                        .padding(.horizontal, MSCRemoteStyle.spaceMD)
                        .frame(height: 30)
                        .background(isPaired ? MSCRemoteStyle.accent : MSCRemoteStyle.bgElevated)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                }
                .buttonStyle(.plain)
                .disabled(!isPaired)
            }
        }
        .padding(MSCRemoteStyle.spaceSM)
        .background(MSCRemoteStyle.bgElevated.opacity(0.55))
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1)
        )
    }

    // MARK: - Filter chip strip

    private var filterChipStrip: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: MSCRemoteStyle.spaceSM) {

                // SOURCE chips — only show sources that actually have lines
                if hasAppLines {
                    chip("APP",
                         isActive: activeSourceChips.contains("app"),
                         accent: MSCRemoteStyle.accent) {
                        toggle(&activeSourceChips, value: "app")
                    }
                }
                if hasServerLines {
                    chip("SERVER",
                         isActive: activeSourceChips.contains("server"),
                         accent: MSCRemoteStyle.accent) {
                        toggle(&activeSourceChips, value: "server")
                    }
                }

                // Subtle divider between source and level groups
                if (hasAppLines || hasServerLines) {
                    Divider()
                        .frame(height: 16)
                        .background(MSCRemoteStyle.borderMid)
                        .padding(.horizontal, 2)
                }

                // LEVEL chips — always shown so user can see what's available
                chip("INFO",
                     isActive: activeLevelChips.contains("INFO"),
                     accent: MSCRemoteStyle.accent) {
                    toggle(&activeLevelChips, value: "INFO")
                }
                chip("WARN",
                     isActive: activeLevelChips.contains("WARN"),
                     accent: MSCRemoteStyle.warning) {
                    toggle(&activeLevelChips, value: "WARN")
                }
                chip("ERROR",
                     isActive: activeLevelChips.contains("ERROR"),
                     accent: MSCRemoteStyle.danger) {
                    toggle(&activeLevelChips, value: "ERROR")
                }

                Divider()
                    .frame(height: 16)
                    .background(MSCRemoteStyle.borderMid)
                    .padding(.horizontal, 2)

                // PLUGINS chip — only shown when plugin lines exist
                if hasPluginLines {
                    chip("PLUGINS",
                         isActive: pluginsOnly,
                         accent: MSCRemoteStyle.accent) {
                        withAnimation(.easeInOut(duration: 0.15)) { pluginsOnly.toggle() }
                    }
                }

                // HIDE AUTO chip — only shown when auto lines exist
                if hasAutoLines {
                    chip("HIDE AUTO",
                         isActive: hideAuto,
                         accent: MSCRemoteStyle.warning) {
                        withAnimation(.easeInOut(duration: 0.15)) { hideAuto.toggle() }
                    }
                }
            }
            .padding(.horizontal, 2)
            .padding(.vertical, 2)
        }
    }

    // MARK: - Chip component

    private func chip(
        _ label: String,
        isActive: Bool,
        accent: Color,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Text(label)
                .font(.system(size: 9, weight: .semibold, design: .monospaced))
                .kerning(0.5)
                .foregroundStyle(isActive ? MSCRemoteStyle.bgBase : MSCRemoteStyle.textSecondary)
                .padding(.horizontal, MSCRemoteStyle.spaceSM)
                .padding(.vertical, 5)
                .background(isActive ? accent : MSCRemoteStyle.bgElevated)
                .clipShape(Capsule())
                .overlay(Capsule().strokeBorder(isActive ? accent : MSCRemoteStyle.borderMid, lineWidth: 1))
        }
        .buttonStyle(.plain)
        .animation(.easeInOut(duration: 0.15), value: isActive)
    }

    private func toggle(_ set: inout Set<String>, value: String) {
        withAnimation(.easeInOut(duration: 0.15)) {
            if set.contains(value) { set.remove(value) } else { set.insert(value) }
        }
    }

    // MARK: - Search bar

    private var searchBar: some View {
        HStack(spacing: MSCRemoteStyle.spaceSM) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(searchFocused ? MSCRemoteStyle.accent : MSCRemoteStyle.textTertiary)
                .animation(.easeInOut(duration: 0.15), value: searchFocused)

            TextField("Search output…", text: $searchText)
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .tint(MSCRemoteStyle.accent)
                .focused($searchFocused)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)

            if !searchText.isEmpty {
                Button { searchText = "" } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 14))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, MSCRemoteStyle.spaceMD)
        .padding(.vertical, MSCRemoteStyle.spaceSM)
        .background(MSCRemoteStyle.bgElevated)
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                .strokeBorder(
                    searchFocused ? MSCRemoteStyle.accent.opacity(0.45) : MSCRemoteStyle.borderMid,
                    lineWidth: 1
                )
                .animation(.easeInOut(duration: 0.15), value: searchFocused)
        )
    }

    // MARK: - Console line row

    @ViewBuilder
    private func consoleLine(_ line: ConsoleLineDTO) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("\(formatTimestamp(line.ts))  [\(line.source)]")
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .lineLimit(1)
            Text(line.text)
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(lineColor(for: line))
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    // MARK: - Timestamp

    private static let isoParser: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter(); f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]; return f
    }()
    private static let isoParserNoFraction: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter(); f.formatOptions = [.withInternetDateTime]; return f
    }()
    private static let timeFormatter: DateFormatter = {
        let f = DateFormatter(); f.dateFormat = "h:mm:ss a"; f.amSymbol = "AM"; f.pmSymbol = "PM"; return f
    }()

    private func formatTimestamp(_ raw: String) -> String {
        if let d = Self.isoParser.date(from: raw)           { return Self.timeFormatter.string(from: d) }
        if let d = Self.isoParserNoFraction.date(from: raw) { return Self.timeFormatter.string(from: d) }
        return raw
    }

    // MARK: - Line color (by inferred level for consistency with filter)

    private func lineColor(for line: ConsoleLineDTO) -> Color {
        if line.source == "app" { return Color.white.opacity(0.45) }
        switch inferredLevel(from: line.text) {
        case "WARN":  return MSCRemoteStyle.warning
        case "ERROR": return MSCRemoteStyle.danger
        default:      return Color.white.opacity(0.75)
        }
    }

    // MARK: - Helpers

    private func resetFilters() {
        activeSourceChips.removeAll()
        activeLevelChips.removeAll()
        pluginsOnly  = false
        hideAuto     = false
        searchText   = ""
        showSearch   = false
    }

    private func copyVisibleLines(_ lines: [ConsoleLineDTO]) {
        UIPasteboard.general.string = lines.map { $0.text }.joined(separator: "\n")
        hapticSuccess()
    }

    private var footerText: some View {
        Text("TempleTech · MSC REMOTE")
            .font(.system(size: 10, weight: .regular, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .frame(maxWidth: .infinity, alignment: .center)
    }

    private func fetchTail() async {
        guard let baseURL = settings.resolvedBaseURL(), let token = settings.resolvedToken() else { return }
        await vm.refreshAll(baseURL: baseURL, token: token, tailN: tailN)
    }

    private func connectStream() async {
        guard let baseURL = settings.resolvedBaseURL(), let token = settings.resolvedToken() else { return }
        await vm.connectConsoleStream(baseURL: baseURL, token: token)
    }
}
