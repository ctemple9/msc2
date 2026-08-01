import SwiftUI

struct CommandsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var commandText: String = ""
    @State private var showCommandPicker: Bool = false
    @State private var showDangerConfirm: Bool = false
    @State private var dangerCommandToConfirm: String = ""
    @State private var lastSentChip: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String?  { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }

    // MARK: - Quick-send chip data
    //
    // Priority: favorites first, then recents as fill-in.
    // Capped at 5 entries — enough depth without overwhelming the row.

    private var quickSendChips: [CommandTemplate] {
        let allCommands = CommandCatalog.defaultGroups.flatMap { $0.commands }
        let commandByString = Dictionary(uniqueKeysWithValues: allCommands.map { ($0.command, $0) })

        var seen = Set<String>()
        var chips: [CommandTemplate] = []

        for cmd in settings.favoriteCommands {
            guard chips.count < 5 else { break }
            if let template = commandByString[cmd], seen.insert(cmd).inserted {
                chips.append(template)
            }
        }

        for cmd in settings.recentCommands {
            guard chips.count < 5 else { break }
            if let template = commandByString[cmd], seen.insert(cmd).inserted {
                chips.append(template)
            }
        }

        return chips
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        if !quickSendChips.isEmpty {
                            quickSendCard
                        }
                        commandInputCard
                        timeWeatherCard
                        gamemodeCard
                        if let err = vm.errorMessage, !err.isEmpty {
                            errorBanner(err)
                        }
                        footerText
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.space2XL)
                }
            }
            .navigationTitle("Commands")
            .navigationBarTitleDisplayMode(.large)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .sheet(isPresented: $showCommandPicker) {
                CommandPickerSheet(commandText: $commandText)
            }
            .alert("Confirm Command", isPresented: $showDangerConfirm) {
                Button("Cancel", role: .cancel) { }
                Button("Send", role: .destructive) {
                    Task { await sendCommandString(dangerCommandToConfirm) }
                }
            } message: {
                Text("This command can disrupt the server or players:\n\n\(dangerCommandToConfirm)\n\nAre you sure?")
            }
        }
    }

    // MARK: - Quick Send Card

    private var quickSendCard: some View {
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

            if !isPaired {
                HStack(spacing: 6) {
                    Image(systemName: "lock").font(.system(size: 10))
                    Text("Pair in Settings to enable quick send.")
                        .font(.system(size: 11))
                }
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .padding(.top, MSCRemoteStyle.spaceSM)
            }
        }
        .mscCard()
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
                await MainActor.run {
                    withAnimation(.easeInOut(duration: 0.2)) { lastSentChip = nil }
                }
            }
        } label: {
            VStack(alignment: .leading, spacing: 4) {
                Text(chip.title)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(
                        isSent
                            ? MSCRemoteStyle.bgBase
                            : (isPaired ? MSCRemoteStyle.textPrimary : MSCRemoteStyle.textTertiary)
                    )
                    .lineLimit(1)

                Text(chip.command)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(
                        isSent
                            ? MSCRemoteStyle.bgBase.opacity(0.7)
                            : MSCRemoteStyle.textTertiary
                    )
                    .lineLimit(1)
            }
            .padding(.horizontal, MSCRemoteStyle.spaceMD)
            .padding(.vertical, MSCRemoteStyle.spaceSM)
            .background(
                isSent
                    ? MSCRemoteStyle.success
                    : (isPaired ? MSCRemoteStyle.bgElevated : MSCRemoteStyle.bgElevated.opacity(0.5))
            )
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(
                        isSent
                            ? MSCRemoteStyle.success
                            : (isPaired ? MSCRemoteStyle.borderMid : MSCRemoteStyle.borderSubtle),
                        lineWidth: 1
                    )
            )
            .animation(.easeInOut(duration: 0.15), value: isSent)
        }
        .disabled(!isPaired)
    }

    // MARK: - Command Input Card

    private var commandInputCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Send Command")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(spacing: MSCRemoteStyle.spaceSM) {
                Text(">")
                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.accent)

                TextField("Type a command…", text: $commandText)
                    .font(.system(size: 14, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled(true)
                    .submitLabel(.send)
                    .onSubmit { onSendTapped() }
            }
            .padding(MSCRemoteStyle.spaceMD)
            .background(MSCRemoteStyle.bgDeep)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
            )
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Button {
                    hapticLight()
                    showCommandPicker = true
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "list.bullet")
                        Text("Browse")
                    }
                    .font(.system(size: 14, weight: .semibold))
                    .frame(maxWidth: .infinity)
                    .frame(height: 44)
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .background(MSCRemoteStyle.bgElevated)
                    .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                            .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
                    )
                }

                MSCActionButton(
                    title: "Send",
                    icon: "paperplane.fill",
                    style: .primary,
                    isEnabled: isPaired && !commandText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ) {
                    onSendTapped()
                }
            }

            if !isPaired {
                HStack(spacing: 6) {
                    Image(systemName: "lock").font(.system(size: 11))
                    Text("Pair in Settings to enable commands.")
                        .font(.system(size: 12))
                }
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .padding(.top, MSCRemoteStyle.spaceMD)
            }
        }
        .mscCard()
    }

    // MARK: - Time & Weather Card
    //
    // Mirrors the macOS Quick Commands grid: visual buttons grouped by
    // category with SF Symbol icons and tinted backgrounds.
    // Buttons send immediately with the same danger-check + confirm path.
    // The "Dawn" / "Dusk" distinction is intentional — partial-day values
    // (1000 and 13000) give a visually distinct result without hitting noon
    // or midnight, which matters for atmospheric screenshots.

    private var timeWeatherCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceMD) {
            MSCSectionHeader(title: "Time of Day")

            HStack(spacing: MSCRemoteStyle.spaceSM) {
                quickIconButton(
                    title: "Dawn",
                    icon: "sunrise.fill",
                    tint: MSCRemoteStyle.cmdGameplay,
                    bgTint: MSCRemoteStyle.cmdGameplay.opacity(0.12),
                    command: "/time set 1000"
                )
                quickIconButton(
                    title: "Dusk",
                    icon: "sunset.fill",
                    tint: MSCRemoteStyle.cmdEnvironment,
                    bgTint: MSCRemoteStyle.cmdEnvironment.opacity(0.12),
                    command: "/time set 13000"
                )
                quickIconButton(
                    title: "Night",
                    icon: "moon.stars.fill",
                    tint: MSCRemoteStyle.cmdPlayer,
                    bgTint: MSCRemoteStyle.cmdPlayer.opacity(0.12),
                    command: "/time set 18000"
                )
            }

            MSCSectionHeader(title: "Weather")

            HStack(spacing: MSCRemoteStyle.spaceSM) {
                quickIconButton(
                    title: "Clear",
                    icon: "sun.max.fill",
                    tint: MSCRemoteStyle.cmdGameplay,
                    bgTint: MSCRemoteStyle.cmdGameplay.opacity(0.12),
                    command: "/weather clear"
                )
                quickIconButton(
                    title: "Rain",
                    icon: "cloud.rain.fill",
                    tint: MSCRemoteStyle.cmdModeration,
                    bgTint: MSCRemoteStyle.cmdModeration.opacity(0.12),
                    command: "/weather rain"
                )
                quickIconButton(
                    title: "Storm",
                    icon: "cloud.bolt.rain.fill",
                    tint: MSCRemoteStyle.cmdServer,
                    bgTint: MSCRemoteStyle.cmdServer.opacity(0.12),
                    command: "/weather thunder"
                )
            }
        }
        .mscCard()
    }

    // MARK: - Gamemode Card
    //
    // Settings-style section with a label column and a picker/toggle column,
    // matching the macOS design's segmented pickers for Difficulty, Gamemode,
    // and Whitelist. On iOS, Picker with .menu style is the idiomatic equivalent.
    // The commands are sent the moment the user selects a new value — the
    // confirmation requirement is deliberately NOT applied here because these
    // are global server settings, not player-targeting commands. A separate
    // design decision could add confirmation; for now, the visual intent is
    // speed and directness, matching macOS.

    @State private var selectedDifficulty: String = "easy"
    @State private var selectedGamemode: String = "survival"

    private var gamemodeCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Settings")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            VStack(spacing: 0) {
                // Difficulty
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

                // Gamemode
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

                if !isPaired {
                    Divider().background(MSCRemoteStyle.borderSubtle)
                    HStack(spacing: 6) {
                        Image(systemName: "lock").font(.system(size: 11))
                        Text("Pair in Settings to send commands.")
                            .font(.system(size: 12))
                    }
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.top, MSCRemoteStyle.spaceSM)
                }
            }
        }
        .mscCard()
    }

    // MARK: - Reusable sub-components

    /// A square icon button used in the Time/Weather grid.
    /// Three buttons in an HStack fill the card width evenly.
    private func quickIconButton(
        title: String,
        icon: String,
        tint: Color,
        bgTint: Color,
        command: String
    ) -> some View {
        let isSent = lastSentChip == command

        return Button {
            guard isPaired else { return }
            hapticLight()
            withAnimation(.easeInOut(duration: 0.12)) { lastSentChip = command }
            Task {
                await sendCommandString(command)
                try? await Task.sleep(nanoseconds: 600_000_000)
                await MainActor.run {
                    withAnimation(.easeInOut(duration: 0.2)) { lastSentChip = nil }
                }
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
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(isSent ? tint : tint.opacity(0.25), lineWidth: 1)
            )
            .animation(.easeInOut(duration: 0.15), value: isSent)
        }
        .disabled(!isPaired)
        .opacity(isPaired ? 1 : 0.45)
    }

    private func settingsRow<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceSM) {
            content()
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM)
    }

    private func errorBanner(_ message: String) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.danger)
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
    }

    private var footerText: some View {
        Text("TempleTech · MSC REMOTE")
            .font(.system(size: 10, weight: .regular, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .frame(maxWidth: .infinity, alignment: .center)
    }

    // MARK: - Actions

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
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
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
            if parts.count >= 2 {
                let sub = String(parts[1])
                if sub == "off" || sub == "remove" { return true }
            }
            return false
        default: return false
        }
    }
}

