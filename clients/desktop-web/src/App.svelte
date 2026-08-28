<script lang="ts">
  import { onMount } from 'svelte';
  import { bundleIdentity } from './lib/bundle-identity';
  import { ApiClient, ApiError } from './lib/api/client';
  import ApplicationShell from './lib/components/ApplicationShell.svelte';
  import FirstLaunchGate from './lib/help/FirstLaunchGate.svelte';
  import SplashGate from './lib/help/SplashGate.svelte';
  import { createClientRouter } from './routes/router';
  import UnknownSection from './routes/UnknownSection.svelte';
  import { buildSectionPath } from './lib/navigation/route';
  import {
    AgentHealthTimeoutError,
    createAgentTransport,
    getPlatform,
    LOCAL_AGENT_ORIGIN,
    openLocalAgentBrowser,
    prepareLocalAgent,
    type AgentReadiness,
    type AgentServiceStatus,
  } from './lib/platform';
  import { redeemBrowserHandoff } from './lib/auth/browser-handoff';
  import { DesktopSessionAuth, loadTauriDesktopCredentialBridge } from './lib/auth/desktop';
  import { clearClientPreferences, HostStore } from './lib/hosts/registry';
  import type { HostId, HostRecord } from './lib/hosts/types';
  import ManageSheet from './lib/sections/fleet/ManageSheet.svelte';
  import AppSettingsSheet from './lib/sections/app-settings/AppSettingsSheet.svelte';
  import ResetSheet from './lib/sections/app-settings/ResetSheet.svelte';
  import { restoreAccent } from './lib/styles/accent';
  import { bannerColorFor } from './lib/styles/bannerColor';
  import { PRIMARY_TABS } from './lib/navigation/primaryTabs';
  import { selectAvailableServerId } from './lib/navigation/serverSelection';
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
      label: 'Local agent',
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
  const localAgentHostId = 'local-agent';

  // Keeps every host's connection/credential/cache state (D-013). A browser
  // tab only ever has this one Local entry -- createAgentTransport's browser
  // branch always targets window.location.origin, with no per-host baseUrl,
  // so a second host is unreachable from a browser regardless of what's
  // registered here. Add Host / host switching UI is gated on isDesktopShell.
  const hostStore = new HostStore();
  let hosts: readonly HostRecord[] = [];
  let hostId = localAgentHostId;
  let isDesktopShell = false;
  let manageOpen = false;
  let settingsOpen = false;
  let resetOpen = false;
  let browserHandoffError = '';

  function refreshHosts(): void {
    hosts = hostStore.listHosts();
  }

  function hostSummaries(): Map<HostId, { connection: string; serverCount: number }> {
    const summaries = new Map<HostId, { connection: string; serverCount: number }>();
    for (const host of hosts) {
      const cache = hostStore.getState(host.id).cache;
      summaries.set(host.id, { connection: cache.connection, serverCount: cache.servers.length });
    }
    return summaries;
  }

  async function switchHost(id: HostId): Promise<void> {
    if (id === hostId) return;
    hostStore.selectHost(id);
    hostId = id;
    await initializeClient();
  }

  async function addRemoteHost(
    label: string,
    baseUrl: string,
    pairingCode: string,
  ): Promise<string> {
    const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
    const result = await auth.redeemRemotePairing(baseUrl, pairingCode);
    hostStore.addHost({ id: result.agentHostId, label, baseUrl });
    refreshHosts();
    return result.agentHostId;
  }

  async function pairAgain(pairingCode: string): Promise<void> {
    if (!isDesktopShell || hostId === localAgentHostId) {
      throw new Error('Fresh pairing is available only for a remote desktop host.');
    }
    const previousHost = hosts.find((host) => host.id === hostId);
    if (!previousHost) throw new Error('The selected host is no longer registered.');

    const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
    // The reset already revoked this credential on the host. Forget it here as
    // well so a failed or interrupted recovery cannot leave stale local state.
    await auth.forgetCredentials([previousHost.id], false);
    const result = await auth.redeemRemotePairing(previousHost.baseUrl, pairingCode);

    hostStore.removeHost(previousHost.id);
    hostStore.addHost({
      id: result.agentHostId,
      label: previousHost.label,
      baseUrl: previousHost.baseUrl,
    });
    hostStore.selectHost(result.agentHostId);
    hostId = result.agentHostId;
    refreshHosts();
    await initializeClient();
  }

  function removeRemoteHost(id: HostId): void {
    if (id === localAgentHostId) return;
    if (id === hostId) void switchHost(localAgentHostId);
    hostStore.removeHost(id);
    refreshHosts();
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

  function requireClient(): ApiClient {
    if (!client) throw new Error('The selected host client is still initializing.');
    return client;
  }

  function createScreenApi(): ScreenApi {
    return {
      get: <T,>(path: string) => requireClient().requestJson<T>('GET', path),
      post: <T,>(path: string, body?: unknown) =>
        requireClient().requestJson<T>('POST', path, { body }),
      upload: (purpose, bytes, options) =>
        requireClient().stagedUpload({ purpose, ...options }, bytes),
      download: (id) => requireClient().downloadBytes(id),
    };
  }

  const screenApi: ScreenApi = createScreenApi();

  let activeSection = '';
  let activeComponent: any;
  let selectedServerId = '';
  let permissions: readonly string[] = [];
  let capabilities: Capabilities | null = null;
  let shellMessage = 'Connecting to the selected host…';
  let servers: readonly Schema['ServerDTO'][] = [];
  let status: Schema['RemoteAPIStatus'] = defaultStatus;

  type HostResetResult = {
    operationId: string;
    hostId: string;
    mode: 'configuration' | 'everything';
    agentState: 'restarting' | 'needs_pairing' | 'unavailable';
    message: string;
  };

  $: currentAgentHostId = client?.host ?? hostId;

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

  async function completeHostReset(result: HostResetResult): Promise<void> {
    let cleanupError = '';
    if (isDesktopShell) {
      try {
        const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
        await auth.forgetCredentials([result.hostId], hostId === localAgentHostId);
      } catch (error) {
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

    if (isLocalHostForReset() && isDesktopShell && result.mode === 'everything') {
      try {
        await (await getPlatform()).manageAgentService('uninstall');
        agentReadiness = 'missing';
        shellMessage = cleanupError || 'The local host was reset. Install the agent to continue.';
      } catch (error) {
        agentReadiness = 'unavailable';
        shellMessage = `The host was reset, but the local agent service could not be removed: ${String(error)}`;
      }
      hostStore.updateConnection(hostId, 'error');
      await selectSection('agent-setup');
      return;
    }

    hostStore.updateConnection(hostId, 'error');
    if (isLocalHostForReset() && isDesktopShell) {
      agentReadiness = 'starting';
      shellMessage = cleanupError || 'The local host was reset. Reconnecting with its new identity…';
      await selectSection('agent-setup');
      void initializeClient();
    } else {
      agentReadiness = 'unavailable';
      shellMessage = cleanupError || `Host reset complete. Pair ${hostLabelForCurrentHost()} again.`;
      await selectSection('agent-setup');
    }
  }

  function isLocalHostForReset(): boolean {
    return hostId === localAgentHostId;
  }

  function hostLabelForCurrentHost(): string {
    return hosts.find((host) => host.id === hostId)?.label ?? hostId;
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
  // `bannerColorAccentVersion` has no meaning of its own -- it's a parameter
  // here only so AppSettingsSheet's onAccentColorSaved can force a re-read of
  // localStorage, which changing hostId/selectedServerId alone wouldn't catch.
  let bannerColorAccentVersion = 0;
  function readBannerColor(host: string, server: string, _accentVersion: number): string {
    return bannerColorFor(host, server);
  }
  $: bannerColor = readBannerColor(hostId, selectedServerId, bannerColorAccentVersion);
  // Referencing servers/status/hostId directly (not just through hostSummaries'
  // internals) makes Svelte re-run this when the active host's live state
  // changes, not only when a host is added or removed.
  $: currentHostSummaries = ((): Map<HostId, { connection: string; serverCount: number }> => {
    void servers;
    void status;
    void hostId;
    return hosts.length ? hostSummaries() : new Map();
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

  async function restoreHostContext(): Promise<boolean> {
    try {
      const selectedClient = requireClient();
      capabilities = await selectedClient.getCapabilities();
      const me = await selectedClient.requestJson<{ permissions: string[] }>('GET', '/v1/me');
      permissions = me.permissions;
      servers = await selectedClient.requestJson<Schema['ServerDTO'][]>('GET', '/v1/servers');
      status = await selectedClient.requestJson<Schema['RemoteAPIStatus']>('GET', '/v1/status');
      selectedServerId = selectAvailableServerId(servers, status.activeServerId, selectedServerId);
      agentReadiness = 'ready';
      shellMessage = `Connected to ${hosts.find((host) => host.id === hostId)?.label ?? hostId}`;
      hostStore.setServers(hostId, servers);
      hostStore.updateConnection(hostId, 'connected');
      await selectFromLocation();
      return true;
    } catch (error) {
      capabilities = null;
      permissions = [];
      agentReadiness = readinessForError(error);
      shellMessage = `Unable to establish the selected host context: ${String(error)}`;
      hostStore.updateConnection(hostId, 'error');
      await selectSection('agent-setup');
      return false;
    }
  }

  async function selectServer(id: string): Promise<void> {
    try {
      await screenApi.post('/v1/active-server', { serverId: id });
      selectedServerId = id;
      status = { ...status, activeServerId: id };
      const section = router.get(activeSection);
      if (section) history.pushState({}, '', buildSectionPath(section, hostId, selectedServerId));
    } catch (error) {
      shellMessage = `Unable to switch servers: ${String(error)}`;
    }
  }

  async function lifecycle(action: 'start' | 'stop'): Promise<void> {
    try {
      await screenApi.post(action === 'start' ? '/v1/start' : '/v1/stop');
      status = await screenApi.get<Schema['RemoteAPIStatus']>('/v1/status');
    } catch (error) {
      shellMessage = `Unable to ${action} the server: ${String(error)}`;
    }
  }

  async function initializeClient(): Promise<void> {
    clientReady = false;
    capabilities = null;
    permissions = [];
    agentReadiness = 'starting';
    hostStore.updateConnection(hostId, 'connecting');
    try {
      // Only the local host has an OS service this client can prepare --
      // a remote host's agent is either already reachable or it isn't;
      // there is nothing here to install/start on someone else's machine.
      const serviceStatus = hostId === localAgentHostId ? await prepareLocalAgent() : null;
      if (serviceStatus) {
        agentReadiness = readinessForService(serviceStatus);
        if (serviceStatus.state !== 'running') {
          shellMessage = serviceStatus.detail;
          await selectSection('agent-setup');
          return;
        }
      }
      client = await createClient(hostId);
      clientReady = await restoreHostContext();
    } catch (error) {
      agentReadiness = readinessForError(error);
      shellMessage = `Unable to prepare the local agent connection: ${String(error)}`;
      hostStore.updateConnection(hostId, 'error');
      await selectSection('agent-setup');
    }
  }

  onMount(() => {
    restoreAccent();
    void initializeShell();
    const onPopState = () => void selectFromLocation();
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  });

  async function initializeShell(): Promise<void> {
    const platform = await getPlatform();
    isDesktopShell = platform.kind === 'tauri';
    if (!isDesktopShell) {
      await redeemBrowserHandoff(window.location, window.history);
    }
    // A browser only has one host: the agent that served this page. Keeping its
    // actual origin here prevents a remote page from ever being mistaken for a
    // loopback agent on the browser user's own computer.
    hostStore.addHost({
      id: localAgentHostId,
      label: 'Local agent',
      baseUrl: isDesktopShell ? LOCAL_AGENT_ORIGIN : window.location.origin,
    });
    refreshHosts();
    await initializeClient();
  }

  async function selectSection(id: string, updateUrl = true): Promise<void> {
    const section = router.get(id);
    const context = currentNavigationContext();
    // Setup is deliberately reachable before an agent exists or a browser has
    // paired: its truthful fallback is how this host becomes manageable.
    if (!section || (!context && section.id !== 'agent-setup')) {
      shellMessage = 'That section is unavailable for the selected host or credential.';
      return;
    }
    activeSection = section.id;
    activeComponent = (await section.load()).default;
    if (updateUrl) {
      history.pushState({}, '', buildSectionPath(section, hostId, selectedServerId));
    }
  }

  async function selectFromLocation(): Promise<void> {
    const context = currentNavigationContext();
    if (!context) return;
    if (window.location.pathname === '/') {
      await selectSection('home');
      return;
    }
    const resolution = router.resolve(window.location.pathname, context);
    if (resolution.kind !== 'section' || resolution.match.hostId !== hostId) {
      activeSection = '';
      activeComponent = UnknownSection;
      shellMessage = 'This link is unavailable for the currently selected host.';
      return;
    }
    if (resolution.match.serverId) selectedServerId = resolution.match.serverId;
    await selectSection(resolution.descriptor.id, false);
  }

  function openAgentSetup(): void {
    void selectSection('agent-setup');
  }

  async function openLocalAgentInBrowser(): Promise<void> {
    browserHandoffError = '';
    try {
      await openLocalAgentBrowser();
    } catch (error) {
      browserHandoffError = `Could not open the local agent in a browser: ${String(error)}`;
      await selectSection('agent-setup');
    }
  }
</script>

<svelte:head>
  <meta name="description" content="Minecraft Server Controller" />
</svelte:head>

<ApplicationShell
  hostLabel={hosts.find((host) => host.id === hostId)?.label ?? 'Local agent'}
  {hosts}
  activeHostId={hostId}
  {isDesktopShell}
  api={screenApi}
  {servers}
  activeServerId={selectedServerId}
  running={status.running}
  connected={!!capabilities}
  {canControl}
  {bannerColor}
  tabs={primaryTabs}
  {activeSection}
  selectSection={(id) => void selectSection(id)}
  onSelectServer={(id) => void selectServer(id)}
  onSwitchHost={(id) => void switchHost(id)}
  onLifecycle={(action) => void lifecycle(action)}
  onOpenAgentSetup={openAgentSetup}
  onOpenBrowser={isDesktopShell ? () => void openLocalAgentInBrowser() : undefined}
  onManage={() => (manageOpen = true)}
  onHelp={() => void selectSection('handbook')}
  onSettings={() => (settingsOpen = true)}
  onRefresh={() => void initializeClient()}
>
  {#if activeComponent}
    <svelte:component
      this={activeComponent}
      api={screenApi}
      {hostId}
      hostLabel={hosts.find((host) => host.id === hostId)?.label ?? 'Local agent'}
      hostBaseUrl={hostStore.getState(hostId).host.baseUrl}
      {isDesktopShell}
      isLocalHost={hostId === localAgentHostId}
      serverId={selectedServerId}
      {permissions}
      readiness={agentReadiness}
      {browserHandoffError}
      onAgentRetry={() => void initializeClient()}
      onPairAgain={(code: string) => pairAgain(code)}
      onServerSelected={(id: string) => (selectedServerId = id)}
      onFleet={() => (manageOpen = true)}
      onWorlds={() => void selectSection('worlds')}
    />
  {:else}
    <div class="dashboard" data-bundle-id={bundleIdentity.id} data-client-surface="shared">
      <p class="eyebrow">Minecraft Server Controller</p>
      <p class="intro-copy" role="status">{shellMessage}</p>
    </div>
  {/if}
</ApplicationShell>

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
    onAddHost={(label, baseUrl, code) => addRemoteHost(label, baseUrl, code)}
    onRemoveHost={(id) => removeRemoteHost(id)}
    onServersChanged={(updated) => (servers = updated)}
    onActivated={(id) => {
      selectedServerId = id;
      status = { ...status, activeServerId: id };
    }}
  />
{/if}

{#if settingsOpen}
  <AppSettingsSheet
    api={screenApi}
    {hostId}
    serverId={selectedServerId || undefined}
    serverLabel={servers.find((server) => server.id === selectedServerId)?.name}
    onClose={() => (settingsOpen = false)}
    onAccentColorSaved={() => (bannerColorAccentVersion += 1)}
    onOpenReset={openReset}
    canResetHost={permissions.includes('admin')}
  />
{/if}

{#if resetOpen}
  <ResetSheet
    api={screenApi}
    agentHostId={currentAgentHostId}
    hostLabel={hostLabelForCurrentHost()}
    {permissions}
    {isDesktopShell}
    isLocalHost={isLocalHostForReset()}
    onClose={() => (resetOpen = false)}
    onClientReset={resetClientState}
    onHostResetComplete={completeHostReset}
  />
{/if}

<SplashGate />
{#if clientReady}<FirstLaunchGate api={screenApi} agentReady={agentReadiness === 'ready'} />{/if}

<style>
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
</style>
