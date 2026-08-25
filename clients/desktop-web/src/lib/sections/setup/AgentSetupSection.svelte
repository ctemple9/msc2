<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import StatePanel from '../../components/StatePanel.svelte';
  import SurfaceCard from '../../components/SurfaceCard.svelte';
  import {
    getPlatform,
    type AgentReadiness,
    type AgentServiceAction,
    type AgentServiceStatus,
  } from '../../platform';
  import ScreenHeader from '../shared/ScreenHeader.svelte';

  export let readiness: AgentReadiness = 'starting';
  export let onAgentRetry: (() => void) | undefined = undefined;

  const readinessTitles: Record<AgentReadiness, string> = {
    missing: 'Agent not installed',
    stopped: 'Agent stopped',
    starting: 'Agent starting',
    ready: 'Agent ready',
    incompatible: 'Agent version incompatible',
    unavailable: 'Agent unavailable',
  };
  const readinessMessages: Record<AgentReadiness, string> = {
    missing: 'Install the local agent to continue. MSC will not install it automatically.',
    stopped: 'The installed agent is stopped. Start it, then reconnect to this computer.',
    starting: 'The agent is starting. Reconnect after its health endpoint responds.',
    ready: 'The local agent is ready for server management.',
    incompatible: 'This agent cannot serve the current client. Install a compatible update.',
    unavailable: 'MSC cannot reach or authenticate with the local agent. Reconnect or repair it.',
  };

  let status: AgentServiceStatus | undefined;
  let busy = false;
  let errorMessage = '';
  $: readinessTitle = readinessTitles[readiness];
  $: readinessMessage = readinessMessages[readiness];

  onMount(() => void refresh());

  async function refresh(): Promise<void> {
    status = await (await getPlatform()).agentServiceStatus();
  }

  async function manage(action: AgentServiceAction): Promise<void> {
    busy = true;
    errorMessage = '';
    try {
      status = await (await getPlatform()).manageAgentService(action);
      if (status.state === 'running') await onAgentRetry?.();
    } catch (error) {
      errorMessage = String(error);
    } finally {
      busy = false;
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="This computer"
    title="Local agent"
    description="The background agent keeps Minecraft servers running after this window closes."
    status={readinessTitle}
    statusTone={readiness === 'ready'
      ? 'positive'
      : readiness === 'incompatible'
        ? 'danger'
        : 'warning'}
    actionLabel={onAgentRetry ? 'Reconnect' : 'Refresh service status'}
    onAction={onAgentRetry ?? refresh}
  />

  <div class="screen-grid two">
    <SurfaceCard eyebrow="Connection readiness" title={readinessTitle}>
      <p class="metric-large">{readiness === 'ready' ? 'Ready' : 'Action needed'}</p>
      <p class="muted">{readinessMessage}</p>
    </SurfaceCard>

    <SurfaceCard eyebrow="Background service" title={status?.serviceName ?? 'MSC agent'}>
      <p class="metric-large">{status?.state ?? 'Checking'}</p>
      <p class="muted">{status?.detail ?? 'Looking for the local service.'}</p>
      {#if status?.pid}<p class="muted">Service process: {status.pid}</p>{/if}
    </SurfaceCard>

    <SurfaceCard eyebrow="Service actions" title="Keep servers independent of the window">
      {#if status?.available}
        <div class="actions">
          {#if status.state === 'not-installed'}
            <ActionButton
              label="Install and start local agent"
              disabled={busy}
              onclick={() => manage('install')}
            >
              Install and start
            </ActionButton>
          {:else}
            <ActionButton
              label="Start local agent"
              disabled={busy || status.state === 'running'}
              onclick={() => manage('start')}
            >
              Start agent
            </ActionButton>
            <ActionButton
              kind="quiet"
              label="Stop local agent"
              disabled={busy || status.state !== 'running'}
              onclick={() => manage('stop')}
            >
              Stop agent
            </ActionButton>
            <ActionButton
              kind="quiet"
              label="Repair local agent service"
              disabled={busy}
              onclick={() => manage('repair')}
            >
              Repair service
            </ActionButton>
          {/if}
        </div>
      {:else}
        <StatePanel
          kind="empty"
          title="Headless install"
          message={status?.detail ?? 'Use the headless package on this host.'}
        />
      {/if}
      <p class="muted">
        Stopping this service is explicit. Closing the app window never stops the service or any
        Minecraft server it manages.
      </p>
    </SurfaceCard>
  </div>

  {#if errorMessage}
    <StatePanel kind="error" title="Could not change the agent service" message={errorMessage} />
  {/if}
</div>

<style>
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
  }
</style>
