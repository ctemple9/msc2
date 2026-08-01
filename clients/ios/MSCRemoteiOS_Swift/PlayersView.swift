import SwiftUI
import UIKit

// MARK: - PlayersView
//
// Shows online player count and a list of connected players.
// Tapping a player expands an inline action panel — no sheet or navigation
// push needed. This "expand-in-place" pattern keeps context clear: you
// can see the player you're acting on throughout the interaction.
//
// All player actions translate directly into server commands sent via
// the existing sendCommand path. This is the correct architecture here —
// the macOS API doesn't expose player-specific REST endpoints, so commands
// are the universal interface, exactly as they are on the Commands tab.

struct PlayersView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var expandedPlayerName: String? = nil
    @State private var selectedProfile: PlayerProfileDTO? = nil
    @State private var profilesExpanded: Bool = true
    @State private var sessionLogExpanded: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }

    private var activeServerType: ServerType {
        if let fromServers = vm.servers.first(where: { $0.id == (vm.status?.activeServerId ?? "") })?.resolvedServerType {
            return fromServers
        }
        return vm.status?.resolvedServerType ?? .java
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: 0) {
                    ScrollView(showsIndicators: false) {
                        VStack(spacing: MSCRemoteStyle.spaceLG) {
                            playerCountCard
                            if activeServerType == .bedrock {
                                allowlistLinkCard
                            }
                            playerListCard
                            playerProfilesCard
                            sessionLogCard
                        }
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        .frame(maxWidth: MSCRemoteStyle.contentMaxWidth)
                        .frame(maxWidth: .infinity)
                        .padding(.top, MSCRemoteStyle.spaceMD)
                        .padding(.bottom, MSCRemoteStyle.spaceLG)
                    }
                    .refreshable { await refresh() }
                    footerText.padding(.vertical, MSCRemoteStyle.spaceMD)
                }
            }
            .navigationTitle("Players")
            .navigationBarTitleDisplayMode(.large)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .task(id: isPaired) {
                guard isPaired else { return }
                await refresh()
            }
            .sheet(item: $selectedProfile) { profile in
                PlayerProfileSheet(profile: profile)
            }
        }
    }

    private func refresh() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        async let p: () = vm.fetchPlayerProfiles(baseURL: baseURL, token: token)
        async let s: () = vm.fetchSessionLog(baseURL: baseURL, token: token)
        if activeServerType == .bedrock {
            async let a: () = vm.fetchAllowlist(baseURL: baseURL, token: token)
            _ = await (p, s, a)
        } else {
            _ = await (p, s)
        }
    }

    // MARK: - Player Profiles Card

    private var playerProfilesCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Collapsible header
            Button {
                withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                    profilesExpanded.toggle()
                }
            } label: {
                HStack {
                    MSCSectionHeader(
                        title: "Player Data",
                        trailing: vm.playerProfilesResponse.map { "\($0.profiles.count)" }
                    )
                    Image(systemName: profilesExpanded ? "chevron.up" : "chevron.down")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            .buttonStyle(.plain)

            if profilesExpanded {
                Divider()
                    .background(MSCRemoteStyle.borderSubtle)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.horizontal, -MSCRemoteStyle.spaceLG)

                if let response = vm.playerProfilesResponse {
                    if response.profiles.isEmpty {
                        Text("No player data found. Make sure a server is selected on your Mac.")
                            .font(.system(size: 13))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .padding(.top, MSCRemoteStyle.spaceLG)
                            .frame(maxWidth: .infinity, alignment: .center)
                    } else {
                        let sorted = response.profiles.sorted {
                            if $0.isOnline != $1.isOnline { return $0.isOnline }
                            return ($0.displayName.lowercased()) < ($1.displayName.lowercased())
                        }
                        VStack(spacing: 0) {
                            ForEach(Array(sorted.enumerated()), id: \.element.id) { idx, profile in
                                Button { selectedProfile = profile } label: {
                                    profileRow(profile)
                                }
                                .buttonStyle(.plain)
                                if idx < sorted.count - 1 {
                                    Divider().background(MSCRemoteStyle.borderSubtle)
                                }
                            }
                        }
                        .padding(.top, MSCRemoteStyle.spaceSM)
                    }
                } else {
                    Text("Pull to refresh to load player data.")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .padding(.top, MSCRemoteStyle.spaceLG)
                        .frame(maxWidth: .infinity, alignment: .center)
                }
            }
        }
        .mscCard()
    }

    private func profileRow(_ profile: PlayerProfileDTO) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            PlayerSkinImageView(profile: profile, size: 36, cornerRadius: 5)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: 3) {
                HStack(spacing: 6) {
                    Text(profile.displayName)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    if profile.isOnline {
                        Circle().fill(MSCRemoteStyle.success).frame(width: 6, height: 6)
                    }
                    if profile.isOp {
                        Image(systemName: "star.fill")
                            .font(.system(size: 9))
                            .foregroundStyle(MSCRemoteStyle.warning)
                    }
                    if profile.isHiddenResolved {
                        Text("Hidden")
                            .font(.system(size: 9, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.warning)
                            .padding(.horizontal, MSCRemoteStyle.spaceSM)
                            .padding(.vertical, 2)
                            .background(Capsule().fill(MSCRemoteStyle.warning.opacity(0.12)))
                    }
                }
                if let lastSeen = profile.lastSeen {
                    Text(profile.isOnline ? "Online now" : "Last seen \(relativeDate(lastSeen))")
                        .font(.system(size: 11))
                        .foregroundStyle(profile.isOnline ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                }
            }

            Spacer()

            Image(systemName: "chevron.right")
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM + 2)
        .opacity(profile.isHiddenResolved ? 0.62 : 1.0)
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(profileAccessibilityLabel(profile))
        .accessibilityHint("Double tap for player details")
    }

    private func profileAccessibilityLabel(_ profile: PlayerProfileDTO) -> String {
        var parts = [profile.displayName]
        parts.append(profile.isOnline ? "Online" : "Offline")
        if profile.isOp { parts.append("Operator") }
        if profile.isHiddenResolved { parts.append("Hidden") }
        if !profile.isOnline, let lastSeen = profile.lastSeen {
            parts.append("last seen \(relativeDate(lastSeen))")
        }
        return parts.joined(separator: ", ")
    }

    // MARK: - Session Log Card

    private var sessionLogCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button {
                withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                    sessionLogExpanded.toggle()
                }
            } label: {
                HStack {
                    MSCSectionHeader(
                        title: "Session Log",
                        trailing: vm.sessionLogResponse.map { "\($0.events.count)" }
                    )
                    Image(systemName: sessionLogExpanded ? "chevron.up" : "chevron.down")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            .buttonStyle(.plain)

            if sessionLogExpanded {
                Divider()
                    .background(MSCRemoteStyle.borderSubtle)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.horizontal, -MSCRemoteStyle.spaceLG)
                    .padding(.bottom, MSCRemoteStyle.spaceSM)

                if let events = vm.sessionLogResponse?.events, !events.isEmpty {
                    VStack(spacing: 0) {
                        ForEach(Array(events.enumerated()), id: \.element.id) { idx, event in
                            sessionEventRow(event)
                            if idx < events.count - 1 {
                                Divider().background(MSCRemoteStyle.borderSubtle)
                            }
                        }
                    }
                } else {
                    Text("No session events recorded yet.")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.vertical, MSCRemoteStyle.spaceMD)
                }
            }
        }
        .mscCard()
    }

    private func sessionEventRow(_ event: SessionEventDTO) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: event.eventType == "joined" ? "arrow.right.circle.fill" : "arrow.left.circle.fill")
                .font(.system(size: 16))
                .foregroundStyle(event.eventType == "joined" ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: 2) {
                Text(event.playerName)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                Text(event.eventType == "joined" ? "joined" : "left")
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            }

            Spacer()

            Text(relativeDate(event.timestamp))
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM)
    }

    // MARK: - Formatting

    private func relativeDate(_ iso: String) -> String {
        let parser = ISO8601DateFormatter()
        parser.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        guard let date = parser.date(from: iso) else { return iso }
        let diff = Date().timeIntervalSince(date)
        if diff < 60 { return "just now" }
        if diff < 3600 { return "\(Int(diff / 60))m ago" }
        if diff < 86400 { return "\(Int(diff / 3600))h ago" }
        let days = Int(diff / 86400)
        if days < 30 { return "\(days)d ago" }
        let f = DateFormatter(); f.dateStyle = .medium; f.timeStyle = .none
        return f.string(from: date)
    }

    // MARK: - Player Count Card

    private var playerCountCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Online")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(vm.performanceLatest?.playersOnline.map(String.init) ?? "—")
                        .font(.system(.largeTitle, design: .rounded).weight(.bold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("players online")
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                Spacer()
                MSCStatusDot(isActive: vm.status?.running == true, size: 14)
                    .accessibilityHidden(true)
            }
            .accessibilityElement(children: .combine)
            .accessibilityLabel("Players online")
            .accessibilityValue("\(vm.performanceLatest?.playersOnline.map(String.init) ?? "unknown"), server \(vm.status?.running == true ? "running" : "stopped")")
        }
        .mscCard()
    }

    // MARK: - Allowlist Link (Bedrock only)

    private var allowlistLinkCard: some View {
        NavigationLink {
            AllowlistView()
                .environmentObject(settings)
                .environmentObject(vm)
        } label: {
            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Image(systemName: "list.bullet.clipboard")
                    .font(.system(size: 18))
                    .foregroundStyle(MSCRemoteStyle.accent)
                    .frame(width: 28)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text("Allowlist")
                        .font(.system(size: 15, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text(allowlistSubtitle)
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

    private var allowlistSubtitle: String {
        if let count = vm.allowlistResponse?.entries.count {
            return count == 0 ? "Empty — all players can join" : "\(count) player\(count == 1 ? "" : "s") allowed"
        }
        return "Manage who can join this Bedrock server"
    }

    // MARK: - Player List Card

    private var playerListCard: some View {
        let list = vm.players?.players ?? []
        let isRunning = vm.status?.running == true

        return VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Online Now")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if !isPaired {
                Text("Pair with your Mac to see players.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            } else if !isRunning || list.isEmpty {
                HStack(spacing: MSCRemoteStyle.spaceSM) {
                    Image(systemName: "person.slash")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                    Text("No players online")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            } else {
                VStack(spacing: 0) {
                    ForEach(list) { player in
                        ExpandablePlayerRow(
                            player: player,
                            serverType: activeServerType,
                            skinProfile: profileForOnlinePlayer(player),
                            isExpanded: expandedPlayerName == player.name,
                            onToggle: {
                                withAnimation(.spring(response: 0.3, dampingFraction: 0.75)) {
                                    if expandedPlayerName == player.name {
                                        expandedPlayerName = nil
                                    } else {
                                        expandedPlayerName = player.name
                                    }
                                }
                            },
                            onAction: { command in
                                Task { await sendCommand(command) }
                            }
                        )

                        if player.id != list.last?.id {
                            Divider()
                                .background(MSCRemoteStyle.borderSubtle)
                                .padding(.leading, 44)
                        }
                    }
                }
            }
        }
        .mscCard()
    }

    private func profileForOnlinePlayer(_ player: PlayerDTO) -> PlayerProfileDTO? {
        vm.playerProfilesResponse?.profiles.first {
            $0.displayName.caseInsensitiveCompare(player.name) == .orderedSame
        }
    }

    // MARK: - Footer

    private var footerText: some View {
        Text("TempleTech · MSC REMOTE")
            .font(.system(size: 10, weight: .regular, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .frame(maxWidth: .infinity, alignment: .center)
    }

    // MARK: - Command dispatch
    //
    // Delegates to DashboardViewModel.sendCommand, which is the same path
    // used by CommandsView. Player actions are just server commands with
    // the player name interpolated — there's no separate player API.

    private func sendCommand(_ command: String) async {
        guard let baseURL = settings.resolvedBaseURL(),
              let token = settings.resolvedToken() else { return }
        _ = await vm.sendCommand(baseURL: baseURL, token: token, command: command)
    }
}

// MARK: - ExpandablePlayerRow
//
// Composes PlayerRow (avatar + name) with an animated inline action panel.
// The panel appears below the name row with a spring animation — it feels
// physical and intentional, not like a sheet materialising from nowhere.
//
// Why inline expansion vs. a sheet?
// Sheets on iOS draw focus away from the list and require an extra dismiss
// tap. Inline expansion keeps the player name visible throughout the action,
// so there's no ambiguity about who you're acting on.

struct PlayerSkinImageView: View {
    let profile: PlayerProfileDTO?
    let size: CGFloat
    let cornerRadius: CGFloat

    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }

    private var cacheKey: String {
        guard let profile else { return "none" }
        return "\(profile.id)-\(profile.skinOverrideIdentifier ?? "")-\(profile.hasSkinFileOverride == true)"
    }

    private var uiImage: UIImage? {
        guard let profile,
              let base64 = vm.playerSkinResponses[profile.id]?.imageBase64,
              let data = Data(base64Encoded: base64) else { return nil }
        return UIImage(data: data)
    }

    var body: some View {
        Group {
            if let uiImage {
                Image(uiImage: uiImage)
                    .resizable()
                    .interpolation(.none)
                    .scaledToFit()
                    .frame(width: size, height: size)
                    .background(MSCRemoteStyle.bgElevated)
            } else {
                ZStack {
                    RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                        .fill(MSCRemoteStyle.bgElevated)
                    Image(systemName: "person.fill")
                        .font(.system(size: max(12, size * 0.38)))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
                .frame(width: size, height: size)
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
        .task(id: cacheKey) {
            guard let profile, let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
            await vm.fetchPlayerSkin(baseURL: baseURL, token: token, profileId: profile.id)
        }
    }
}

struct ExpandablePlayerRow: View {
    let player: PlayerDTO
    let serverType: ServerType
    let skinProfile: PlayerProfileDTO?
    let isExpanded: Bool
    let onToggle: () -> Void
    let onAction: (String) -> Void

    // Tracks the confirm state for destructive actions.
    // Only one confirmation prompt can be active at a time per row.
    @State private var pendingConfirmCommand: String? = nil
    @State private var pendingConfirmLabel: String? = nil
    @State private var showConfirm: Bool = false

    // Message field state — shown when the message action is tapped.
    @State private var showMessageField: Bool = false
    @State private var messageText: String = ""

    var body: some View {
        VStack(spacing: 0) {
            // Tappable header row
            Button(action: onToggle) {
                HStack(spacing: MSCRemoteStyle.spaceMD) {
                    PlayerSkinImageView(profile: skinProfile, size: 32, cornerRadius: 4)

                    Text(player.name)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)

                    Spacer()

                    // Chevron indicates expandability
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .accessibilityHidden(true)
                }
                .padding(.vertical, MSCRemoteStyle.spaceSM)
                .contentShape(Rectangle())
                .accessibilityElement(children: .combine)
                .accessibilityLabel(player.name)
                .accessibilityHint("Double tap to show actions")
                .accessibilityValue(isExpanded ? "Expanded" : "Collapsed")
            }
            .buttonStyle(.plain)

            // Expanded action panel — hidden by default, animated in on expand.
            // `clipped()` ensures the panel doesn't bleed outside the card
            // during the height animation.
            if isExpanded {
                VStack(spacing: MSCRemoteStyle.spaceSM) {
                    Divider()
                        .background(MSCRemoteStyle.borderSubtle)
                        .padding(.horizontal, -MSCRemoteStyle.spaceLG)

                    // Message player (non-destructive)
                    if showMessageField {
                        messageInputRow
                    } else {
                        actionGrid
                    }
                }
                .transition(.opacity.combined(with: .move(edge: .top)))
                .padding(.bottom, MSCRemoteStyle.spaceSM)
            }
        }
        .alert("Confirm Action", isPresented: $showConfirm) {
            Button("Cancel", role: .cancel) {
                pendingConfirmCommand = nil
                pendingConfirmLabel = nil
            }
            Button(pendingConfirmLabel ?? "Confirm", role: .destructive) {
                if let cmd = pendingConfirmCommand {
                    onAction(cmd)
                }
                pendingConfirmCommand = nil
                pendingConfirmLabel = nil
            }
        } message: {
            Text("Send \"\(pendingConfirmLabel ?? "this command")\" for \(player.name)?")
        }
    }

    // MARK: - Action Grid
    //
    // Two-column grid of labeled action buttons. Each action maps to a
    // specific Minecraft server command with the player name interpolated.
    //
    // Destructive actions (kick, ban, deop) require a confirmation tap
    // before the command is sent. This mirrors the CommandsView danger-check
    // and protects against accidental taps in a list.

    private var actionGrid: some View {
        VStack(spacing: MSCRemoteStyle.spaceSM) {
            // Row 1: Message (Java only — /tell not supported on BDS stdin) + TP to spawn
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                if serverType == .java {
                    playerActionButton(
                        title: "Message",
                        icon: "bubble.left.fill",
                        tint: MSCRemoteStyle.accent,
                        destructive: false
                    ) {
                        withAnimation { showMessageField = true }
                    }
                }

                playerActionButton(
                    title: "TP to Spawn",
                    icon: "location.fill",
                    tint: MSCRemoteStyle.actionMessage,
                    destructive: false
                ) {
                    // /tp works on both Java and BDS
                    onAction("/tp \(player.name) 0 64 0")
                }
            }

            // Row 2: Gamemode survival + creative
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                playerActionButton(
                    title: "Survival",
                    icon: "shield.fill",
                    tint: MSCRemoteStyle.actionKick,
                    destructive: false
                ) {
                    onAction("/gamemode survival \(player.name)")
                }

                playerActionButton(
                    title: "Creative",
                    icon: "wand.and.stars",
                    tint: MSCRemoteStyle.actionBan,
                    destructive: false
                ) {
                    onAction("/gamemode creative \(player.name)")
                }
            }

            // Row 3: OP + Kick
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                playerActionButton(
                    title: "Make OP",
                    icon: "star.fill",
                    tint: MSCRemoteStyle.actionKick,
                    destructive: true
                ) {
                    // Both Java and BDS use /op <name>
                    triggerConfirm(command: "/op \(player.name)", label: "Make OP")
                }

                playerActionButton(
                    title: "Kick",
                    icon: "person.fill.xmark",
                    tint: MSCRemoteStyle.danger,
                    destructive: true
                ) {
                    // Both Java and BDS use /kick <name>
                    triggerConfirm(command: "/kick \(player.name)", label: "Kick")
                }
            }
        }
        .padding(.top, MSCRemoteStyle.spaceSM)
    }

    // MARK: - Message Input Row

    private var messageInputRow: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                Text(">")
                    .font(.system(size: 13, weight: .bold, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.accent)

                TextField("Message to \(player.name)…", text: $messageText)
                    .font(.system(size: 13, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                    .textInputAutocapitalization(.sentences)
                    .submitLabel(.send)
                    .onSubmit { sendMessage() }
            }
            .padding(MSCRemoteStyle.spaceSM)
            .background(MSCRemoteStyle.bgDeep)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(MSCRemoteStyle.borderMid, lineWidth: 1)
            )

            HStack(spacing: MSCRemoteStyle.spaceSM) {
                Button {
                    withAnimation { showMessageField = false }
                    messageText = ""
                } label: {
                    Text("Cancel")
                        .font(.system(size: 13, weight: .semibold))
                        .frame(maxWidth: .infinity)
                        .frame(height: 36)
                        .foregroundStyle(MSCRemoteStyle.textSecondary)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                }

                Button {
                    sendMessage()
                } label: {
                    Text("Send")
                        .font(.system(size: 13, weight: .semibold))
                        .frame(maxWidth: .infinity)
                        .frame(height: 36)
                        .foregroundStyle(messageText.trimmingCharacters(in: .whitespaces).isEmpty ? MSCRemoteStyle.textTertiary : MSCRemoteStyle.bgBase)
                        .background(messageText.trimmingCharacters(in: .whitespaces).isEmpty ? MSCRemoteStyle.bgElevated : MSCRemoteStyle.accent)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                }
                .disabled(messageText.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding(.top, MSCRemoteStyle.spaceSM)
    }

    // MARK: - Sub-components

    /// A square action button used in the player action grid.
    private func playerActionButton(
        title: String,
        icon: String,
        tint: Color,
        destructive: Bool,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: icon)
                    .font(.system(size: 13))
                    .foregroundStyle(destructive ? MSCRemoteStyle.danger : tint)
                    .accessibilityHidden(true)
                Text(title)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(destructive ? MSCRemoteStyle.danger : MSCRemoteStyle.textPrimary)
            }
            .frame(maxWidth: .infinity)
            .frame(height: 40)
            .background(
                destructive
                    ? MSCRemoteStyle.danger.opacity(0.10)
                    : tint.opacity(0.10)
            )
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(
                        destructive
                            ? MSCRemoteStyle.danger.opacity(0.25)
                            : tint.opacity(0.25),
                        lineWidth: 1
                    )
            )
        }
        .buttonStyle(.plain)
    }

    private var playerAvatar: some View {
        Group {
            if let uuid = player.uuid, !uuid.isEmpty,
               let url = URL(string: "https://crafatar.com/avatars/\(uuid)?size=32&overlay=true") {
                AsyncImage(url: url) { phase in
                    switch phase {
                    case .success(let image):
                        image
                            .resizable()
                            .interpolation(.none)
                            .scaledToFit()
                    case .failure:
                        genericAvatarIcon
                    case .empty:
                        RoundedRectangle(cornerRadius: 4, style: .continuous)
                            .fill(MSCRemoteStyle.bgElevated)
                    @unknown default:
                        genericAvatarIcon
                    }
                }
            } else {
                genericAvatarIcon
            }
        }
    }

    private var genericAvatarIcon: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 4, style: .continuous)
                .fill(MSCRemoteStyle.bgElevated)
            Image(systemName: "person.fill")
                .font(.system(size: 14))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
        }
    }

    // MARK: - Helpers

    private func triggerConfirm(command: String, label: String) {
        hapticLight()
        pendingConfirmCommand = command
        pendingConfirmLabel = label
        showConfirm = true
    }

    private func sendMessage() {
        let text = messageText.trimmingCharacters(in: .whitespaces)
        guard !text.isEmpty else { return }
        hapticLight()
        // /tell is the private message command in vanilla/Paper Minecraft.
        // /msg and /w are aliases — /tell is most universally supported.
        onAction("/tell \(player.name) \(text)")
        messageText = ""
        withAnimation { showMessageField = false }
    }
}

