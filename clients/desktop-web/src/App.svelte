<script lang="ts">
  import { bundleIdentity } from './lib/bundle-identity';
  import ActionButton from './lib/components/ActionButton.svelte';
  import ApplicationShell from './lib/components/ApplicationShell.svelte';
  import StatePanel from './lib/components/StatePanel.svelte';
  import SurfaceCard from './lib/components/SurfaceCard.svelte';
  import type { SectionDescriptor } from './lib/navigation/types';

  const sections: SectionDescriptor[] = [
    {
      id: 'overview',
      label: 'Overview',
      segment: 'overview',
      scope: 'server',
      load: async () => ({ default: 'overview' }),
    },
    {
      id: 'settings',
      label: 'Settings',
      segment: 'settings',
      scope: 'server',
      load: async () => ({ default: 'settings' }),
    },
  ];

  let activeSection = 'overview';
  let shellMessage = 'Ready for an agent connection';

  function acknowledgeShell(): void {
    shellMessage = 'The shared client shell is running';
  }
</script>

<svelte:head>
  <meta name="description" content="Minecraft Server Controller" />
</svelte:head>

<ApplicationShell
  hostLabel="Local agent"
  serverLabel="Survival"
  connectionLabel="Disconnected"
  {sections}
  {activeSection}
  onSection={(id) => (activeSection = id)}
  onHostSwitcher={() => (shellMessage = 'Host switching is ready for the injected registry')}
  onConsole={() => (shellMessage = 'Console is always available for the selected server')}
>
  <div class="dashboard" data-bundle-id={bundleIdentity.id} data-client-surface="shared">
    <div class="intro-row">
      <div>
        <p class="eyebrow">Minecraft Server Controller</p>
        <h2>Keep the important state in view.</h2>
        <p class="intro-copy">
          The same responsive interface will run in the desktop shell and the agent-served browser.
        </p>
      </div>
      <ActionButton label="Acknowledge shell" onclick={acknowledgeShell}
        >Acknowledge shell</ActionButton
      >
    </div>

    <div class="card-grid">
      <SurfaceCard eyebrow="Connection" title="Host status" tone="accent">
        <p class="metric">Not connected</p>
        <p class="card-copy">
          Connection, capability, and permission state will remain keyed to Local agent.
        </p>
      </SurfaceCard>
      <SurfaceCard eyebrow="Active server" title="Survival">
        <StatePanel
          kind="empty"
          title="No live data yet"
          message="The selected server's status will appear here without changing the shell layout."
        />
      </SurfaceCard>
      <SurfaceCard eyebrow="Console" title="Always available">
        <p class="card-copy">
          Console history is bounded per host and reconnects without showing another host's lines.
        </p>
        <p class="status" role="status">{shellMessage}</p>
      </SurfaceCard>
    </div>

    <small class="bundle-note">Bundle {bundleIdentity.id} · v{bundleIdentity.version}</small>
  </div>
</ApplicationShell>

<style>
  .dashboard {
    display: grid;
    gap: 1.5rem;
  }
  .intro-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 1.5rem;
  }
  .eyebrow {
    margin: 0 0 0.55rem;
    color: var(--msc-accent);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  h2 {
    max-width: 35rem;
    margin: 0;
    font-size: clamp(1.7rem, 5vw, 3rem);
    line-height: 1.05;
  }
  .intro-copy,
  .card-copy {
    max-width: 38rem;
    color: var(--msc-muted);
    line-height: 1.6;
  }
  .card-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
  }
  .metric {
    margin: 0;
    color: var(--msc-warning);
    font-size: 1.5rem;
    font-weight: 800;
  }
  .status {
    margin: 1rem 0 0;
    color: var(--msc-warning);
    font-size: 0.82rem;
  }
  .bundle-note {
    color: var(--msc-subtle);
  }
  @media (max-width: 900px) {
    .card-grid {
      grid-template-columns: 1fr 1fr;
    }
    .card-grid :global(.surface-card:last-child) {
      grid-column: 1 / -1;
    }
  }
  @media (max-width: 600px) {
    .intro-row {
      display: grid;
      align-items: start;
    }
    .card-grid {
      grid-template-columns: 1fr;
    }
    .card-grid :global(.surface-card:last-child) {
      grid-column: auto;
    }
  }
</style>
