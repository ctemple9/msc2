<script lang="ts">
  import ActionButton from '../../components/ActionButton.svelte';
  import SurfaceCard from '../../components/SurfaceCard.svelte';
  import StatePanel from '../../components/StatePanel.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { ScreenProps } from '../shared/types';

  export let api: ScreenProps['api'] = undefined;
  export let serverId = 'survival';
  export let onFleet: (() => void) | undefined = undefined;

  let connection = api ? 'Connected' : 'Agent disconnected';
  let refreshed = false;

  async function refresh(): Promise<void> {
    if (api) {
      try {
        await api.get('/v1/status');
        connection = 'Connected';
      } catch {
        connection = 'Reconnecting';
      }
    }
    refreshed = true;
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Host overview"
    title="Home"
    description="Keep the selected host, active server, and the next safe action visible at a glance."
    status={connection}
    statusTone={connection === 'Connected' ? 'positive' : 'warning'}
    actionLabel="Refresh host"
    onAction={refresh}
  />

  <div class="screen-grid three">
    <SurfaceCard eyebrow="Active server" title={serverId === 'survival' ? 'Survival' : serverId}>
      <p class="metric-large">Stopped</p>
      <p class="muted">The agent will report live lifecycle state when this host is connected.</p>
      <ActionButton label="Open fleet" onclick={onFleet}>Manage servers</ActionButton>
    </SurfaceCard>
    <SurfaceCard eyebrow="Operations" title="Durable progress">
      <p class="metric-large">0 active</p>
      <p class="muted">Create, import, provisioning, and backup work survives reconnects.</p>
    </SurfaceCard>
    <SurfaceCard eyebrow="Safety" title="Explicit actions" tone="accent">
      <StatePanel
        kind="empty"
        title="Ready"
        message="Destructive actions name the host and server before they run."
      />
    </SurfaceCard>
  </div>

  {#if refreshed}<p class="muted" role="status">Host status refreshed for {serverId}.</p>{/if}
</div>
