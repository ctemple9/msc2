//
//  ComponentsView.swift
//  MSCRemoteiOS
//
//  U2 (flowstate): split out of HealthView.swift. Hosts server-component
//  management — the components status strip, version/JAR swap, resource
//  packs, and mods + the catalog browser — so Health can stay focused
//  on diagnostics and startup repairs. Reached from Health's "Manage
//  Components" card on iPhone, or its own sidebar entry on iPad.
//

import SwiftUI

struct ComponentsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var updatingComponent: String? = nil
    @State private var updateToast: String? = nil
    @State private var isRefreshing: Bool = false
    @State private var updatingAddon: String? = nil
    @State private var isUpdatingAll: Bool = false
    @State private var addonToRemove: AddonItemDTO? = nil
    @State private var showCatalog: Bool = false
    @State private var showVersionPicker: Bool = false
    @State private var showResourcePacks: Bool = false
    @State private var showClientExport: Bool = false
    @State private var showAllMods: Bool = false

    private var resolvedBaseURL: URL? { settings.resolvedBaseURL() }
    private var resolvedToken: String? { settings.resolvedToken() }
    private var isPaired: Bool { resolvedBaseURL != nil && resolvedToken != nil }

    var body: some View {
        ZStack {
            MSCRemoteStyle.bgBase.ignoresSafeArea()

            VStack(spacing: 0) {
                ScrollView(showsIndicators: false) {
                    VStack(spacing: MSCRemoteStyle.spaceLG) {
                        componentsCard
                        addonsCard
                        clientExportCard
                        resourcePackCard
                        versionCard
                    }
                    .padding(.horizontal, MSCRemoteStyle.spaceLG)
                    .padding(.top, MSCRemoteStyle.spaceMD)
                    .padding(.bottom, MSCRemoteStyle.spaceLG)
                }
                .refreshable { await refresh() }
                footerText.padding(.vertical, MSCRemoteStyle.spaceMD)
            }
        }
        .task(id: isPaired) {
            guard isPaired else { return }
            // Initial fetch on appear
            await refresh()
            // Keep polling while this view is visible
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 5_000_000_000)
                guard !Task.isCancelled else { break }
                await refresh()
            }
        }
        .background(clientExportSheetAnchor)
    }

    // MARK: - Components Card

    private var componentsCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Server Components")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            if let status = vm.componentsStatus {
                if status.components.isEmpty {
                    Text("No components found for the active server.")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                } else {
                    VStack(spacing: 0) {
                        ForEach(Array(status.components.enumerated()), id: \.element.id) { index, component in
                            componentRow(component)
                            if index < status.components.count - 1 {
                                Divider().background(MSCRemoteStyle.borderSubtle)
                            }
                        }
                    }

                    if status.restartRequiredToApply {
                        HStack(spacing: 6) {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .font(.system(size: 11))
                                .foregroundStyle(Color.orange)
                            Text("Restart the server to apply any updates.")
                                .font(.system(size: 12))
                                .foregroundStyle(Color.orange)
                        }
                        .padding(.top, MSCRemoteStyle.spaceMD)
                    }

                    if !settings.allowServerUpdates, status.components.contains(where: { $0.isUpdatable && !$0.isUpToDate }) {
                        HStack(spacing: 6) {
                            Image(systemName: "lock.fill")
                                .font(.system(size: 11))
                            Text("Component updates are disabled in Settings.")
                                .font(.system(size: 12))
                        }
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .padding(.top, MSCRemoteStyle.spaceMD)
                    }
                }
            } else {
                HStack(spacing: 8) {
                    ProgressView().scaleEffect(0.8)
                    Text("Loading component status…")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
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

    @ViewBuilder
    private func componentRow(_ component: ComponentStatusDTO) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            // Status dot
            Circle()
                .fill(rowDotColor(component))
                .frame(width: 8, height: 8)
                .shadow(color: rowDotColor(component).opacity(0.5), radius: 3)

            // Name + version info
            VStack(alignment: .leading, spacing: 2) {
                Text(component.name)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)

                if let installedDisplay = component.installedDisplay {
                    Text(installedDisplay)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                } else {
                    Text("Not found")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                }

                if let latest = component.latestBuild, let installed = component.installedBuild, installed < latest {
                    let latestVer = component.latestVersion.map { "\($0) · " } ?? ""
                    Text("Latest: \(latestVer)build \(latest)")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(Color.orange.opacity(0.85))
                }
            }

            Spacer()

            // Status badge or update button
            if !component.isInstalled {
                Text("Not installed")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
            } else if !component.isUpdatable {
                // Flavor server jars (Fabric, Vanilla, Forge, …) are managed via
                // Change Version / JAR, so they show a neutral "Installed" badge.
                Text("Installed")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textSecondary)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Capsule().fill(MSCRemoteStyle.textSecondary.opacity(0.10)))
                    .overlay(Capsule().stroke(MSCRemoteStyle.textSecondary.opacity(0.25), lineWidth: 0.75))
            } else if component.isUpToDate {
                HStack(spacing: 4) {
                    Image(systemName: "checkmark")
                        .font(.system(size: 9, weight: .bold))
                    Text("Up to date")
                        .font(.system(size: 10, weight: .semibold))
                }
                .foregroundStyle(MSCRemoteStyle.accent)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Capsule().fill(MSCRemoteStyle.accent.opacity(0.12)))
                .overlay(Capsule().stroke(MSCRemoteStyle.accent.opacity(0.3), lineWidth: 0.75))
            } else if vm.connectedRole != "guest" && settings.allowServerUpdates {
                Button {
                    Task { await updateComponent(component) }
                } label: {
                    if updatingComponent == component.name {
                        ProgressView()
                            .scaleEffect(0.7)
                            .frame(width: 60, height: 24)
                    } else {
                        Text("Update")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(.white)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 5)
                            .background(Capsule().fill(Color.orange))
                    }
                }
                .buttonStyle(.plain)
                .disabled(updatingComponent != nil || !isPaired)
            } else if vm.connectedRole != "guest" {
                Text("Locked")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Capsule().fill(MSCRemoteStyle.bgElevated))
                    .overlay(Capsule().stroke(MSCRemoteStyle.borderSubtle, lineWidth: 0.75))
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM)
    }

    private func rowDotColor(_ component: ComponentStatusDTO) -> Color {
        guard component.isInstalled else { return MSCRemoteStyle.textTertiary }
        return component.isUpToDate ? MSCRemoteStyle.accent : Color.orange
    }

    // MARK: - Version Card

    @ViewBuilder
    private var versionCard: some View {
        if isPaired && vm.connectedRole != "guest" {
            VStack(alignment: .leading, spacing: 0) {
                MSCSectionHeader(title: "Server Version")
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                Button {
                    showVersionPicker = true
                } label: {
                    HStack(spacing: MSCRemoteStyle.spaceMD) {
                        Image(systemName: "server.rack")
                            .font(.system(size: 22))
                            .foregroundStyle(MSCRemoteStyle.accent.opacity(0.8))
                            .frame(width: 32)

                        VStack(alignment: .leading, spacing: 3) {
                            Text("Change Version / JAR")
                                .font(.system(size: 14, weight: .semibold))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                            Text("View available versions and apply a new server JAR.")
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
            .sheet(isPresented: $showVersionPicker) {
                ServerVersionView(onDidChange: { Task { await refresh() } })
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
        }
    }

    // MARK: - Resource Pack Card

    @ViewBuilder
    private var resourcePackCard: some View {
        if isPaired {
            VStack(alignment: .leading, spacing: 0) {
                MSCSectionHeader(title: "Manage Resource Packs")
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                Button {
                    showResourcePacks = true
                } label: {
                    HStack(spacing: MSCRemoteStyle.spaceMD) {
                        Image(systemName: "shippingbox")
                            .font(.system(size: 22))
                            .foregroundStyle(MSCRemoteStyle.accent.opacity(0.8))
                            .frame(width: 32)

                        VStack(alignment: .leading, spacing: 3) {
                            Text("Manage Resource Packs")
                                .font(.system(size: 14, weight: .semibold))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                            Text("View, activate, and remove server resource packs.")
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
            .sheet(isPresented: $showResourcePacks) {
                ResourcePacksView(onDidChange: { Task { await refresh() } })
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
        }
    }

    // MARK: - Mods Card

    @ViewBuilder
    private var addonsCard: some View {
        if let response = vm.addonsResponse, response.serverSupportsAddons {
            let updateCount = response.updateCount
            let previewLimit = 15
            let visibleAddons = showAllMods ? response.addons : Array(response.addons.prefix(previewLimit))
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: 6) {
                    MSCSectionHeader(
                        title: "Mods",
                        trailing: updateCount > 0 ? "\(updateCount) update\(updateCount == 1 ? "" : "s")" : nil
                    )
                    if vm.connectedRole != "guest" {
                        Button {
                            showCatalog = true
                        } label: {
                            Image(systemName: "plus.circle.fill")
                                .font(.system(size: 18))
                                .foregroundStyle(MSCRemoteStyle.accent)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Browse mods")
                    }
                }
                .padding(.bottom, MSCRemoteStyle.spaceMD)

                if response.isResolving {
                    HStack(spacing: 8) {
                        ProgressView().scaleEffect(0.7)
                        Text("Checking for updates…")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                    .padding(.bottom, MSCRemoteStyle.spaceSM)
                }

                if response.packManaged == true {
                    HStack(spacing: 6) {
                        Image(systemName: "shippingbox")
                            .font(.system(size: 11))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                        Text(response.packName.map { "Pack-managed: \($0)" } ?? "Pack-managed server")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                    }
                    .padding(.bottom, MSCRemoteStyle.spaceSM)
                }

                if vm.connectedRole != "guest" && updateCount > 0 && settings.allowServerUpdates {
                    Button {
                        Task { await doUpdateAllAddons() }
                    } label: {
                        HStack(spacing: 6) {
                            if isUpdatingAll {
                                ProgressView().scaleEffect(0.8)
                            } else {
                                Image(systemName: "arrow.down.circle.fill")
                                    .font(.system(size: 13, weight: .semibold))
                            }
                            Text(isUpdatingAll ? "Starting updates…" : "Update All Mods (\(updateCount))")
                                .font(.system(size: 13, weight: .semibold))
                        }
                        .frame(maxWidth: .infinity)
                        .frame(height: 36)
                        .foregroundStyle(isUpdatingAll ? MSCRemoteStyle.textTertiary : .white)
                        .background(isUpdatingAll ? MSCRemoteStyle.bgElevated : Color.orange)
                        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
                    }
                    .buttonStyle(.plain)
                    .disabled(isUpdatingAll || !isPaired)
                    .padding(.bottom, MSCRemoteStyle.spaceMD)
                } else if vm.connectedRole != "guest" && updateCount > 0 {
                    HStack(spacing: 6) {
                        Image(systemName: "lock.fill")
                            .font(.system(size: 11))
                        Text("Mod and component updates are disabled in Settings.")
                            .font(.system(size: 12))
                    }
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .padding(.bottom, MSCRemoteStyle.spaceMD)
                }

                if response.addons.isEmpty {
                    Text(response.isResolving ? "Scanning for mods…" : "No tracked mods found.")
                        .font(.system(size: 13))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                } else {
                    VStack(spacing: 0) {
                        ForEach(Array(visibleAddons.enumerated()), id: \.element.id) { idx, addon in
                            addonRow(addon)
                            if idx < visibleAddons.count - 1 {
                                Divider().background(MSCRemoteStyle.borderSubtle)
                            }
                        }
                    }

                    if response.addons.count > previewLimit {
                        Button {
                            withAnimation(.easeInOut(duration: 0.2)) {
                                showAllMods.toggle()
                            }
                        } label: {
                            HStack(spacing: 6) {
                                Text(showAllMods ? "Show Fewer" : "Show All Mods")
                                    .font(.system(size: 12, weight: .semibold))
                                if !showAllMods {
                                    Text("(\(response.addons.count))")
                                        .font(.system(size: 11, design: .monospaced))
                                }
                            }
                            .foregroundStyle(MSCRemoteStyle.accent)
                            .frame(maxWidth: .infinity)
                            .padding(.top, MSCRemoteStyle.spaceMD)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .mscCard()
            .alert(
                "Remove \"\(addonToRemove?.displayName ?? "this mod")\"?",
                isPresented: Binding(get: { addonToRemove != nil }, set: { if !$0 { addonToRemove = nil } })
            ) {
                Button("Cancel", role: .cancel) { addonToRemove = nil }
                Button("Remove", role: .destructive) {
                    if let addon = addonToRemove {
                        Task { await doRemoveAddon(addon) }
                    }
                    addonToRemove = nil
                }
            } message: {
                Text("This will delete the file from the server and cannot be undone.")
            }
            .sheet(isPresented: $showCatalog) {
                CatalogBrowserView(onDidInstall: { Task { await refresh() } })
                    .environmentObject(settings)
                    .environmentObject(vm)
            }
        }
    }

    // MARK: - Client Export Card

    @ViewBuilder
    private var clientExportCard: some View {
        if isPaired {
            VStack(alignment: .leading, spacing: 0) {
                MSCSectionHeader(title: "Client Export")
                    .padding(.bottom, MSCRemoteStyle.spaceMD)

                Button {
                    hapticLight()
                    showClientExport = true
                } label: {
                    HStack(spacing: MSCRemoteStyle.spaceMD) {
                        Image(systemName: "square.and.arrow.up")
                            .font(.system(size: 22))
                            .foregroundStyle(MSCRemoteStyle.accent.opacity(0.8))
                            .frame(width: 32)

                        VStack(alignment: .leading, spacing: 3) {
                            Text("Client Export")
                                .font(.system(size: 14, weight: .semibold))
                                .foregroundStyle(MSCRemoteStyle.textPrimary)
                            Text("Share required client mods or Modrinth links with players.")
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
            }
            .mscCard()
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

    @ViewBuilder
    private func addonRow(_ addon: AddonItemDTO) -> some View {
        HStack(spacing: MSCRemoteStyle.spaceMD) {
            Circle()
                .fill(addonDotColor(addon))
                .frame(width: 8, height: 8)
                .shadow(color: addonDotColor(addon).opacity(0.5), radius: 3)

            VStack(alignment: .leading, spacing: 2) {
                Text(addon.displayName)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(addon.isEnabled ? MSCRemoteStyle.textPrimary : MSCRemoteStyle.textTertiary)

                Text(addon.jarStem)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)

                if addon.hasUpdate, let available = addon.availableVersion {
                    let current = addon.currentVersion.map { "\($0) → " } ?? ""
                    Text("\(current)\(available)")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(Color.orange.opacity(0.85))
                }
            }

            Spacer()

            HStack(spacing: MSCRemoteStyle.spaceSM) {
                if addon.hasUpdate && vm.connectedRole != "guest" && settings.allowServerUpdates {
                    Button {
                        Task { await doUpdateAddon(addon) }
                    } label: {
                        if updatingAddon == addon.jarStem {
                            ProgressView().scaleEffect(0.7).frame(width: 50, height: 24)
                        } else {
                            Text("Update")
                                .font(.system(size: 10, weight: .semibold))
                                .foregroundStyle(.white)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(Capsule().fill(Color.orange))
                        }
                    }
                    .buttonStyle(.plain)
                    .disabled(updatingAddon != nil || !isPaired)
                } else if addon.hasUpdate && vm.connectedRole != "guest" {
                    Text("Locked")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Capsule().fill(MSCRemoteStyle.bgElevated))
                        .overlay(Capsule().stroke(MSCRemoteStyle.borderSubtle, lineWidth: 0.75))
                } else {
                    addonStatusBadge(addon)
                }

                if vm.connectedRole != "guest" {
                    Button {
                        hapticLight()
                        addonToRemove = addon
                    } label: {
                        Image(systemName: "trash")
                            .font(.system(size: 12))
                            .foregroundStyle(MSCRemoteStyle.danger.opacity(0.7))
                    }
                    .buttonStyle(.plain)
                    .disabled(!isPaired)
                }
            }
        }
        .padding(.vertical, MSCRemoteStyle.spaceSM)
    }

    private func addonDotColor(_ addon: AddonItemDTO) -> Color {
        guard addon.isEnabled else { return MSCRemoteStyle.textTertiary }
        switch addon.bucket {
        case "upToDate":       return MSCRemoteStyle.accent
        case "updateAvailable": return Color.orange
        default:               return MSCRemoteStyle.textTertiary
        }
    }

    @ViewBuilder
    private func addonStatusBadge(_ addon: AddonItemDTO) -> some View {
        switch addon.bucket {
        case "upToDate":
            HStack(spacing: 4) {
                Image(systemName: "checkmark").font(.system(size: 9, weight: .bold))
                Text("Up to date").font(.system(size: 10, weight: .semibold))
            }
            .foregroundStyle(MSCRemoteStyle.accent)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Capsule().fill(MSCRemoteStyle.accent.opacity(0.12)))
            .overlay(Capsule().stroke(MSCRemoteStyle.accent.opacity(0.3), lineWidth: 0.75))
        case "noCompatibleVersion":
            Text("No match")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Capsule().fill(MSCRemoteStyle.textTertiary.opacity(0.10)))
        default:
            Text("Unlinked")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Capsule().fill(MSCRemoteStyle.textTertiary.opacity(0.10)))
        }
    }

    // MARK: - Actions

    private func refresh() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        isRefreshing = true
        async let c: () = vm.fetchComponentsAndBroadcast(baseURL: baseURL, token: token)
        async let a: () = vm.fetchAddons(baseURL: baseURL, token: token)
        _ = await (c, a)
        isRefreshing = false
    }

    private func updateComponent(_ component: ComponentStatusDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        guard settings.allowServerUpdates else {
            showUpdateToast("Updates are disabled in Settings")
            return
        }
        updatingComponent = component.name
        vm.updateCredentials(baseURL: baseURL, token: token)
        do {
            let client = try vm.requireClient()
            let result = try await client.updateComponent(component.name.lowercased())
            showUpdateToast(result.success ? result.message : "Failed: \(result.message)")
            if result.success {
                await vm.fetchComponentsAndBroadcast(baseURL: baseURL, token: token)
            }
        } catch {
            showUpdateToast("Error: \(error.localizedDescription)")
        }
        updatingComponent = nil
    }

    private func doUpdateAddon(_ addon: AddonItemDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        guard settings.allowServerUpdates else {
            showUpdateToast("Updates are disabled in Settings")
            return
        }
        updatingAddon = addon.jarStem
        let result = await vm.updateAddon(baseURL: baseURL, token: token, jarStem: addon.jarStem)
        switch result {
        case "update_started": showUpdateToast("Update started for \(addon.displayName)")
        case "no_updates_available": showUpdateToast("\(addon.displayName) is already up to date")
        case let r?: showUpdateToast(r)
        default: showUpdateToast("Update started")
        }
        updatingAddon = nil
    }

    private func doUpdateAllAddons() async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        guard settings.allowServerUpdates else {
            showUpdateToast("Updates are disabled in Settings")
            return
        }
        isUpdatingAll = true
        let result = await vm.updateAllAddons(baseURL: baseURL, token: token)
        switch result {
        case "update_started": showUpdateToast("All mod updates started")
        case "no_updates_available": showUpdateToast("All mods are already up to date")
        case let r?: showUpdateToast(r)
        default: showUpdateToast("Updates started")
        }
        isUpdatingAll = false
    }

    private func doRemoveAddon(_ addon: AddonItemDTO) async {
        guard let baseURL = resolvedBaseURL, let token = resolvedToken else { return }
        let errMsg = await vm.removeAddon(baseURL: baseURL, token: token, jarStem: addon.jarStem)
        if let err = errMsg {
            showUpdateToast("Remove failed: \(err)")
        } else {
            showUpdateToast("Removed \(addon.displayName)")
        }
    }

    private func showUpdateToast(_ text: String) {
        withAnimation { updateToast = text }
        DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
            withAnimation { updateToast = nil }
        }
    }

    private var footerText: some View {
        Text("TempleTech · MSC REMOTE")
            .font(.system(size: 10, weight: .regular, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .frame(maxWidth: .infinity, alignment: .center)
    }
}
