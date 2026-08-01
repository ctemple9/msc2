import SwiftUI
import UIKit

struct SettingsView: View {
    @EnvironmentObject private var settings: SettingsStore
    @EnvironmentObject private var vm: DashboardViewModel

    @State private var testIsRunning: Bool = false
    @State private var safetyWarning: String? = nil
    @State private var showQRScanner: Bool = false
    @State private var showTailscaleHelp: Bool = false
    @State private var toastText: String = ""
    @State private var showToast: Bool = false
    @State private var saveConfirmed: Bool = false

    // Java runtime switching
    @State private var currentJavaConfig: JavaConfigResponseDTO? = nil
    @State private var javaRuntimes: [JavaRuntimeDTO] = []
    @State private var isLoadingJavaConfig: Bool = false
    @State private var showJavaRuntimePicker: Bool = false

    // Xbox Broadcast JAR
    @State private var broadcastJarStatus: BroadcastJarStatusDTO? = nil
    @State private var isDownloadingBroadcastJar: Bool = false

    private var isPaired: Bool {
        settings.resolvedBaseURL() != nil && settings.resolvedToken() != nil
    }

    private var hasToken: Bool {
        !settings.tokenDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private var isFirstRun: Bool {
        settings.baseURLString.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !hasToken
    }

    var body: some View {
        NavigationStack {
            ZStack {
                MSCRemoteStyle.bgBase.ignoresSafeArea()

                VStack(spacing: 0) {
                    ScrollView(showsIndicators: false) {
                        VStack(spacing: MSCRemoteStyle.spaceLG) {
                            SettingsPairingCard(
                                isPaired: isPaired,
                                isFirstRun: isFirstRun,
                                hasToken: hasToken,
                                safetyWarning: safetyWarning,
                                saveConfirmed: saveConfirmed,
                                showQRScanner: $showQRScanner,
                                showTailscaleHelp: $showTailscaleHelp,
                                saveAction: savePairing,
                                clearTokenAction: clearToken,
                                clearBaseURLAction: clearBaseURLAction,
                                pastePairingAction: pastePairingLinkFromClipboard
                            )

                            SettingsConnectionTestSection(
                                testIsRunning: testIsRunning,
                                lastTestResult: settings.lastTestResult,
                                lastTestWasSuccess: settings.lastTestWasSuccess,
                                testAction: runConnectionTest
                            )

                            updateSafetyCard

                            if isPaired && vm.connectedRole == "admin" {
                                usersAccessCard
                                javaRuntimeCard
                                broadcastJarCard
                            }

                            notificationsCard

                            SettingsJoinCardSection(
                                showJoinCard: $settings.showJoinCard,
                                saveAction: settings.saveJoinCardPreferences
                            )

                            accentColorCard
                        }
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        .padding(.top, MSCRemoteStyle.spaceMD)
                        .padding(.bottom, MSCRemoteStyle.spaceLG)
                    }
                    footerText.padding(.vertical, MSCRemoteStyle.spaceMD)
                }
            }
            .navigationTitle("Settings")
            .navigationBarTitleDisplayMode(.large)
            .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
            .overlay(alignment: .bottom) {
                if showToast {
                    ToastView(text: toastText)
                        .padding(.horizontal, MSCRemoteStyle.spaceLG)
                        .padding(.bottom, MSCRemoteStyle.spaceLG + 50)
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }
            }
            .animation(.easeInOut(duration: 0.2), value: showToast)
            .onAppear {
                settings.loadTokenFromKeychain()
                recomputeSafetyWarning()
                Task { await loadJavaConfigIfAdmin() }
                Task { await loadBroadcastJarStatusIfAdmin() }
            }
            .onChange(of: settings.baseURLString) { _, _ in recomputeSafetyWarning() }
            .fullScreenCover(isPresented: $showQRScanner) {
                QRScannerView { payload in
                    let ok = settings.applyPairingPayload(payload)
                    if !ok {
                        settings.lastTestResult = "QR scan did not contain a valid pairing link."
                        settings.lastTestWasSuccess = false
                        hapticError()
                        presentToast("Invalid QR")
                    } else {
                        DispatchQueue.main.async { settings.loadTokenFromKeychain() }
                        hapticSuccess()
                        presentToast("Pairing imported")
                    }
                    showQRScanner = false
                    recomputeSafetyWarning()
                } onCancel: {
                    showQRScanner = false
                }
            }
            .sheet(isPresented: $showTailscaleHelp) {
                TailscaleHelpSheet(isPresented: $showTailscaleHelp)
            }
            .background(
                Color.clear
                    .sheet(isPresented: $showJavaRuntimePicker) {
                        SettingsJavaRuntimePickerSheet(
                            currentPath: currentJavaConfig?.executablePath,
                            runtimes: javaRuntimes,
                            onSelect: { executablePath in
                                applyJavaRuntime(executablePath: executablePath)
                            }
                        )
                    }
            )
        }
    }

    private var updateSafetyCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Update Safety")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Allow mod & component updates")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Text("Turn this off to hide update buttons that can change server behavior.")
                        .font(.system(size: 12))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer()
                Toggle("", isOn: $settings.allowServerUpdates)
                    .labelsHidden()
                    .tint(MSCRemoteStyle.accent)
                    .onChange(of: settings.allowServerUpdates) { _, _ in
                        settings.saveServerUpdatePreference()
                    }
                    .accessibilityLabel("Allow mod and component updates")
            }
        }
        .mscCard()
    }

    private var usersAccessCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Users & Access")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            NavigationLink {
                UsersView()
                    .environmentObject(settings)
                    .environmentObject(vm)
            } label: {
                settingsNavRow(
                    title: "Users & Access",
                    subtitle: "Manage shared access tokens and permissions.",
                    icon: "person.2.badge.key.fill"
                )
            }
            .buttonStyle(.plain)
        }
        .mscCard()
    }

    private var javaRuntimeCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Java Runtime")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            Button {
                Task { await openJavaRuntimePicker() }
            } label: {
                HStack(spacing: MSCRemoteStyle.spaceMD) {
                    Image(systemName: "cup.and.heat.waves.fill")
                        .font(.system(size: 18, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.accent)
                        .frame(width: 28)
                        .accessibilityHidden(true)

                    VStack(alignment: .leading, spacing: 3) {
                        Text("Active Java Runtime")
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                        if isLoadingJavaConfig {
                            Text("Loading…")
                                .font(.system(size: 12))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                        } else if let path = currentJavaConfig?.executablePath, !path.isEmpty {
                            let match = javaRuntimes.first { $0.executablePath == path }
                            Text(match.map { "\($0.versionLabel) — \($0.name)" } ?? path)
                                .font(.system(size: 12))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        } else {
                            Text("System default")
                                .font(.system(size: 12))
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                        }
                    }

                    Spacer()

                    Image(systemName: "chevron.right")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(MSCRemoteStyle.textTertiary)
                        .accessibilityHidden(true)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
        }
        .mscCard()
    }

    private var broadcastJarCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "Xbox Broadcast")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            HStack(spacing: MSCRemoteStyle.spaceMD) {
                Image(systemName: "xbox.logo")
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(MSCRemoteStyle.accent)
                    .frame(width: 28)
                    .accessibilityHidden(true)

                VStack(alignment: .leading, spacing: 3) {
                    Text("MCXboxBroadcast JAR")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(MSCRemoteStyle.textPrimary)
                    Group {
                        if isDownloadingBroadcastJar || broadcastJarStatus?.downloading == true {
                            Text("Downloading…")
                                .foregroundStyle(MSCRemoteStyle.accent)
                        } else if let status = broadcastJarStatus {
                            if status.installed, let name = status.filename {
                                Text(name)
                                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            } else {
                                Text("Not downloaded")
                                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                            }
                        } else {
                            Text("Checking…")
                                .foregroundStyle(MSCRemoteStyle.textTertiary)
                        }
                    }
                    .font(.system(size: 12))
                }

                Spacer()

                if isDownloadingBroadcastJar || broadcastJarStatus?.downloading == true {
                    ProgressView()
                        .progressViewStyle(.circular)
                        .scaleEffect(0.8)
                        .tint(MSCRemoteStyle.accent)
                } else {
                    Button {
                        Task { await startBroadcastJarDownload() }
                    } label: {
                        Text(broadcastJarStatus?.installed == true ? "Update" : "Download")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.accent)
                            .padding(.horizontal, MSCRemoteStyle.spaceSM)
                            .padding(.vertical, 6)
                            .background(MSCRemoteStyle.accentDim)
                            .clipShape(Capsule())
                            .overlay(Capsule().strokeBorder(MSCRemoteStyle.accent.opacity(0.25), lineWidth: 1))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        .mscCard()
    }

    private func settingsNavRow(title: String, subtitle: String, icon: String) -> some View {
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

    private var notificationsCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .center) {
                MSCSectionHeader(title: "Notifications")
                Spacer()
                Button {
                    hapticLight()
                    NotificationManager.shared.requestPermission()
                } label: {
                    Text("Request")
                        .font(.system(size: 10, weight: .semibold, design: .monospaced))
                        .foregroundStyle(MSCRemoteStyle.accent)
                        .padding(.horizontal, MSCRemoteStyle.spaceSM)
                        .padding(.vertical, 5)
                        .background(MSCRemoteStyle.accentDim)
                        .clipShape(Capsule())
                        .overlay(
                            Capsule().strokeBorder(MSCRemoteStyle.accent.opacity(0.25), lineWidth: 1)
                        )
                }
            }
            .padding(.bottom, MSCRemoteStyle.spaceMD)

            Text("Notifications fire when the app detects a change on its next poll. Requires notification permission.")
                .font(.system(size: 11))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, MSCRemoteStyle.spaceLG)

            VStack(spacing: 0) {
                notificationRow(
                    title: "Server went offline",
                    subtitle: "Fires when a running server stops unexpectedly.",
                    isOn: $settings.notifyServerWentOffline
                )

                Divider()
                    .background(MSCRemoteStyle.borderSubtle)
                    .padding(.vertical, MSCRemoteStyle.spaceSM)

                notificationRow(
                    title: "Server came online",
                    subtitle: "Fires when the server starts or restarts.",
                    isOn: $settings.notifyServerCameOnline
                )

                Divider()
                    .background(MSCRemoteStyle.borderSubtle)
                    .padding(.vertical, MSCRemoteStyle.spaceSM)

                notificationRow(
                    title: "Player joined",
                    subtitle: "Fires for each new player connecting. Can be noisy on busy servers.",
                    isOn: $settings.notifyPlayerJoined
                )
            }

            Text("Notifications fire while MSC Remote is open or was recently opened — iOS pauses background apps. For always-on alerts, watch for notifications on your Mac.")
                .font(.system(size: 11))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.top, MSCRemoteStyle.spaceMD)
        }
        .mscCard()
    }

    private func notificationRow(
        title: String,
        subtitle: String,
        isOn: Binding<Bool>
    ) -> some View {
        HStack(alignment: .top, spacing: MSCRemoteStyle.spaceMD) {
            VStack(alignment: .leading, spacing: 3) {
                Text(title)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(MSCRemoteStyle.textPrimary)
                Text(subtitle)
                    .font(.system(size: 11))
                    .foregroundStyle(MSCRemoteStyle.textTertiary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer()
            Toggle("", isOn: isOn)
                .labelsHidden()
                .tint(MSCRemoteStyle.accent)
                .onChange(of: isOn.wrappedValue) { _, _ in
                    settings.saveNotificationPreferences()
                }
                .accessibilityLabel(title)
                .accessibilityHint(subtitle)
        }
    }

    private var accentColorCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            MSCSectionHeader(title: "App Accent Colour")
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            Text("Tints buttons, tabs, and highlights throughout the app.")
                .font(.system(size: 12))
                .foregroundStyle(MSCRemoteStyle.textTertiary)
                .padding(.bottom, MSCRemoteStyle.spaceMD)

            let columns = Array(repeating: GridItem(.flexible(), spacing: MSCRemoteStyle.spaceSM), count: 4)
            let isCustom = !SettingsStore.accentPresets.map(\.hex).contains(settings.accentColorHex)
            LazyVGrid(columns: columns, spacing: MSCRemoteStyle.spaceSM) {
                ForEach(SettingsStore.accentPresets, id: \.hex) { preset in
                    let isSelected = settings.accentColorHex == preset.hex
                    Button {
                        settings.setAccentColor(preset.hex)
                    } label: {
                        accentSwatch(color: Color(hex: preset.hex), name: preset.name, isSelected: isSelected)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel(preset.name)
                    .accessibilityValue(isSelected ? "Selected" : "")
                }

                // Custom colour picker slot
                let customColor = isCustom ? Color(hex: settings.accentColorHex) : Color.white
                ColorPicker(selection: Binding(
                    get: { isCustom ? Color(hex: settings.accentColorHex) : Color.white },
                    set: { color in
                        if let hex = color.toHex() { settings.setAccentColor(hex) }
                    }
                ), supportsOpacity: false) { EmptyView() }
                .labelsHidden()
                .overlay {
                    accentSwatch(
                        color: isCustom ? customColor : Color(white: 0.4),
                        name: "Custom",
                        isSelected: isCustom,
                        icon: isCustom ? nil : "eyedropper"
                    )
                    .allowsHitTesting(false)
                }
            }
        }
        .mscCard()
    }

    @ViewBuilder
    private func accentSwatch(color: Color, name: String, isSelected: Bool, icon: String? = nil) -> some View {
        VStack(spacing: 6) {
            ZStack {
                Circle()
                    .fill(color)
                    .frame(width: 36, height: 36)
                if isSelected {
                    Circle()
                        .strokeBorder(.white, lineWidth: 2.5)
                        .frame(width: 36, height: 36)
                    Image(systemName: "checkmark")
                        .font(.system(size: 12, weight: .bold))
                        .foregroundStyle(.white)
                } else if let icon {
                    Image(systemName: icon)
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(.white.opacity(0.7))
                }
            }
            Text(name)
                .font(.system(size: 10, weight: isSelected ? .semibold : .regular))
                .foregroundStyle(isSelected ? color : MSCRemoteStyle.textTertiary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, MSCRemoteStyle.spaceSM)
        .background(isSelected ? color.opacity(0.1) : Color.clear)
        .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusSM, style: .continuous)
                .strokeBorder(isSelected ? color.opacity(0.4) : Color.clear, lineWidth: 1)
        )
    }

    private var footerText: some View {
        Text("TempleTech · MSC REMOTE")
            .font(.system(size: 10, weight: .regular, design: .monospaced))
            .foregroundStyle(MSCRemoteStyle.textTertiary)
            .frame(maxWidth: .infinity, alignment: .center)
    }

    private func loadJavaConfigIfAdmin() async {
        guard vm.connectedRole == "admin",
              let url = settings.resolvedBaseURL(),
              let token = settings.resolvedToken() else { return }
        isLoadingJavaConfig = true
        async let configTask = vm.fetchJavaConfig(baseURL: url, token: token)
        async let runtimesTask = vm.fetchJavaRuntimes(baseURL: url, token: token)
        let (config, runtimes) = await (configTask, runtimesTask)
        currentJavaConfig = config
        javaRuntimes = runtimes?.runtimes ?? []
        isLoadingJavaConfig = false
    }

    private func openJavaRuntimePicker() async {
        guard let url = settings.resolvedBaseURL(), let token = settings.resolvedToken() else { return }
        isLoadingJavaConfig = true
        async let configTask = vm.fetchJavaConfig(baseURL: url, token: token)
        async let runtimesTask = vm.fetchJavaRuntimes(baseURL: url, token: token)
        let (config, runtimes) = await (configTask, runtimesTask)
        currentJavaConfig = config
        javaRuntimes = runtimes?.runtimes ?? []
        isLoadingJavaConfig = false
        showJavaRuntimePicker = true
    }

    private func loadBroadcastJarStatusIfAdmin() async {
        guard vm.connectedRole == "admin",
              let url = settings.resolvedBaseURL(),
              let token = settings.resolvedToken() else { return }
        broadcastJarStatus = await vm.fetchBroadcastJarStatus(baseURL: url, token: token)
    }

    private func startBroadcastJarDownload() async {
        guard let url = settings.resolvedBaseURL(), let token = settings.resolvedToken() else { return }
        isDownloadingBroadcastJar = true
        hapticLight()
        let result = await vm.triggerBroadcastJarDownload(baseURL: url, token: token)
        isDownloadingBroadcastJar = false
        if result?.success == true {
            hapticSuccess()
            presentToast("JAR downloaded")
            broadcastJarStatus = await vm.fetchBroadcastJarStatus(baseURL: url, token: token)
        } else {
            hapticError()
            let msg = result?.message ?? "Download failed"
            presentToast(msg == "already_downloading" ? "Already downloading" : "Download failed")
        }
    }

    private func applyJavaRuntime(executablePath: String?) {
        guard let url = settings.resolvedBaseURL(), let token = settings.resolvedToken() else { return }
        showJavaRuntimePicker = false
        currentJavaConfig = JavaConfigResponseDTO(executablePath: executablePath)
        hapticSuccess()
        presentToast("Java runtime updated")
        Task {
            _ = await vm.applyJavaConfig(baseURL: url, token: token, executablePath: executablePath)
        }
    }

    @MainActor
    private func presentToast(_ text: String) {
        toastText = text
        withAnimation(.easeInOut(duration: 0.2)) { showToast = true }
        Task {
            try? await Task.sleep(nanoseconds: 1_500_000_000)
            await MainActor.run {
                withAnimation(.easeInOut(duration: 0.2)) { showToast = false }
            }
        }
    }

    private struct ToastView: View {
        let text: String

        var body: some View {
            Text(text)
                .font(.system(size: 13, weight: .medium, design: .monospaced))
                .foregroundStyle(MSCRemoteStyle.textPrimary)
                .padding(.vertical, 10)
                .padding(.horizontal, MSCRemoteStyle.spaceLG)
                .background(.ultraThinMaterial)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous))
        }
    }

    private func savePairing() {
        settings.save()
        DispatchQueue.main.async { settings.loadTokenFromKeychain() }
        recomputeSafetyWarning()
        hapticSuccess()
        withAnimation(.easeInOut(duration: 0.15)) { saveConfirmed = true }
        presentToast("Saved")
        Task {
            try? await Task.sleep(nanoseconds: 1_500_000_000)
            await MainActor.run {
                withAnimation(.easeInOut(duration: 0.2)) { saveConfirmed = false }
            }
        }
    }

    private func clearToken() {
        settings.clearToken()
        recomputeSafetyWarning()
        hapticLight()
        presentToast("Token cleared")
    }

    private func clearBaseURLAction() {
        settings.baseURLString = ""
        settings.save()
        recomputeSafetyWarning()
        hapticLight()
        presentToast("Base URL cleared")
    }

    private func runConnectionTest() {
        hapticLight()
        presentToast("Testing…")
        Task {
            await testConnection()
            if settings.lastTestWasSuccess {
                hapticSuccess()
                await MainActor.run { presentToast("Test OK") }
            } else {
                hapticError()
                await MainActor.run { presentToast("Test failed") }
            }
        }
    }

    private func pastePairingLinkFromClipboard() {
        hapticLight()
        let pasted = (UIPasteboard.general.string ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        guard !pasted.isEmpty else {
            settings.lastTestResult = "Clipboard is empty."
            settings.lastTestWasSuccess = false
            hapticError()
            presentToast("Clipboard empty")
            return
        }
        let ok = settings.applyPairingPayload(pasted)
        if ok {
            settings.lastTestResult = "Pairing imported from clipboard."
            settings.lastTestWasSuccess = true
            DispatchQueue.main.async { settings.loadTokenFromKeychain() }
            recomputeSafetyWarning()
            hapticSuccess()
            presentToast("Pairing imported")
        } else {
            settings.lastTestResult = "Clipboard did not contain a valid MSC pairing link."
            settings.lastTestWasSuccess = false
            hapticError()
            presentToast("Invalid pairing link")
        }
    }

    private func recomputeSafetyWarning() {
        safetyWarning = nil
        guard let url = settings.resolvedBaseURL() else { return }
        let scheme = (url.scheme ?? "").lowercased()
        if scheme == "http" {
            if let host = url.host, !NetworkSafety.isLocalOrPrivateHost(host) {
                safetyWarning = "Blocked: HTTP only allowed for local/private hosts."
            } else {
                safetyWarning = "Using HTTP on LAN/Tailscale is OK."
            }
        }
    }

    private struct SettingsJavaRuntimePickerSheet: View {
        let currentPath: String?
        let runtimes: [JavaRuntimeDTO]
        let onSelect: (String?) -> Void

        @Environment(\.dismiss) private var dismiss

        var body: some View {
            NavigationStack {
                ZStack {
                    MSCRemoteStyle.bgBase.ignoresSafeArea()

                    ScrollView(showsIndicators: false) {
                        VStack(spacing: MSCRemoteStyle.spaceSM) {
                            runtimeRow(
                                label: "System default",
                                detail: "Use whatever Java is on PATH",
                                path: nil
                            )

                            if !runtimes.isEmpty {
                                Divider()
                                    .background(MSCRemoteStyle.borderSubtle)
                                    .padding(.horizontal, MSCRemoteStyle.spaceLG)

                                ForEach(runtimes) { runtime in
                                    runtimeRow(
                                        label: "\(runtime.versionLabel) — \(runtime.name)",
                                        detail: runtime.executablePath,
                                        path: runtime.executablePath
                                    )
                                }
                            }
                        }
                        .padding(.top, MSCRemoteStyle.spaceMD)
                        .padding(.bottom, MSCRemoteStyle.spaceLG)
                    }
                }
                .navigationTitle("Java Runtime")
                .navigationBarTitleDisplayMode(.inline)
                .toolbarBackground(MSCRemoteStyle.bgBase, for: .navigationBar)
                .toolbarColorScheme(.dark, for: .navigationBar)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Cancel") { dismiss() }
                            .foregroundStyle(MSCRemoteStyle.accent)
                    }
                }
            }
        }

        private func runtimeRow(label: String, detail: String, path: String?) -> some View {
            let isSelected = (currentPath ?? "") == (path ?? "")
            return Button {
                onSelect(path)
            } label: {
                HStack(spacing: MSCRemoteStyle.spaceMD) {
                    VStack(alignment: .leading, spacing: 3) {
                        Text(label)
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.textPrimary)
                        Text(detail)
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(MSCRemoteStyle.textTertiary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    Spacer()
                    if isSelected {
                        Image(systemName: "checkmark")
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(MSCRemoteStyle.accent)
                    }
                }
                .padding(MSCRemoteStyle.spaceMD)
                .background(isSelected ? MSCRemoteStyle.accentDim : MSCRemoteStyle.bgCard)
                .clipShape(RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: MSCRemoteStyle.radiusMD, style: .continuous)
                        .strokeBorder(isSelected ? MSCRemoteStyle.accent.opacity(0.35) : MSCRemoteStyle.borderSubtle, lineWidth: 1)
                )
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .padding(.horizontal, MSCRemoteStyle.spaceLG)
        }
    }

    private func testConnection() async {
        settings.lastTestResult = nil
        settings.lastTestWasSuccess = false
        testIsRunning = true
        defer { testIsRunning = false }
        guard let url = settings.resolvedBaseURL() else {
            settings.lastTestResult = "Base URL missing/invalid."
            return
        }
        guard let token = settings.resolvedToken() else {
            settings.lastTestResult = "Token missing."
            return
        }
        do {
            let client = try RemoteAPIClient(baseURL: url, token: token)
            let status = try await client.getStatus()
            settings.lastTestResult = "OK  running=\(status.running)  server=\(status.activeServerId ?? "nil")  pid=\(status.pid.map(String.init) ?? "nil")"
            settings.lastTestWasSuccess = true
        } catch {
            settings.lastTestResult = error.localizedDescription
            settings.lastTestWasSuccess = false
        }
    }
}
