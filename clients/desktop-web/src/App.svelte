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
  const hostIds = ['local-agent', 'demo-agent'] as const;
  let hostId: (typeof hostIds)[number] = 'local-agent';

  function createClient(id: string): ApiClient {
    return new ApiClient({
      baseUrl: typeof window === 'undefined' ? 'http://127.0.0.1' : window.location.origin,
      hostId: id,
    });
  }
  function createScreenApi(): ScreenApi {
    return {
      get: <T,>(path: string) => client.requestJson<T>('GET', path),
      post: <T,>(path: string, body?: unknown) => client.requestJson<T>('POST', path, { body }),
      upload: (purpose, bytes) => client.stagedUpload({ purpose }, bytes),
      download: (id) => client.downloadBytes(id),
    };
  }

  let client = createClient(hostId);
  let screenApi: ScreenApi = createScreenApi();

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
      capabilities = await client.getCapabilities();
      const me = await client.requestJson<{ permissions: string[] }>('GET', '/v1/me');
      permissions = me.permissions;
      shellMessage = `Connected to ${hostId === 'local-agent' ? 'Local agent' : 'Demo agent'}`;
      await selectFromLocation();
    } catch (error) {
      shellMessage = `Unable to establish the selected host context: ${String(error)}`;
    }
  }

  async function switchHost(): Promise<void> {
    const nextIndex = (hostIds.indexOf(hostId) + 1) % hostIds.length;
    hostId = hostIds[nextIndex];
    selectedServerId = 'survival';
    permissions = [];
    capabilities = null;
    client = createClient(hostId);
    screenApi = createScreenApi();
    history.replaceState({}, '', '/');
    await restoreHostContext();
  }

  onMount(() => {
    const onPopState = () => void selectFromLocation();
    void restoreHostContext();
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  });

  async function selectSection(id: string, updateUrl = true): Promise<void> {
    const section = router.get(id);
    if (!section || !navigationContext) {
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
  hostLabel={hostId === 'local-agent' ? 'Local agent' : 'Demo agent'}
  serverLabel={selectedServerId === 'survival' ? 'Survival' : selectedServerId}
  connectionLabel={capabilities ? 'Connected' : 'Disconnected'}
  sections={visibleSections}
  {activeSection}
  selectSection={(id) => void selectSection(id)}
  switchHost={() => void switchHost()}
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
<FirstLaunchGate api={screenApi} />

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
