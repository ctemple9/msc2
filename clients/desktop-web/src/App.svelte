<script lang="ts">
  import { onMount } from 'svelte';
  import { bundleIdentity } from './lib/bundle-identity';
  import { ApiClient } from './lib/api/client';
  import ActionButton from './lib/components/ActionButton.svelte';
  import ApplicationShell from './lib/components/ApplicationShell.svelte';
  import FirstLaunchGate from './lib/help/FirstLaunchGate.svelte';
  import SplashGate from './lib/help/SplashGate.svelte';
  import { createClientRouter } from './routes/router';
  import UnknownSection from './routes/UnknownSection.svelte';
  import { buildSectionPath } from './lib/navigation/route';
  import { createAgentTransport } from './lib/platform';
  import { restoreAccent } from './lib/styles/accent';
  import type { Capabilities, NavigationContext, SectionDescriptor } from './lib/navigation/types';
  import type { ScreenApi } from './lib/sections/shared/types';
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
      id: 'fleet',
      label: 'Fleet',
      segment: 'fleet',
      scope: 'server',
      requiredPermissions: ['fleet'],
      load: () => import('./lib/sections/fleet/FleetSection.svelte'),
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
  let hostId = localAgentHostId;

  async function createClient(id: string): Promise<ApiClient> {
    const transport = await createAgentTransport(id);
    return new ApiClient({ ...transport, hostId: id });
  }

  let client: ApiClient | undefined;
  let clientReady = false;

  function requireClient(): ApiClient {
    if (!client) throw new Error('The selected host client is still initializing.');
    return client;
  }

  function createScreenApi(): ScreenApi {
    return {
      get: <T,>(path: string) => requireClient().requestJson<T>('GET', path),
      post: <T,>(path: string, body?: unknown) =>
        requireClient().requestJson<T>('POST', path, { body }),
      upload: (purpose, bytes) => requireClient().stagedUpload({ purpose }, bytes),
      download: (id) => requireClient().downloadBytes(id),
    };
  }

  const screenApi: ScreenApi = createScreenApi();

  let activeSection = '';
  let activeComponent: any;
  let selectedServerId = 'survival';
  let permissions: readonly string[] = [];
  let capabilities: Capabilities | null = null;
  let shellMessage = 'Connecting to the selected host…';

  $: navigationContext = capabilities
    ? ({
        hostId,
        serverId: selectedServerId,
        permissions,
        capabilities,
      } satisfies NavigationContext)
    : null;
  $: visibleSections = navigationContext ? router.visibleSections(navigationContext) : [];

  async function restoreHostContext(): Promise<void> {
    try {
      const selectedClient = requireClient();
      capabilities = await selectedClient.getCapabilities();
      const me = await selectedClient.requestJson<{ permissions: string[] }>('GET', '/v1/me');
      permissions = me.permissions;
      shellMessage = 'Connected to Local agent';
      await selectFromLocation();
    } catch (error) {
      shellMessage = `Unable to establish the selected host context: ${String(error)}`;
      await selectSection('agent-setup');
    }
  }

  async function initializeClient(): Promise<void> {
    try {
      client = await createClient(hostId);
      clientReady = true;
      await restoreHostContext();
    } catch (error) {
      shellMessage = `Unable to prepare the local agent connection: ${String(error)}`;
      await selectSection('agent-setup');
    }
  }

  onMount(() => {
    restoreAccent();
    const onPopState = () => void selectFromLocation();
    void initializeClient();
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  });

  async function selectSection(id: string, updateUrl = true): Promise<void> {
    const section = router.get(id);
    // Setup is deliberately reachable before an agent exists or a browser has
    // paired: its truthful fallback is how this host becomes manageable.
    if (!section || (!navigationContext && section.id !== 'agent-setup')) {
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
    if (!navigationContext) return;
    if (window.location.pathname === '/') {
      await selectSection('home');
      return;
    }
    const resolution = router.resolve(window.location.pathname, navigationContext);
    if (resolution.kind !== 'section' || resolution.match.hostId !== hostId) {
      activeSection = '';
      activeComponent = UnknownSection;
      shellMessage = 'This link is unavailable for the currently selected host.';
      return;
    }
    if (resolution.match.serverId) selectedServerId = resolution.match.serverId;
    await selectSection(resolution.descriptor.id, false);
  }

  function acknowledgeShell(): void {
    shellMessage = 'Shared client workflows are ready for the selected host';
  }
</script>

<svelte:head>
  <meta name="description" content="Minecraft Server Controller" />
</svelte:head>

<ApplicationShell
  hostLabel="Local agent"
  serverLabel={selectedServerId === 'survival' ? 'Survival' : selectedServerId}
  connectionLabel={capabilities ? 'Connected' : 'Disconnected'}
  sections={visibleSections}
  {activeSection}
  selectSection={(id) => void selectSection(id)}
  openConsole={() => void selectSection('console')}
>
  {#if activeComponent}
    <svelte:component
      this={activeComponent}
      api={screenApi}
      {hostId}
      serverId={selectedServerId}
      {permissions}
      onServerSelected={(id: string) => (selectedServerId = id)}
      onFleet={() => void selectSection('fleet')}
    />
  {:else}
    <div class="dashboard" data-bundle-id={bundleIdentity.id} data-client-surface="shared">
      <div class="intro-row">
        <p class="eyebrow">Minecraft Server Controller</p>
        <ActionButton label="Acknowledge shell" onclick={acknowledgeShell}
          >Acknowledge shell</ActionButton
        >
      </div>
      <p class="intro-copy" role="status">{shellMessage}</p>
    </div>
  {/if}
</ApplicationShell>

<SplashGate />
{#if clientReady}<FirstLaunchGate api={screenApi} />{/if}

<style>
  .dashboard {
    display: grid;
    gap: 1rem;
  }
  .intro-row {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: center;
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
