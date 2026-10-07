<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { observeServerRun, forgetHostRuns } from './lib/sections/shared/server-uptime';
  import { bundleIdentity } from './lib/bundle-identity';
  import { ApiClient, ApiError } from './lib/api/client';
  import ApplicationShell from './lib/components/ApplicationShell.svelte';
  import { Button, Field, Sheet } from './lib/components/base';
  import FirstLaunchGate from './lib/help/FirstLaunchGate.svelte';
  import SplashGate from './lib/help/SplashGate.svelte';
  import { createClientRouter } from './routes/router';
  import UnknownSection from './routes/UnknownSection.svelte';
  import FirstStartSheet from './lib/sections/server-editor/FirstStartSheet.svelte';
  import StartupFailureSheet from './lib/sections/server-editor/StartupFailureSheet.svelte';
  import BroadcastAuthSheet from './lib/sections/server-editor/BroadcastAuthSheet.svelte';
  import ServerEditorSheet from './lib/sections/server-editor/ServerEditorSheet.svelte';
  import { buildSectionPath } from './lib/navigation/route';
  import {
    AgentHealthTimeoutError,
    createAgentTransport,
    getPlatform,
    LOCAL_AGENT_ORIGIN,
    prepareLocalAgent,
    type AgentReadiness,
    type AgentServiceStatus,
  } from './lib/platform';
  import {
    DesktopSessionAuth,
    loadTauriDesktopCredentialBridge,
    type RemoteDesktopPairingResult,
  } from './lib/auth/desktop';
  import { clearClientPreferences, HostStore } from './lib/hosts/registry';
  import { loadSavedRemoteHosts } from './lib/hosts/saved';
  import { HostConnectionManager } from './lib/hosts/connection';
  import { formatConnectionFailure } from './lib/hosts/connection-errors';
  import { HostConnectionOrchestrator } from './lib/hosts/orchestration';
  import {
    createLocalHostRecord,
    hostManagementUrl,
    LOCAL_HOST_ID,
    type HostId,
    type HostRecord,
    type HostRoute,
    type RemoteHostConnectionInput,
  } from './lib/hosts/types';
  import ManageSheet from './lib/sections/fleet/ManageSheet.svelte';
  import AppSettingsSheet from './lib/sections/app-settings/AppSettingsSheet.svelte';
  import UninstallSheet from './lib/sections/app-settings/UninstallSheet.svelte';
  import ResetSheet from './lib/sections/app-settings/ResetSheet.svelte';
  import { restoreAccent } from './lib/styles/accent';
  import { bannerColorFor } from './lib/styles/bannerColor';
  import { waitForRunningState } from './lib/lifecycle/serverLifecycle';
  import { PRIMARY_TABS } from './lib/navigation/primaryTabs';
  import { selectAvailableServerId } from './lib/navigation/serverSelection';
  import {
    readTabPreloadPreference,
    scheduleTabPreload,
    setTabPreloadPreference,
  } from './lib/navigation/tabPreloading';
  import type { Capabilities, NavigationContext, SectionDescriptor } from './lib/navigation/types';
  import type { Schema, ScreenApi } from './lib/sections/shared/types';
  import './lib/sections/shared/screen.css';

  const sections: SectionDescriptor[] = [
    {
      id: 'home',
      label: 'Home',
      segment: 'home',
      scope: 'server',
      load: () => import('./lib/sections/home/HomeSection.svelte'),
    },
    {
      id: 'agent-setup',
      label: 'Agent home',
      segment: 'local-agent',
      scope: 'host',
      load: () => import('./lib/sections/setup/AgentSetupSection.svelte'),
    },
    {
      id: 'handbook',
      label: 'Handbook',
      segment: 'handbook',
      scope: 'server',
      load: () => import('./lib/sections/handbook/HelpSection.svelte'),
    },
    {
      id: 'console',
      label: 'Console',
      segment: 'console',
      scope: 'server',
      load: () => import('./lib/sections/console/ConsoleSection.svelte'),
    },
    {
      id: 'performance',
      label: 'Performance',
      segment: 'performance',
      scope: 'server',
      load: () => import('./lib/sections/performance/PerformanceSection.svelte'),
    },
    {
      id: 'players-online',
      label: 'Players',
      segment: 'players-online',
      scope: 'server',
      load: () => import('./lib/sections/players-online/PlayersOnlineSection.svelte'),
    },
    {
      id: 'worlds',
      label: 'Worlds',
      segment: 'worlds',
      scope: 'server',
      requiredPermissions: ['worlds'],
      load: () => import('./lib/sections/worlds/WorldsSection.svelte'),
    },
    {
      id: 'backups',
      label: 'Backups',
      segment: 'backups',
      scope: 'server',
      requiredPermissions: ['worlds'],
      load: () => import('./lib/sections/backups/BackupsSection.svelte'),
    },
    {
      id: 'addons',
      label: 'Add-ons',
      segment: 'addons',
      scope: 'server',
      requiredPermissions: ['addons'],
      load: () => import('./lib/sections/addons/AddonsSection.svelte'),
    },
    {
      id: 'components',
      label: 'Components',
      segment: 'components',
      scope: 'server',
      load: () => import('./lib/sections/components/ComponentsSection.svelte'),
    },
    {
      id: 'settings',
      label: 'Settings',
      segment: 'settings',
      scope: 'server',
      requiredPermissions: ['settings'],
      load: () => import('./lib/sections/settings/SettingsSection.svelte'),
    },
    {
      id: 'files',
      label: 'Files',
      segment: 'files',
      scope: 'server',
      requiredPermissions: ['admin'],
      load: () => import('./lib/sections/files/FilesSection.svelte'),
    },
    {
      id: 'health',
      label: 'Health',
      segment: 'health',
      scope: 'server',
      load: () => import('./lib/sections/health/HealthSection.svelte'),
    },
    {
      id: 'connectivity',
      label: 'Networking',
      segment: 'connectivity',
      scope: 'server',
      requiredPermissions: ['networking'],
      load: () => import('./lib/sections/connectivity/ConnectivitySection.svelte'),
    },
    {
      id: 'access',
      label: 'Access',
      segment: 'access',
      scope: 'server',
      requiredPermissions: ['admin'],
      load: () => import('./lib/sections/access/AccessSection.svelte'),
    },
  ];
  const router = createClientRouter(sections);
  const localAgentHostId = LOCAL_HOST_ID;

  // Keeps every host's connection/credential/cache state (D-013). Tauri
  // rehydrates remote host metadata from localStorage on startup; credentials
  // are never put there and remain in the native secret store.
  const hostStore = new HostStore();
  const hostConnectionManager = new HostConnectionManager();
  let hosts: readonly HostRecord[] = [];
  let hostId = localAgentHostId;
  let isDesktopShell = false;
  let manageOpen = false;
  let settingsOpen = false;
  let resetOpen = false;
  let uninstallOpen = false;
  let headerEditingServer: Schema['ServerDTO'] | undefined;
  let addressesVisible = false;
  let sshPasswordPromptHostId: HostId | null = null;
  let sshPasswordInput = '';
  let sshPasswordPromptError = '';
  let sshPasswordPromptBusy = false;

  function promptForSshPassword(id: HostId, error = ''): void {
    sshPasswordPromptHostId = id;
    sshPasswordInput = '';
    sshPasswordPromptError = error;
    shellMessage = `Enter the SSH password to reconnect to ${hosts.find((host) => host.id === id)?.displayName ?? 'this computer'}.`;
    void selectSection('agent-setup');
  }

  function closeSshPasswordPrompt(): void {
    const pendingHostId = sshPasswordPromptHostId;
    sshPasswordPromptHostId = null;
    sshPasswordInput = '';
    sshPasswordPromptError = '';
    if (pendingHostId && pendingHostId === hostId) void switchHost(localAgentHostId);
  }

  async function submitSshPassword(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const pendingHostId = sshPasswordPromptHostId;
    if (!pendingHostId || !sshPasswordInput) {
      sshPasswordPromptError = 'Enter the password for this computer.';
      return;
    }
    hostConnectionManager.rememberSessionPassword(pendingHostId, sshPasswordInput);
    sshPasswordPromptBusy = true;
    sshPasswordPromptError = '';
    const generation = hostOrchestrator.nextGeneration();
    await initializeClient(generation);
    if (generation !== hostOrchestrator.currentGeneration) return;
    sshPasswordPromptBusy = false;
    if (clientReady && hostId === pendingHostId) {
      sshPasswordPromptHostId = null;
      sshPasswordInput = '';
    } else if (!sshPasswordPromptError) {
      sshPasswordPromptHostId = null;
      sshPasswordInput = '';
    }
  }

  function toggleAddresses(): void {
    addressesVisible = !addressesVisible;
  }

  function refreshHosts(): void {
    hosts = hostStore.listHosts();
  }

  const hostOrchestrator: HostConnectionOrchestrator = new HostConnectionOrchestrator({
    store: hostStore,
    connectionManager: hostConnectionManager,
    localHostId: localAgentHostId,
    hosts: () => hosts,
    selectedHostId: () => hostId,
    selectHostId: (id) => (hostId = id),
    desktopShell: () => isDesktopShell,
    agentReadiness: () => agentReadiness,
    setShellMessage: (message) => (shellMessage = message),
    clearLoadedSections: () => (loadedSections = []),
    refreshHosts,
    initializeClient: (generation) => initializeClient(generation),
    selectSetupSection: (generation) => selectSection('agent-setup', true, generation),
  });

  async function switchHost(id: HostId): Promise<void> {
    await hostOrchestrator.switchHost(id);
  }

  async function connectRemoteHost(
    input: RemoteHostConnectionInput,
  ): Promise<RemoteDesktopPairingResult | void> {
    return hostOrchestrator.connectRemoteHost(input);
  }

  async function selectHostRoute(id: HostId, route: HostRoute): Promise<void> {
    await hostOrchestrator.selectHostRoute(id, route);
  }

  async function pairAgain(pairingCode: string): Promise<void> {
    await hostOrchestrator.pairAgain(pairingCode);
  }

  async function removeRemoteHost(id: HostId): Promise<void> {
    await hostOrchestrator.removeRemoteHost(id);
  }

  async function removeCurrentRemoteHost(): Promise<void> {
    await hostOrchestrator.removeCurrentRemoteHost();
  }

  function openReset(): void {
    settingsOpen = false;
    resetOpen = true;
  }

  async function createClient(id: string): Promise<ApiClient> {
    const transport = await createAgentTransport(id);
    return new ApiClient({ ...transport, hostId: transport.hostId });
  }

  const defaultStatus: Schema['RemoteAPIStatus'] = { running: false };

  let client: ApiClient | undefined;
  let clientReady = false;
  let agentReadiness: AgentReadiness = 'starting';
  let splashComplete = false;

  function completeSplash(): void {
    splashComplete = true;
  }

  function requireClient(): ApiClient {
    if (!client) throw new Error('The selected host client is still initializing.');
    return client;
  }

  function createScreenApi(): ScreenApi {
    return {
      get: <T,>(path: string) => requireClient().requestJson<T>('GET', path),
      post: <T,>(path: string, body?: unknown) =>
        requireClient().requestJson<T>('POST', path, { body }),
      getBytes: (path: string) => requireClient().requestBytes('GET', path),
      resourceUrl: (path: string) => requireClient().resourceUrl(path),
      upload: (purpose, bytes, options) =>
        requireClient().stagedUpload({ purpose, ...options }, bytes),
      uploadFile: (purpose, source, options) => {
        const { onProgress, chunkSizeBytes, signal, ...uploadOptions } = options ?? {};
        return requireClient().stagedUploadFromFile({ purpose, ...uploadOptions }, source, {
          onProgress,
          chunkSizeBytes,
          signal,
        });
      },
      download: (id, maxBytes) => requireClient().downloadBytes(id, maxBytes),
    };
  }

  const screenApi: ScreenApi = createScreenApi();

  let activeSection = '';
  let sectionRequest = 0;
  // Keep visited tab instances alive for this server so their lightweight UI
  // state and loaded responses are available immediately on return. The list
  // is cleared before every accepted server/host switch, keeping memory and
  // displayed data scoped to the current server.
  type LoadedSection = { id: string; component: any };
  let loadedSections: LoadedSection[] = [];
  let selectedServerId = '';
  let permissions: readonly string[] = [];
  let capabilities: Capabilities | null = null;
  let shellMessage = 'Connecting to the selected host…';
  let servers: readonly Schema['ServerDTO'][] = [];
  let status: Schema['RemoteAPIStatus'] = defaultStatus;
  let healthRefreshVersion = 0;
  let initiationServer: Schema['ServerDTO'] | undefined;
  let initiationVisible = false;
  let initiationComplete = false;
  let broadcastAuth: Schema['BroadcastAuthPromptDTO'] | undefined;
  let broadcastAuthTimer: ReturnType<typeof setInterval> | undefined;
  let broadcastAuthRequest = 0;
  let broadcastAuthContextServerId = '';
  let hiddenBroadcastAuthKey = '';
  let startupFailure:
    | {
        serverName: string;
        errorCode: string;
        message: string;
        problems?: Schema['StartupProblemDTO'][];
      }
    | undefined;
  let preloadTabs = readTabPreloadPreference();
  let cancelTabPreload: (() => void) | undefined;

  $: activeServer = servers.find((server) => server.id === selectedServerId);

  $: broadcastAuthShouldPoll =
    clientReady &&
    Boolean(activeServer) &&
    !manageOpen &&
    !headerEditingServer &&
    !initiationServer;

  async function refreshBroadcastAuthPrompt(): Promise<void> {
    const request = ++broadcastAuthRequest;
    const serverId = selectedServerId;
    if (!broadcastAuthShouldPoll) {
      broadcastAuth = undefined;
      return;
    }
    if (broadcastAuthContextServerId !== serverId) {
      broadcastAuthContextServerId = serverId;
      hiddenBroadcastAuthKey = '';
    }
    try {
      const next = await screenApi.get<Schema['BroadcastAuthPromptDTO']>(
        '/v1/broadcast/auth-prompt',
      );
      if (
        request !== broadcastAuthRequest ||
        serverId !== selectedServerId ||
        !broadcastAuthShouldPoll
      ) {
        return;
      }
      if (!next.isPresent) {
        broadcastAuth = undefined;
        hiddenBroadcastAuthKey = '';
        return;
      }
      const key = next.code ?? next.linkURL ?? 'present';
      if (key !== hiddenBroadcastAuthKey) broadcastAuth = next;
    } catch {
      // The shell can continue while the agent is reconnecting. The next poll
      // will pick up a newly available device-code prompt.
    }
  }

  function closeBroadcastAuthPrompt(): void {
    const prompt = broadcastAuth;
    hiddenBroadcastAuthKey = prompt?.code ?? prompt?.linkURL ?? 'present';
    broadcastAuth = undefined;
  }

  $: if (broadcastAuthShouldPoll && broadcastAuthTimer === undefined) {
    void refreshBroadcastAuthPrompt();
    broadcastAuthTimer = setInterval(() => void refreshBroadcastAuthPrompt(), 1000);
  }
  $: if (!broadcastAuthShouldPoll && broadcastAuthTimer !== undefined) {
    clearInterval(broadcastAuthTimer);
    broadcastAuthTimer = undefined;
    broadcastAuth = undefined;
    broadcastAuthRequest += 1;
  }

  type HostResetResult = {
    operationId: string;
    hostId: string;
    mode: 'configuration' | 'everything';
    agentState: 'restarting' | 'needs_pairing' | 'unavailable';
    message: string;
  };

  async function resetClientState(): Promise<void> {
    const rememberedHostIds = hosts.map((host) => host.id);
    if (isDesktopShell) {
      const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
      await auth.forgetCredentials(rememberedHostIds, true);
    }
    hostStore.reset();
    clearClientPreferences();
    // Re-entering through the normal startup path recreates only the local
    // connection placeholder and reopens first-launch from a clean profile.
    window.location.reload();
  }

  async function completeHostReset(
    result: HostResetResult,
    resetClientAfterHost = false,
  ): Promise<void> {
    let cleanupError = '';
    let credentialCleanupFailed = false;
    if (isDesktopShell) {
      try {
        const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
        await auth.forgetCredentials(
          resetClientAfterHost ? hosts.map((host) => host.id) : [result.hostId],
          resetClientAfterHost || hostId === localAgentHostId,
        );
      } catch (error) {
        credentialCleanupFailed = true;
        cleanupError = `The host reset completed, but this desktop could not forget its old credential: ${String(error)}`;
      }
    }

    settingsOpen = false;
    resetOpen = false;
    clientReady = false;
    client = undefined;
    capabilities = null;
    permissions = [];
    servers = [];
    selectedServerId = '';
    status = defaultStatus;
    forgetHostRuns(hostId);

    if (isLocalHostForReset() && isDesktopShell && result.mode === 'everything') {
      let localServiceRemoved = true;
      try {
        const platform = await getPlatform();
        const serviceStatus = await platform.agentServiceStatus();
        if (serviceStatus.state === 'running') {
          await platform.manageAgentService('stop');
        }
        const uninstallStatus = await platform.manageAgentService('uninstall');
        agentReadiness = uninstallStatus.state === 'not-installed' ? 'missing' : 'unavailable';
        if (agentReadiness === 'unavailable') {
          throw new Error(uninstallStatus.detail);
        }
        shellMessage = cleanupError || 'The local host was reset. Install the agent to continue.';

        if (resetClientAfterHost && !credentialCleanupFailed) {
          hostStore.reset();
          clearClientPreferences();
          await platform.quitApplication();
          return;
        }
      } catch (error) {
        localServiceRemoved = false;
        agentReadiness = 'unavailable';
        shellMessage = `The host was reset, but the local agent service could not be removed: ${String(error)}`;
      }
      if (resetClientAfterHost && localServiceRemoved && !credentialCleanupFailed) {
        hostStore.reset();
        clearClientPreferences();
        window.location.reload();
        return;
      }
      hostStore.updateConnection(hostId, 'error');
      await selectSection('agent-setup');
      return;
    }

    hostStore.updateConnection(hostId, 'error');
    if (isLocalHostForReset() && isDesktopShell) {
      agentReadiness = 'starting';
      shellMessage =
        cleanupError || 'The local host was reset. Reconnecting with its new identity…';
      await selectSection('agent-setup');
      void initializeClient();
    } else {
      agentReadiness = 'unavailable';
      shellMessage =
        cleanupError || `Host reset complete. Pair ${hostLabelForCurrentHost()} again.`;
      await selectSection('agent-setup');
    }
  }

  function isLocalHostForReset(): boolean {
    return hostId === localAgentHostId;
  }

  function hostLabelForCurrentHost(): string {
    return hosts.find((host) => host.id === hostId)?.displayName ?? hostId;
  }

  $: navigationContext = capabilities
    ? ({
        hostId,
        serverId: selectedServerId,
        permissions,
        capabilities,
      } satisfies NavigationContext)
    : null;

  function currentNavigationContext(): NavigationContext | null {
    if (!capabilities) return null;
    return { hostId, serverId: selectedServerId, permissions, capabilities };
  }
  $: visibleSections = navigationContext ? router.visibleSections(navigationContext) : [];
  $: canControl =
    permissions.length === 0 ||
    permissions.includes('serverControl') ||
    permissions.includes('admin');
  $: primaryTabs = PRIMARY_TABS.map((tab) => ({
    ...tab,
    available: visibleSections.some((section) => section.id === tab.id),
  }));
  function readBannerColor(host: string, server: string): string {
    return bannerColorFor(host, server);
  }
  $: bannerColor = readBannerColor(hostId, selectedServerId);
  // Referencing servers/status/hostId directly (not just through hostSummaries'
  // internals) makes Svelte re-run this when the active host's live state
  // changes, not only when a host is added or removed.
  $: currentHostSummaries = ((): Map<HostId, { connection: string; serverCount: number }> => {
    void servers;
    void status;
    void hostId;
    return hosts.length ? hostOrchestrator.hostSummaries() : new Map();
  })();

  function readinessForService(status: AgentServiceStatus): AgentReadiness {
    switch (status.state) {
      case 'not-installed':
        return 'missing';
      case 'stopped':
        return 'stopped';
      case 'running':
        return 'starting';
      case 'unavailable':
        return 'unavailable';
    }
  }

  function readinessForError(error: unknown): AgentReadiness {
    if (
      error instanceof ApiError &&
      (error.status === 426 || error.error.code === 'client_version_unsupported')
    ) {
      return 'incompatible';
    }
    if (error instanceof AgentHealthTimeoutError) return 'starting';
    return 'unavailable';
  }

  async function waitForStartOperation(
    operationId: string,
  ): Promise<Schema['OperationDTO'] | undefined> {
    const deadline = Date.now() + 10_000;
    for (;;) {
      const operation = await screenApi.get<Schema['OperationDTO']>(
        `/v1/operations/${encodeURIComponent(operationId)}`,
      );
      if (
        operation.state === 'succeeded' ||
        operation.state === 'failed' ||
        operation.state === 'cancelled'
      ) {
        return operation;
      }
      if (Date.now() >= deadline) return undefined;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  }

  async function restoreHostContext(
    generation: number,
    selectedHostId: HostId,
    selectedClient: ApiClient,
  ): Promise<boolean> {
    try {
      const rememberedServerId = hostStore.getState(selectedHostId).cache.activeServerId;
      let nextCapabilities = await selectedClient.getCapabilities();
      if (generation !== hostOrchestrator.currentGeneration) return false;
      const me = await selectedClient.requestJson<{ permissions: string[] }>('GET', '/v1/me');
      if (generation !== hostOrchestrator.currentGeneration) return false;
      const nextServers = await selectedClient.requestJson<Schema['ServerDTO'][]>(
        'GET',
        '/v1/servers',
      );
      if (generation !== hostOrchestrator.currentGeneration) return false;
      let nextStatus = await selectedClient.requestJson<Schema['RemoteAPIStatus']>(
        'GET',
        '/v1/status',
      );
      if (generation !== hostOrchestrator.currentGeneration) return false;
      const nextServerId = selectAvailableServerId(
        nextServers,
        nextStatus.activeServerId,
        rememberedServerId ?? selectedServerId,
      );
      // The sidebar's fallback selection must also become the agent's active
      // server before section APIs can serve that server's data.
      if (nextServerId && nextServerId !== nextStatus.activeServerId) {
        await selectedClient.requestJson('POST', '/v1/active-server', { body: { serverId: nextServerId } });
        if (generation !== hostOrchestrator.currentGeneration) return false;
        nextStatus = await selectedClient.requestJson<Schema['RemoteAPIStatus']>('GET', '/v1/status');
        nextCapabilities = await selectedClient.getCapabilities();
        if (generation !== hostOrchestrator.currentGeneration) return false;
      }
      client = selectedClient;
      capabilities = nextCapabilities;
      permissions = me.permissions;
      servers = nextServers;
      status = nextStatus;
      observeServerRun(hostId, nextStatus.activeServerId ?? '', nextStatus.running);
      selectedServerId = nextServerId;
      hostStore.setServers(selectedHostId, nextServers);
      if (nextServerId) hostStore.selectServer(selectedHostId, nextServerId);
      agentReadiness = 'ready';
      shellMessage = `Connected to ${hosts.find((host) => host.id === selectedHostId)?.displayName ?? selectedHostId}`;
      hostStore.updateConnection(selectedHostId, 'connected');
      await selectFromLocation(generation);
      return true;
    } catch (error) {
      if (generation !== hostOrchestrator.currentGeneration) return false;
      capabilities = null;
      permissions = [];
      servers = [];
      selectedServerId = '';
      status = defaultStatus;
      forgetHostRuns(hostId);
      agentReadiness = readinessForError(error);
      shellMessage = `Unable to establish the selected host context: ${formatConnectionFailure(error)}`;
      hostStore.updateConnection(hostId, 'error');
      await selectSection('agent-setup');
      return false;
    }
  }

  async function selectServer(id: string): Promise<void> {
    try {
      await screenApi.post('/v1/active-server', { serverId: id });
      loadedSections = [];
      selectedServerId = id;
      status = { ...status, activeServerId: id };
      // Capabilities carry the selected runtime's relative-time support, so
      // refresh them after a server switch before showing time shortcuts.
      capabilities = await screenApi.get<Capabilities>('/v1/capabilities');
      const section = router.get(activeSection);
      if (section) {
        history.pushState({}, '', buildSectionPath(section, hostId, selectedServerId));
        await selectSection(section.id, false);
      }
    } catch (error) {
      shellMessage = `Unable to switch servers: ${String(error)}`;
    }
  }

  async function lifecycle(action: 'start' | 'stop'): Promise<void> {
    try {
      const accepted = await screenApi.post<Schema['SimpleResult']>(
        action === 'start' ? '/v1/start' : '/v1/stop',
      );
      if (action === 'start' && accepted.operationId) {
        const operation = await waitForStartOperation(accepted.operationId);
        if (operation?.state === 'failed' || operation?.state === 'cancelled') {
          startupFailure = {
            serverName: activeServer?.name ?? 'Server',
            errorCode: operation.error?.code ?? 'server_start_failed',
            message: operation.error?.message ?? 'The server did not become ready.',
          };
          shellMessage = startupFailure.message;
          return;
        }
      }
      const nextStatus = await waitForRunningState(
        () => screenApi.get<Schema['RemoteAPIStatus']>('/v1/status'),
        action === 'start',
      );
      status = nextStatus;
      observeServerRun(hostId, nextStatus.activeServerId ?? '', nextStatus.running);
      if (action === 'start' && nextStatus.running && activeServer?.serverType === 'java') {
        try {
          const diagnosis = await screenApi.get<Schema['HealthProblemsResponseDTO']>(
            '/v1/health/problems',
          );
          const helperProblem = diagnosis.isSoftFail
            ? diagnosis.problems.find((problem) =>
                /geyser|floodgate/i.test(problem.offenderName),
              )
            : undefined;
          if (helperProblem) {
            startupFailure = {
              serverName: activeServer.name,
              errorCode: 'geyser_plugin_failed',
              message: `${helperProblem.offenderName} did not load.`,
              problems: [helperProblem],
            };
            shellMessage = startupFailure.message;
          }
        } catch {
          // A diagnosis lookup must not turn a successful server start into a failure.
        }
      }
      if (action === 'start' && !nextStatus.running) {
        startupFailure = {
          serverName: activeServer?.name ?? 'Server',
          errorCode: 'server_startup_timeout',
          message: accepted.operationId
            ? 'The server did not become ready before the start check timed out.'
            : 'The server did not become ready after the start request.',
        };
        shellMessage = startupFailure.message;
      }
    } catch (error) {
      if (action === 'start') {
        const apiError = error instanceof ApiError ? error.error : undefined;
        startupFailure = {
          serverName: activeServer?.name ?? 'Server',
          errorCode: apiError?.code ?? 'server_start_failed',
          message: apiError?.message ?? (error instanceof Error ? error.message : String(error)),
        };
        shellMessage = startupFailure.message;
      } else {
        shellMessage = `Unable to ${action} the server: ${String(error)}`;
      }
    } finally {
      // The health card describes the completed lifecycle attempt, not just
      // the shell's cached status snapshot. The Overview consumes this
      // version as a targeted refresh signal while retaining its tab instance.
      healthRefreshVersion += 1;
    }
  }

  async function retryStartup(): Promise<void> {
    startupFailure = undefined;
    if (status.running) await lifecycle('stop');
    if (!status.running) await lifecycle('start');
  }

  function openInitiation(): void {
    if (!activeServer || status.running) return;
    if (initiationServer?.id === activeServer.id) {
      initiationVisible = true;
      return;
    }
    initiationServer = activeServer;
    initiationVisible = true;
    initiationComplete = false;
  }

  function resumeInitiation(): void {
    if (initiationServer) {
      initiationVisible = true;
    } else {
      openInitiation();
    }
  }

  async function refreshServerSnapshot(): Promise<void> {
    if (!clientReady) return;
    try {
      servers = await screenApi.get<Schema['ServerDTO'][]>('/v1/servers');
      status = await screenApi.get<Schema['RemoteAPIStatus']>('/v1/status');
      observeServerRun(hostId, status.activeServerId ?? '', status.running);
    } catch {
      // The sheet already presents the operation's result; a later shell poll
      // can reconcile the list if the host drops during the final stop.
    }
  }

  function closeInitiation(): void {
    initiationVisible = false;
    if (initiationComplete) {
      initiationServer = undefined;
      initiationComplete = false;
    }
    void refreshServerSnapshot();
  }

  async function initializeClient(
    generation: number = hostOrchestrator.nextGeneration(),
  ): Promise<void> {
    if (generation !== hostOrchestrator.currentGeneration) return;
    const selectedHostId = hostId;
    cancelTabPreload?.();
    cancelTabPreload = undefined;
    clientReady = false;
    client = undefined;
    capabilities = null;
    permissions = [];
    servers = [];
    selectedServerId = '';
    status = defaultStatus;
    forgetHostRuns(hostId);
    agentReadiness = 'starting';
    hostStore.updateConnection(selectedHostId, 'connecting');
    try {
      // Only the local host has an OS service this client can prepare --
      // a remote host's agent is either already reachable or it isn't;
      // there is nothing here to install/start on someone else's machine.
      const serviceStatus = selectedHostId === localAgentHostId ? await prepareLocalAgent() : null;
      if (generation !== hostOrchestrator.currentGeneration) return;
      if (serviceStatus) {
        agentReadiness = readinessForService(serviceStatus);
        if (serviceStatus.state !== 'running') {
          shellMessage = serviceStatus.detail;
          await selectSection('agent-setup', true, generation);
          return;
        }
      }
      if (isDesktopShell && selectedHostId !== localAgentHostId) {
        const host = hostStore.getState(selectedHostId).host;
        if (
          host.ssh.authentication === 'password' &&
          !hostConnectionManager.hasSessionPassword(host.id) &&
          !host.manualAgentAddress
        ) {
          promptForSshPassword(host.id);
          return;
        }
        const connection = await hostConnectionManager.connect(host);
        if (generation !== hostOrchestrator.currentGeneration) return;
        shellMessage = connection.detail;
      }
      const selectedClient = await createClient(selectedHostId);
      if (generation !== hostOrchestrator.currentGeneration) return;
      const ready = await restoreHostContext(generation, selectedHostId, selectedClient);
      if (generation !== hostOrchestrator.currentGeneration) return;
      clientReady = ready;
      if (clientReady) scheduleAvailableTabPreload();
    } catch (error) {
      if (generation !== hostOrchestrator.currentGeneration) return;
      const host = hostStore.getState(selectedHostId).host;
      const connectionError = formatConnectionFailure(error);
      if (
        isDesktopShell &&
        selectedHostId !== localAgentHostId &&
        host.ssh.authentication === 'password' &&
        (connectionError.includes('Password authentication needs a password') ||
          connectionError.includes('SSH password or key was refused'))
      ) {
        hostConnectionManager.forgetSessionPassword(selectedHostId);
        promptForSshPassword(
          selectedHostId,
          connectionError.includes('refused') ? 'That password was not accepted. Try again.' : '',
        );
        hostStore.updateConnection(selectedHostId, 'connecting');
        return;
      }
      agentReadiness = readinessForError(error);
      shellMessage = `Unable to prepare the selected host connection: ${connectionError}`;
      hostStore.updateConnection(selectedHostId, 'error');
      await selectSection('agent-setup', true, generation);
    }
  }

  function scheduleAvailableTabPreload(): void {
    cancelTabPreload?.();
    cancelTabPreload = undefined;
    if (!preloadTabs) return;

    const context = currentNavigationContext();
    if (!context) return;
    const visibleIds = new Set(router.visibleSections(context).map((section) => section.id));
    const tabSections = PRIMARY_TABS.map((tab) => router.get(tab.id)).filter(
      (section): section is SectionDescriptor => !!section && visibleIds.has(section.id),
    );
    cancelTabPreload = scheduleTabPreload(tabSections);
  }

  function setPreloadTabs(enabled: boolean): void {
    preloadTabs = enabled;
    setTabPreloadPreference(enabled);
    if (enabled && clientReady) scheduleAvailableTabPreload();
    else if (!enabled) {
      cancelTabPreload?.();
      cancelTabPreload = undefined;
    }
  }

  onMount(() => {
    restoreAccent();
    void initializeShell();
    const onPopState = () => void selectFromLocation();
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  });

  onDestroy(() => {
    if (broadcastAuthTimer !== undefined) clearInterval(broadcastAuthTimer);
  });

  async function initializeShell(): Promise<void> {
    const platform = await getPlatform();
    isDesktopShell = platform.kind === 'tauri';
    hostStore.addHost({
      ...createLocalHostRecord(LOCAL_AGENT_ORIGIN),
    });
    if (isDesktopShell) {
      for (const host of loadSavedRemoteHosts()) hostStore.addHost(host);
    }
    refreshHosts();
    await initializeClient();
  }

  async function selectSection(
    id: string,
    updateUrl = true,
    generation = hostOrchestrator.currentGeneration,
  ): Promise<void> {
    if (generation !== hostOrchestrator.currentGeneration) return;
    const request = ++sectionRequest;
    const section = router.get(id);
    const context = currentNavigationContext();
    // Setup is deliberately reachable before an agent exists, so service
    // installation can be the first meaningful action.
    if (!section || (!context && section.id !== 'agent-setup')) {
      shellMessage = 'That section is unavailable for the selected host or credential.';
      return;
    }
    if (!loadedSections.some((loaded) => loaded.id === section.id)) {
      const component = (await section.load()).default;
      if (generation !== hostOrchestrator.currentGeneration || request !== sectionRequest) return;
      loadedSections = [...loadedSections, { id: section.id, component }];
    }
    if (generation !== hostOrchestrator.currentGeneration || request !== sectionRequest) return;
    activeSection = section.id;
    if (updateUrl) {
      history.pushState({}, '', buildSectionPath(section, hostId, selectedServerId));
    }
  }

  async function selectFromLocation(
    generation = hostOrchestrator.currentGeneration,
  ): Promise<void> {
    if (generation !== hostOrchestrator.currentGeneration) return;
    const context = currentNavigationContext();
    if (!context) return;
    if (window.location.pathname === '/') {
      await selectSection('agent-setup', true, generation);
      return;
    }
    const resolution = router.resolve(window.location.pathname, context);
    if (resolution.kind !== 'section' || resolution.match.hostId !== hostId) {
      activeSection = '';
      loadedSections = [{ id: 'unknown', component: UnknownSection }];
      shellMessage = 'This link is unavailable for the currently selected host.';
      return;
    }
    if (resolution.match.serverId && resolution.match.serverId !== selectedServerId) {
      loadedSections = [];
      selectedServerId = resolution.match.serverId;
    }
    await selectSection(resolution.descriptor.id, false, generation);
  }

  function openAgentSetup(): void {
    void selectSection('agent-setup');
  }
</script>

<svelte:head>
  <meta name="description" content="Minecraft Server Controller" />
</svelte:head>

<ApplicationShell
  hostLabel={hosts.find((host) => host.id === hostId)?.displayName ?? 'Local agent'}
  {hosts}
  activeHostId={hostId}
  {isDesktopShell}
  api={screenApi}
  {servers}
  activeServerId={selectedServerId}
  running={status.running}
  connected={!!capabilities}
  {capabilities}
  {canControl}
  {bannerColor}
  tabs={primaryTabs}
  {activeSection}
  selectSection={(id) => void selectSection(id)}
  onSelectServer={(id) => void selectServer(id)}
  onSwitchHost={(id) => void switchHost(id)}
  onLifecycle={(action) => void lifecycle(action)}
  onInitiate={openInitiation}
  initiationHidden={Boolean(
    initiationServer && !initiationVisible && initiationServer.id === selectedServerId,
  )}
  initiationServerId={initiationServer?.id}
  onResumeInitiation={resumeInitiation}
  onOpenAgentSetup={openAgentSetup}
  onManage={() => (manageOpen = true)}
  {addressesVisible}
  onToggleAddresses={toggleAddresses}
  onHelp={() => void selectSection('handbook')}
  onSettings={() => (settingsOpen = true)}
  onRefresh={() => void initializeClient()}
  onEditServer={activeServer ? () => (headerEditingServer = activeServer) : undefined}
>
  {#if loadedSections.length}
    {#each loadedSections as loaded (loaded.id)}
      <div
        class="tab-pane"
        class:tab-pane-active={loaded.id === activeSection}
        aria-hidden={loaded.id !== activeSection}
      >
        <svelte:component
          this={loaded.component}
          api={screenApi}
          {hostId}
          {hosts}
          activeHostId={hostId}
          hostSummaries={currentHostSummaries}
          hostLabel={hosts.find((host) => host.id === hostId)?.displayName ?? 'Local agent'}
          hostBaseUrl={hostManagementUrl(hostStore.getState(hostId).host)}
          {isDesktopShell}
          isLocalHost={hostId === localAgentHostId}
          serverId={selectedServerId}
          active={loaded.id === activeSection}
          {permissions}
          readiness={agentReadiness}
          onAgentRetry={() => initializeClient()}
          {servers}
          serverRunning={status.running}
          onOpenServer={async (id: string) => {
            if (id !== selectedServerId) await selectServer(id);
            if (id === selectedServerId) await selectSection('home');
          }}
          onPairAgain={(code: string) => pairAgain(code)}
          onConnectHost={(input: RemoteHostConnectionInput) => connectRemoteHost(input)}
          onSelectRoute={(id: HostId, route: HostRoute) => selectHostRoute(id, route)}
          onRemoveHost={isDesktopShell && hostId !== localAgentHostId
            ? removeCurrentRemoteHost
            : undefined}
          onSwitchHost={(id: string) => switchHost(id)}
          onRemoveSavedHost={(id: string) => removeRemoteHost(id)}
          onServerSelected={(id: string) => {
            if (id !== selectedServerId) loadedSections = [];
            selectedServerId = id;
          }}
          onFleet={() => (manageOpen = true)}
          onWorlds={() => void selectSection('worlds')}
          addressesVisible={activeSection === 'home' ? addressesVisible : false}
          onToggleAddresses={toggleAddresses}
          {healthRefreshVersion}
        />
      </div>
    {/each}
  {:else}
    <div class="dashboard" data-bundle-id={bundleIdentity.id} data-client-surface="shared">
      <p class="eyebrow">Minecraft Server Controller</p>
      <p class="intro-copy" role="status">{shellMessage}</p>
    </div>
  {/if}
</ApplicationShell>

{#if broadcastAuth?.isPresent}
  <BroadcastAuthSheet api={screenApi} prompt={broadcastAuth} onClose={closeBroadcastAuthPrompt} />
{/if}

{#if initiationServer}
  <FirstStartSheet
    api={screenApi}
    serverName={initiationServer.name}
    serverType={initiationServer.serverType === 'bedrock' ? 'bedrock' : 'java'}
    localPort={initiationServer.gamePort ??
      (initiationServer.serverType === 'bedrock' ? 19132 : 25565)}
    hostAddress={initiationServer.hostAddress}
    localBedrockPort={initiationServer.serverType !== 'bedrock'
      ? initiationServer.bedrockPort
      : undefined}
    playitEnabled={initiationServer.playitEnabled ?? false}
    broadcastEnabled={initiationServer.xboxBroadcastEnabled ?? false}
    onClose={closeInitiation}
    hidden={!initiationVisible}
    onComplete={() => {
      initiationComplete = true;
      void refreshServerSnapshot();
    }}
  />
{/if}

{#if startupFailure}
  <StartupFailureSheet
    api={screenApi}
    serverName={startupFailure.serverName}
    operationKind="start"
    serverRunning={status.running}
    errorCode={startupFailure.errorCode}
    failureMessage={startupFailure.message}
    problems={startupFailure.problems ?? []}
    visible
    onClose={() => (startupFailure = undefined)}
    onRetry={retryStartup}
  />
{/if}

{#if manageOpen}
  <ManageSheet
    api={screenApi}
    {servers}
    {status}
    {permissions}
    {hosts}
    hostSummaries={currentHostSummaries}
    activeHostId={hostId}
    {isDesktopShell}
    onClose={() => (manageOpen = false)}
    onSwitchHost={(id) => void switchHost(id)}
    onRemoveHost={(id) => void removeRemoteHost(id)}
    onServersChanged={(updated) => (servers = updated)}
    onActivated={(id) => {
      if (id !== selectedServerId) {
        loadedSections = [];
        selectedServerId = id;
        void selectSection(activeSection, false);
      }
      status = { ...status, activeServerId: id };
    }}
  />
{/if}

{#if headerEditingServer}
  <ServerEditorSheet
    api={screenApi}
    server={headerEditingServer}
    {canControl}
    onClose={() => (headerEditingServer = undefined)}
    onServersChanged={refreshServerSnapshot}
    onSetActive={selectServer}
  />
{/if}

{#if sshPasswordPromptHostId}
  {@const sshPasswordPromptHost = hosts.find((host) => host.id === sshPasswordPromptHostId)}
  <Sheet
    title={`Connect to ${sshPasswordPromptHost?.displayName ?? 'saved host'}`}
    size="sm"
    onClose={sshPasswordPromptBusy ? undefined : closeSshPasswordPrompt}
  >
    <form class="ssh-password-prompt" onsubmit={submitSshPassword}>
      <p>
        Enter the SSH password for {sshPasswordPromptHost?.displayName ?? 'this computer'}. MSC
        keeps it in memory only until you quit the app.
      </p>
      <label class="ssh-password-label">
        SSH password
        <Field
          type="password"
          bind:value={sshPasswordInput}
          placeholder="Password for this computer"
          disabled={sshPasswordPromptBusy}
        />
      </label>
      {#if sshPasswordPromptError}
        <p class="ssh-password-error" role="alert">{sshPasswordPromptError}</p>
      {/if}
      <div class="ssh-password-actions">
        <Button
          variant="secondary"
          disabled={sshPasswordPromptBusy}
          onclick={closeSshPasswordPrompt}>Cancel</Button
        >
        <Button variant="primary" type="submit" disabled={sshPasswordPromptBusy}>
          {sshPasswordPromptBusy ? 'Connecting…' : 'Connect'}
        </Button>
      </div>
    </form>
  </Sheet>
{/if}

{#if settingsOpen}
  <AppSettingsSheet
    api={screenApi}
    {hostId}
    activeServerXboxBroadcastEnabled={activeServer?.xboxBroadcastEnabled === true}
    serverUsesPlayit={servers.find((server) => server.id === selectedServerId)?.playitEnabled}
    onClose={() => (settingsOpen = false)}
    {preloadTabs}
    onPreloadTabsChanged={setPreloadTabs}
    onOpenReset={openReset}
    onOpenUninstall={() => {
      settingsOpen = false;
      uninstallOpen = true;
    }}
  />
{/if}

{#if uninstallOpen}
  <UninstallSheet onClose={() => (uninstallOpen = false)} />
{/if}

{#if resetOpen}
  <ResetSheet
    api={screenApi}
    hostLabel={hostLabelForCurrentHost()}
    {permissions}
    {isDesktopShell}
    isLocalHost={isLocalHostForReset()}
    onClose={() => (resetOpen = false)}
    onClientReset={resetClientState}
    onHostResetComplete={completeHostReset}
  />
{/if}

<SplashGate onComplete={completeSplash} />
{#if splashComplete && clientReady}
  <FirstLaunchGate api={screenApi} agentReady={agentReadiness === 'ready'} />
{/if}

<style>
  .ssh-password-prompt {
    display: grid;
    gap: 14px;
  }
  .ssh-password-prompt p {
    margin: 0;
    color: var(--msc2-text-secondary);
    line-height: 1.5;
  }
  .ssh-password-label {
    display: grid;
    gap: 6px;
    color: var(--msc2-text-primary);
    font-size: 13px;
  }
  .ssh-password-prompt .ssh-password-error {
    color: var(--msc2-status-error);
  }
  .ssh-password-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .dashboard {
    display: grid;
    gap: 1rem;
  }
  .eyebrow {
    margin: 0;
    color: var(--msc-accent);
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .intro-copy {
    color: var(--msc-muted);
    line-height: 1.6;
  }
  .tab-pane {
    display: none;
  }
  .tab-pane-active {
    display: contents;
  }
</style>
