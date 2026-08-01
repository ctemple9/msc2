import SwiftUI

struct ServerView: View {
    private enum Segment { case content, worlds, admin }

    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var selectedSegment: Segment = .content
    @State private var isContentRefreshing: Bool = false
    @State private var worldsShowCreate: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }
    private var isAdmin: Bool { vm.connectedRole == "admin" }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: 0) {
                    Picker("", selection: $selectedSegment) {
                        Text("Content").tag(Segment.content)
                        Text("Worlds").tag(Segment.worlds)
                        Text("Admin").tag(Segment.admin)
                    }
                    .pickerStyle(.segmented)
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceSM)

                    segmentContent
                }
            }
            .navigationTitle("Server")
            .navigationBarTitleDisplayMode(.large)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .toolbar {
                if selectedSegment == .content {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button {
                            Task { await refreshContent() }
                        } label: {
                            Image(systemName: "arrow.clockwise")
                                .rotationEffect(.degrees(isContentRefreshing ? 360 : 0))
                                .animation(
                                    isContentRefreshing
                                        ? .linear(duration: 0.8).repeatForever(autoreverses: false)
                                        : .default,
                                    value: isContentRefreshing
                                )
                        }
                        .disabled(isContentRefreshing)
                    }
                } else if selectedSegment == .worlds && isAdmin {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button {
                            worldsShowCreate = true
                        } label: {
                            Image(systemName: "plus")
                                .foregroundStyle(MSCRemoteStyle.accent)
                        }
                        .disabled(!isPaired)
                    }
                }
            }
            .onChange(of: selectedSegment) { _, new in
                if new != .worlds { worldsShowCreate = false }
            }
        }
    }

    @ViewBuilder
    private var segmentContent: some View {
        if selectedSegment == .content {
            ComponentsView()
                .environmentObject(settings)
                .environmentObject(vm)
        } else if selectedSegment == .worlds {
            WorldsView(showCreateSheet: $worldsShowCreate)
                .environmentObject(settings)
                .environmentObject(vm)
        } else {
            ServerAdminContent()
                .environmentObject(settings)
                .environmentObject(vm)
        }
    }

    private func refreshContent() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isContentRefreshing = true
        async let c: () = vm.fetchComponentsAndBroadcast(baseURL: baseURL, token: token)
        async let a: () = vm.fetchAddons(baseURL: baseURL, token: token)
        _ = await (c, a)
        isContentRefreshing = false
    }
}

private struct ServerAdminContent: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var showServerFiles: Bool = false

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
    private var connectivitySubtitle: String {
        if activeServer?.supportsJavaBedrockCrossPlay == true {
            return "playit.gg, Xbox Broadcast, DuckDNS, and Geyser."
        }
        if activeServer?.resolvedServerType == .bedrock {
            return "playit.gg, Xbox Broadcast, and DuckDNS."
        }
        return "playit.gg and DuckDNS."
    }

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            VStack(spacing: 0) {
                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        if isPaired {
                            serverSettingsCard
                            serverAdministrationCard
                        } else {
                            unpairedCard
                        }
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                }
                footerText.padding(.vertical, MSCRemoteStyle.spaceMD)
            }
        }
        .background(serverFilesSheetAnchor)
    }

    private var serverSettingsCard: some View {
        NavigationLink {
            ServerSettingsView()
                .environmentObject(settings)
                .environmentObject(vm)
        } label: {
            VStack(alignment: .leading, spacing: 0) {
                MSCSectionHeader(title: "Server Settings")
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                serverNavRow(
                    title: "Server Settings",
                    subtitle: "Edit difficulty, players, MOTD, ports & more.",
                    icon: "slider.horizontal.3"
                )
            }
            .mscCard()
        }
        .buttonStyle(.plain)
    }

    private var serverAdministrationCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Server Administration")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            NavigationLink {
                ConnectivityView()
                    .environmentObject(settings)
                    .environmentObject(vm)
            } label: {
                serverNavRow(
                    title: "Connectivity",
                    subtitle: connectivitySubtitle,
                    icon: "network"
                )
            }
            .buttonStyle(.plain)

            Divider()
                .background(MSCRemoteStyle.borderSubtle)
                .padding(.vertical, MSCRemoteStyle.spaceSM)

            NavigationLink {
                MaintenanceView()
                    .environmentObject(settings)
                    .environmentObject(vm)
            } label: {
                serverNavRow(
                    title: "Diagnostics & Maintenance",
                    subtitle: "Health checks, startup repairs, and memory.",
                    icon: "wrench.and.screwdriver"
                )
            }
            .buttonStyle(.plain)

            if vm.connectedRole == "admin" {
                Divider()
                    .background(MSCRemoteStyle.borderSubtle)
                    .padding(.vertical, MSCRemoteStyle.spaceSM)

                Button {
                    hapticLight()
                    showServerFiles = true
                } label: {
                    serverNavRow(
                        title: "Server Files",
                        subtitle: "Browse and preview active-server files.",
                        icon: "folder"
                    )
                }
                .buttonStyle(.plain)
            }
        }
        .mscCard()
    }

    private var unpairedCard: some View {
        VStack(alignment: .leading, spacing: MSCRemoteStyle.spaceSM) {
            MSCSectionHeader(title: "Server Administration")
            Text("Pair with a server to manage settings, connectivity, maintenance, and files.")
                .font(.system(size: 13))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .mscCard()
    }

    private func serverNavRow(title: String, subtitle: String, icon: String) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Image(systemName: icon)
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.accent)
                .frame(width: 28)
                .accessibilityHidden(true)

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
                .accessibilityHidden(true)
        }
        .contentShape(Rectangle())
    }

    private var serverFilesSheetAnchor: some View {
        Color.clear
            .sheet(isPresented: $showServerFiles) {
                ServerFilesRemoteSheet()
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
    }

    private var footerText: some View {
        Text("TempleTech · MSC REMOTE")
            .font(.system(size: 10, weight: .regular, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .frame(maxWidth: .infinity, alignment: .center)
    }
}