// MARK: - Player Profile Sheet

struct PlayerProfileSheet: View {
    let profile: PlayerProfileDTO
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var skinOverrideInput: String = ""
    @State private var isSavingSkinOverride: Bool = false
    @State private var isTogglingHidden: Bool = false
    @State private var controlToast: String? = nil

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isAdmin: Bool { vm.connectedRole == "admin" }
    private var currentProfile: PlayerProfileDTO {
        vm.playerProfilesResponse?.profiles.first(where: { $0.id == profile.id }) ?? profile
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        headerSection
                        profileControlsSection
                        if let stats = profile.stats {
                            statsSection(stats)
                        } else {
                            noStatsSection
                        }
                        if !profile.inventory.isEmpty {
                            inventorySection
                        }
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceLG)
                    .padding(.bottom, MSCRemoteStyle.space2XL)
                }
            }
            .navigationTitle(currentProfile.displayName)
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                        .foregroundStyle(MSCRemoteStyle.accent)
                }
            }
            .task(id: currentProfile.id) {
                skinOverrideInput = currentProfile.skinOverrideIdentifier ?? ""
            }
        }
    }

    // MARK: - Header

    private var headerSection: some View {
        HStack(spacing: MSCRemoteStyle.spaceLG) {
            PlayerSkinImageView(profile: currentProfile, size: 72, cornerRadius: 8)

            VStack(alignment: .leading, spacing: 6) {
                Text(currentProfile.displayName)
                    .font(.system(size: 20, weight: .bold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)

                HStack(spacing: 6) {
                    statusBadge
                    if profile.isOp { opBadge }
                    if currentProfile.isHiddenResolved { hiddenBadge }
                }

                if let lastSeen = currentProfile.lastSeen {
                    Text(currentProfile.isOnline ? "Online now" : lastSeenText(lastSeen))
                        .font(.system(size: 11))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }
            Spacer()
        }
        .padding(MSCRemoteStyle.spaceLG)
        .background(MSCRemoteStyle.bgCard)
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
            .strokeBorder(MSCRemoteStyle.borderSubtle, lineWidth: 1))
    }

    private var statusBadge: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(currentProfile.isOnline ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
                .frame(width: 6, height: 6)
            Text(currentProfile.isOnline ? "Online" : "Offline")
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(currentProfile.isOnline ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 3)
        .background(Capsule().fill((currentProfile.isOnline ? MSCRemoteStyle.success : MSCRemoteStyle.textTertiary).opacity(0.12)))
    }

    private var opBadge: some View {
        HStack(spacing: 4) {
            Image(systemName: "star.fill").font(.system(size: 9)).foregroundStyle(MSCRemoteStyle.warning)
            Text("Operator").font(.system(size: 11, weight: .semibold)).foregroundStyle(MSCRemoteStyle.warning)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 3)
        .background(Capsule().fill(MSCRemoteStyle.warning.opacity(0.12)))
    }

    private var hiddenBadge: some View {
        Text("Hidden")
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(MSCRemoteStyle.warning)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(Capsule().fill(MSCRemoteStyle.warning.opacity(0.12)))
    }

    private var profileControlsSection: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Profile Controls")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(currentProfile.isHiddenResolved ? "Hidden from overview" : "Visible in overview")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("Hide removes the profile from spotlight lists without deleting player data.")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer()
                if isAdmin {
                    Button {
                        Task { await toggleHiddenProfile() }
                    } label: {
                        if isTogglingHidden {
                            ProgressView().scaleEffect(0.75)
                                .frame(width: 32, height: 32)
                        } else {
                            Image(systemName: currentProfile.isHiddenResolved ? "eye" : "eye.slash")
                                .font(.system(size: 14, weight: .semibold))
                                .foregroundStyle(currentProfile.isHiddenResolved ? MSCRemoteStyle.accent : MSCRemoteStyle.warning)
                                .frame(width: 32, height: 32)
                                .background(MSCRemoteStyle.bgElevated)
                                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                        }
                    }
                    .buttonStyle(.plain)
                    .disabled(isTogglingHidden)
                }
            }

            Divider()
                .background(MSCRemoteStyle.borderSubtle)
                .padding(.vertical, MSCRemoteStyle.spaceMD)

            if isAdmin {
                VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
                    Text("Skin Lookup Override")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                    TextField("Username, UUID, or .BedrockGamertag", text: $skinOverrideInput)
                        .font(.system(size: 13, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .padding(.horizontal, MSCRemoteStyle.spaceMD)
                        .padding(.vertical, 10)
                        .background(MSCRemoteStyle.bgElevated)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))

                    HStack(spacing: MSCRemoteStyle.spaceSM) {
                        profileControlButton(
                            title: isSavingSkinOverride ? "Saving..." : "Save Override",
                            icon: "checkmark",
                            enabled: !isSavingSkinOverride,
                            primary: true
                        ) {
                            Task { await saveSkinOverride(clear: false) }
                        }
                        profileControlButton(
                            title: "Clear",
                            icon: "xmark",
                            enabled: !isSavingSkinOverride && currentProfile.hasSkinOverride,
                            primary: false
                        ) {
                            Task { await saveSkinOverride(clear: true) }
                        }
                    }
                }
            } else {
                Text(currentProfile.hasSkinOverride ? "A skin override is set." : "No skin override is set.")
                    .font(.system(size: 13))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
            }

            if let controlToast {
                Text(controlToast)
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.success)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .transition(.opacity)
            }
        }
        .mscCard()
        .animation(.easeInOut(duration: 0.2), value: controlToast)
    }

    private func profileControlButton(title: String, icon: String, enabled: Bool, primary: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: MSCRemoteStyle.spaceSM) {
                if isSavingSkinOverride && primary {
                    ProgressView().scaleEffect(0.75)
                } else {
                    Image(systemName: icon)
                        .font(.system(size: 11, weight: .semibold))
                }
                Text(title)
                    .font(.system(size: 13, weight: .semibold))
            }
            .frame(maxWidth: .infinity)
            .frame(height: 38)
            .foregroundStyle(enabled ? (primary ? MSCRemoteStyle.bgBase : MSCRemoteStyle.textPrimary) : MSCRemoteStyle.textTertiary)
            .background(enabled ? (primary ? MSCRemoteStyle.accent : MSCRemoteStyle.bgElevated) : MSCRemoteStyle.bgElevated)
            .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                    .strokeBorder(primary ? MSCRemoteStyle.borderMid.opacity(0) : MSCRemoteStyle.borderMid, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
    }

    private func toggleHiddenProfile() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        hapticLight()
        isTogglingHidden = true
        let target = !currentProfile.isHiddenResolved
        let error = await vm.setHiddenProfile(baseURL: baseURL, token: token, profileId: currentProfile.id, hidden: target)
        isTogglingHidden = false
        if let error {
            hapticError()
            showControlToast(error)
        } else {
            hapticSuccess()
            showControlToast(target ? "Profile hidden." : "Profile visible.")
        }
    }

    private func saveSkinOverride(clear: Bool) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        hapticLight()
        isSavingSkinOverride = true
        let trimmed = skinOverrideInput.trimmingCharacters(in: .whitespacesAndNewlines)
        let error = await vm.setPlayerSkinOverride(
            baseURL: baseURL,
            token: token,
            profileId: currentProfile.id,
            lookupIdentifier: clear ? nil : trimmed
        )
        isSavingSkinOverride = false
        if let error {
            hapticError()
            showControlToast(error)
        } else {
            hapticSuccess()
            if clear { skinOverrideInput = "" }
            showControlToast(clear ? "Skin override cleared." : "Skin override saved.")
        }
    }

    private func showControlToast(_ message: String) {
        withAnimation { controlToast = message }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
            withAnimation { controlToast = nil }
        }
    }

    // MARK: - Stats

    @ViewBuilder
    private func statsSection(_ stats: PlayerStatsDTO) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Stats").padding(.bottom, MSCRemoteStyle.spaceMD)

            VStack(spacing: 0) {
                // Health
                statBarRow(
                    icon: "heart.fill", iconColor: MSCRemoteStyle.danger,
                    label: "Health",
                    value: String(format: "%.1f / %.0f", stats.health, stats.maxHealth),
                    fraction: stats.healthFraction,
                    barColor: MSCRemoteStyle.danger
                )
                Divider().background(MSCRemoteStyle.borderSubtle)

                // Food
                statBarRow(
                    icon: "fork.knife", iconColor: Color.orange,
                    label: "Food",
                    value: "\(stats.foodLevel) / 20",
                    fraction: stats.foodFraction,
                    barColor: Color.orange
                )
                Divider().background(MSCRemoteStyle.borderSubtle)

                // XP
                plainStatRow(icon: "sparkles", iconColor: Color.green,
                             label: "XP", value: "Level \(stats.xpLevel)  ·  \(stats.xpTotal) total")
                Divider().background(MSCRemoteStyle.borderSubtle)

                // Game mode
                plainStatRow(icon: "gamecontroller.fill", iconColor: MSCRemoteStyle.accent,
                             label: "Mode", value: stats.gameModeDisplay)
                Divider().background(MSCRemoteStyle.borderSubtle)

                // Position
                plainStatRow(icon: "location.fill", iconColor: MSCRemoteStyle.actionMessage,
                             label: "Position",
                             value: String(format: "x %.0f  y %.0f  z %.0f  ·  %@",
                                          stats.posX, stats.posY, stats.posZ, stats.dimensionDisplay))
                Divider().background(MSCRemoteStyle.borderSubtle)

                // Score
                plainStatRow(icon: "trophy.fill", iconColor: Color.yellow,
                             label: "Score", value: "\(stats.score)")
            }
        }
        .mscCard()
    }

    private func statBarRow(icon: String, iconColor: Color, label: String, value: String, fraction: Double, barColor: Color) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Image(systemName: icon).font(.system(size: 13)).foregroundStyle(iconColor).frame(width: 18)
                Text(label).font(.system(size: 13)).foregroundStyle(MSCRemoteStyle.textSecondary)
                Spacer()
                Text(value).font(.system(size: 13, weight: .semibold, design: .monospaced)).foregroundStyle(MSCRemoteStyle.textPrimary)
            }
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 3).fill(MSCRemoteStyle.bgElevated).frame(height: 6)
                    RoundedRectangle(cornerRadius: 3).fill(barColor)
                        .frame(width: max(0, geo.size.width * fraction), height: 6)
                }
            }
            .frame(height: 6)
        }
        .padding(.vertical, MSCRemoteStyle.spaceMD)
        .padding(.horizontal, MSCRemoteStyle.spaceMD)
    }

    private func plainStatRow(icon: String, iconColor: Color, label: String, value: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: icon).font(.system(size: 13)).foregroundStyle(iconColor).frame(width: 18)
            Text(label).font(.system(size: 13)).foregroundStyle(MSCRemoteStyle.textSecondary)
            Spacer()
            Text(value).font(.system(size: 13, weight: .medium)).foregroundStyle(MSCRemoteStyle.textPrimary)
                .multilineTextAlignment(.trailing)
        }
        .padding(.vertical, MSCRemoteStyle.spaceMD)
        .padding(.horizontal, MSCRemoteStyle.spaceMD)
    }

    // MARK: - Inventory

    private var inventorySection: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Inventory").padding(.bottom, MSCRemoteStyle.spaceMD)
            InventoryGridView(inventory: profile.inventory)
        }
        .mscCard()
    }

    private var noStatsSection: some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            ProgressView().tint(MSCRemoteStyle.textTertiary)
            Text("Loading stats…")
                .font(.system(size: 13)).foregroundStyle(MSCRemoteStyle.textTertiary)
        }
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(MSCRemoteStyle.spaceLG)
        .mscCard()
    }

    // MARK: - Helpers

    private func lastSeenText(_ iso: String) -> String {
        let parser = ISO8601DateFormatter()
        parser.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        guard let date = parser.date(from: iso) else { return "Last seen unknown" }
        let diff = Date().timeIntervalSince(date)
        if diff < 3600 { return "Last seen \(Int(diff / 60))m ago" }
        if diff < 86400 { return "Last seen \(Int(diff / 3600))h ago" }
        let days = Int(diff / 86400)
        if days < 30 { return "Last seen \(days)d ago" }
        let f = DateFormatter(); f.dateStyle = .medium; f.timeStyle = .none
        return "Last seen \(f.string(from: date))"
    }
}

