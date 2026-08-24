<script lang="ts">
  import { onMount } from 'svelte';
  import { bundleIdentity } from './lib/bundle-identity';
  import { ApiClient } from './lib/api/client';
  import ActionButton from './lib/components/ActionButton.svelte';
  import ApplicationShell from './lib/components/ApplicationShell.svelte';
  import type { SectionDescriptor } from './lib/navigation/types';
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

  const client = new ApiClient({
    baseUrl: typeof window === 'undefined' ? 'http://127.0.0.1' : window.location.origin,
    hostId: 'local-agent',
  });
  const screenApi: ScreenApi = {
    get: <T,>(path: string) => client.requestJson<T>('GET', path),
    post: <T,>(path: string, body?: unknown) => client.requestJson<T>('POST', path, { body }),
    upload: (purpose, bytes) => client.stagedUpload({ purpose }, bytes),
    download: (id) => client.downloadBytes(id),
  };

  let activeSection = 'home';
  // Dynamic section modules come from the registry; their shared props are
  // intentionally supplied by the shell rather than an exhaustive switch.
  let activeComponent: any;
  let selectedServerId = 'survival';
  let shellMessage = 'Ready for an agent connection';

  onMount(() => {
    void selectSection(activeSection);
  });

  async function selectSection(id: string): Promise<void> {
    const section = sections.find((candidate) => candidate.id === id) ?? sections[0];
    activeSection = section.id;
    activeComponent = (await section.load()).default;
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
  connectionLabel="Disconnected"
  {sections}
  {activeSection}
  onSection={(id) => void selectSection(id)}
  onHostSwitcher={() => (shellMessage = 'Host switching is ready for the injected registry')}
  onConsole={() => (shellMessage = 'Console is always available for the selected server')}
>
  {#if activeComponent}
    <svelte:component
      this={activeComponent}
      api={screenApi}
      serverId={selectedServerId}
      onServerSelected={(id: string) => (selectedServerId = id)}
    />
  {:else}
    <div class="dashboard" data-bundle-id={bundleIdentity.id} data-client-surface="shared">
      <div class="intro-row">
        <p class="eyebrow">Minecraft Server Controller</p>
        <ActionButton label="Acknowledge shell" onclick={acknowledgeShell}
          >Acknowledge shell</ActionButton
        >
      </div>
      <p class="intro-copy">{shellMessage}</p>
    </div>
  {/if}
</ApplicationShell>

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