// MARK: - Inventory Grid

struct InventoryGridView: View {
    let inventory: [InventoryItemDTO]

    private let slotSize: CGFloat = 38
    private let gap: CGFloat = 3
    private let cols = 9

    private func item(at slot: Int) -> InventoryItemDTO? {
        inventory.first { $0.slot == slot }
    }

    var body: some View {
        VStack(spacing: gap + 4) {
            // Equipment row: Helmet → Chestplate → Leggings → Boots, spacer, Offhand
            HStack(spacing: gap) {
                ForEach([103, 102, 101, 100], id: \.self) { slot in
                    InventorySlotView(item: item(at: slot), size: slotSize, highlighted: false)
                }
                Spacer()
                VStack(spacing: 2) {
                    InventorySlotView(item: item(at: -106), size: slotSize, highlighted: false, accent: true)
                    Text("Off")
                        .font(.system(size: 8, weight: .semibold, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }
            }

            Divider().background(MSCRemoteStyle.borderSubtle).padding(.vertical, 2)

            // Main inventory: rows of 9, slots 9–35
            VStack(spacing: gap) {
                ForEach(0..<3, id: \.self) { row in
                    HStack(spacing: gap) {
                        ForEach(0..<9, id: \.self) { col in
                            InventorySlotView(item: item(at: 9 + row * 9 + col), size: slotSize, highlighted: false)
                        }
                    }
                }
            }

            Divider().background(MSCRemoteStyle.borderSubtle).padding(.vertical, 2)

            // Hotbar: slots 0–8
            HStack(spacing: gap) {
                ForEach(0..<9, id: \.self) { col in
                    InventorySlotView(item: item(at: col), size: slotSize, highlighted: true)
                }
            }
        }
    }
}

struct InventorySlotView: View {
    let item: InventoryItemDTO?
    let size: CGFloat
    let highlighted: Bool
    var accent: Bool = false

    @State private var useBlockTexture = false
    @State private var showTooltip = false

    private static let baseURL = "https://raw.githubusercontent.com/InventivetalentDev/minecraft-assets/1.21.1/assets/minecraft/textures"

    private var iconURL: URL? {
        guard let item else { return nil }
        let path = useBlockTexture ? "block" : "item"
        return URL(string: "\(Self.baseURL)/\(path)/\(item.iconName).png")
    }

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            // Slot background
            RoundedRectangle(cornerRadius: 4, style: .continuous)
                .fill(Color(red: 0.05, green: 0.05, blue: 0.08))
                .overlay(
                    RoundedRectangle(cornerRadius: 4, style: .continuous)
                        .strokeBorder(
                            accent ? MSCRemoteStyle.accent.opacity(0.45)
                                : (highlighted ? Color.white.opacity(0.18) : Color.white.opacity(0.07)),
                            lineWidth: accent ? 1.5 : 1
                        )
                )

            if let item {
                // Item icon
                if let url = iconURL {
                    AsyncImage(url: url) { phase in
                        switch phase {
                        case .success(let img):
                            img.resizable().interpolation(.none).scaledToFit().padding(3)
                        case .failure:
                            Color.clear.onAppear {
                                if !useBlockTexture { useBlockTexture = true }
                            }
                        case .empty:
                            Color.clear
                        @unknown default:
                            Color.clear
                        }
                    }
                }

                // Count badge (only when > 1)
                if item.count > 1 {
                    Text("\(item.count)")
                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                        .foregroundStyle(.white)
                        .shadow(color: .black, radius: 1, x: 1, y: 1)
                        .padding(.trailing, 2)
                        .padding(.bottom, 1)
                }
            }
        }
        .frame(width: size, height: size)
        .popover(isPresented: $showTooltip) {
            if let item {
                VStack(alignment: .leading, spacing: 4) {
                    Text(item.displayName)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    if item.count > 1 {
                        Text("×\(item.count)")
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textSecondary)
                    }
                    ForEach(item.enchantments, id: \.id) { ench in
                        Text(ench.displayName)
                            .font(.system(size: 11))
                            .foregroundStyle(Color(red: 0.5, green: 0.5, blue: 1.0))
                    }
                }
                .padding(MSCRemoteStyle.spaceMD)
                .presentationCompactAdaptation(.popover)
            }
        }
        .onTapGesture {
            if item != nil { showTooltip.toggle() }
        }
    }
}
