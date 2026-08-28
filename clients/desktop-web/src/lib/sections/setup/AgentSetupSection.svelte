<script lang="ts">
  import Badge from '../../components/base/Badge.svelte';
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import Field from '../../components/base/Field.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import {
    getPlatform,
    type AgentReadiness,
    type AgentServiceAction,
    type AgentServiceStatus,
  } from '../../platform';

  export let readiness: AgentReadiness = 'starting';
  export let onAgentRetry: (() => void) | undefined = undefined;
  export let hostId = '';
  export let hostLabel = 'Local agent';
  export let hostBaseUrl = 'http://127.0.0.1:48001';
  export let isDesktopShell = false;
  export let isLocalHost = true;
  export let browserHandoffError = '';
  export let onPairAgain: ((pairingCode: string) => Promise<void>) | undefined = undefined;

  const readinessTitles: Record<AgentReadiness, string> = {
    missing: 'Agent not installed',
    stopped: 'Agent stopped',
    starting: 'Agent starting',
    ready: 'Agent ready',
    incompatible: 'Agent version incompatible',
    unavailable: 'Agent unavailable',
  };
  const readinessMessages: Record<AgentReadiness, string> = {
    missing: 'Install the local agent, then continue into host setup.',
    stopped: 'The installed agent is stopped. Start it, then continue into host setup.',
    starting: 'The agent is starting. Reconnect after its health endpoint responds.',
    ready: 'The local agent is ready for server management.',
    incompatible: 'This agent cannot serve the current client. Install a compatible update.',
    unavailable: 'MSC cannot reach or authenticate with the local agent. Reconnect or repair it.',
  };

  let status: AgentServiceStatus | undefined;
  let busy = false;
  let errorMessage = '';
  let pairingCode = '';
  let pairingBusy = false;
  let copiedCommand = '';
  let readinessTone: 'ok' | 'warn' | 'error' = 'warn';
  let statusTone: 'ok' | 'warn' | 'error' = 'warn';
  let inspectedHostId: string | undefined;
  $: readinessTitle = readinessTitles[readiness];
  $: readinessMessage = readinessMessages[readiness];
  $: isLoopbackHost = loopbackHost(hostBaseUrl);
  $: isLocalDesktopHost = isDesktopShell && isLocalHost;
  $: readinessTone = readiness === 'ready' ? 'ok' : readiness === 'incompatible' ? 'error' : 'warn';
  $: statusTone =
    status?.state === 'running' ? 'ok' : status?.state === 'unavailable' ? 'error' : 'warn';
  $: serviceState = status?.state ?? 'checking';
  const serviceCommands = [
    'msc service status --service-name msc-agent',
    'msc service start --service-name msc-agent',
    'msc service stop --service-name msc-agent',
  ];

  $: {
    if (!isLocalDesktopHost) {
      inspectedHostId = undefined;
      status = undefined;
    } else if (inspectedHostId !== hostId) {
      inspectedHostId = hostId;
      void refresh();
    }
  }

  async function refresh(): Promise<void> {
    if (!isLocalDesktopHost) return;
    const requestedHostId = hostId;
    const nextStatus = await (await getPlatform()).agentServiceStatus();
    if (isLocalDesktopHost && hostId === requestedHostId) status = nextStatus;
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

  async function pairAgain(): Promise<void> {
    const code = pairingCode.trim();
    if (!onPairAgain || !code || pairingBusy) return;
    pairingBusy = true;
    errorMessage = '';
    try {
      await onPairAgain(code);
      pairingCode = '';
    } catch (error) {
      errorMessage = String(error);
    } finally {
      pairingBusy = false;
    }
  }

  function loopbackHost(baseUrl: string): boolean {
    try {
      const { hostname } = new URL(baseUrl);
      return hostname === '127.0.0.1' || hostname === 'localhost' || hostname === '::1';
    } catch {
      return false;
    }
  }

  async function copyCommand(command: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(command);
      copiedCommand = command;
      setTimeout(() => {
        if (copiedCommand === command) copiedCommand = '';
      }, 1500);
    } catch {
      // The command remains selectable in its field when clipboard access is unavailable.
    }
  }
</script>

<div class="screen">
  <div class="heading">
    <p class="breadcrumb">{hostLabel} agent</p>
    <div class="heading-row">
      <div>
        <h1>Background agent</h1>
        <p>The agent keeps Minecraft servers running after this window closes.</p>
      </div>
      <Button variant="secondary" onclick={onAgentRetry ?? refresh}>
        {onAgentRetry ? 'Reconnect' : 'Refresh status'}
      </Button>
    </div>
  </div>

  <div class="screen-grid two">
    <Card>
      <div class="card-heading">
        <span class="msc2-type-overline">Connection</span>
        <Badge variant="status" tone={readinessTone}>{readinessTitle}</Badge>
      </div>
      <StatusDot tone={readinessTone} label={readinessTitle} />
      <p class="detail">{readinessMessage}</p>
    </Card>

    <Card>
      <div class="card-heading">
        <span class="msc2-type-overline">Background service</span>
        <Badge variant="status" tone={isLocalDesktopHost ? statusTone : 'warn'}
          >{isLocalDesktopHost ? serviceState : 'host-managed'}</Badge
        >
      </div>
      {#if isLocalDesktopHost}
        <StatusDot tone={statusTone} label={serviceState} />
        <p class="detail">{status?.detail ?? 'Looking for the local service.'}</p>
        {#if status?.pid}<p class="detail">Service process: {status.pid}</p>{/if}
      {:else if !isDesktopShell && isLoopbackHost}
        <StatusDot tone="warn" label="Service status needs Terminal" />
        <p class="detail">
          This browser can reach the local agent, but only Terminal can inspect or change its
          background service.
        </p>
      {:else}
        <StatusDot tone="warn" label={`Managed on ${hostLabel}`} />
        <p class="detail">
          This client can manage Minecraft through this agent, but its background service is managed
          on {hostLabel}.
        </p>
      {/if}
    </Card>
  </div>

  <Card>
    {#if isLocalDesktopHost}
      <div class="card-heading">
        <span class="msc2-type-overline">Service controls</span>
        <span class="quiet-label">This computer</span>
      </div>
      <h2>Keep servers independent of the window</h2>
      <p class="detail">Use the installed desktop app to change this computer’s agent service.</p>
      <p class="detail">Closing the app window never stops the service.</p>
      <p class="detail">It never stops any Minecraft server either.</p>
      <div class="actions">
        {#if readiness === 'missing' || status?.state === 'not-installed'}
          <Button variant="primary" disabled={busy} onclick={() => manage('install')}
            >Install and Continue</Button
          >
        {:else if readiness === 'stopped' || status?.state === 'stopped'}
          <Button variant="primary" disabled={busy} onclick={() => manage('start')}
            >Start and Continue</Button
          >
        {:else if readiness === 'incompatible'}
          <Button variant="secondary" disabled={busy} onclick={() => manage('repair')}
            >Repair service</Button
          >
        {:else}
          <Button
            variant="start"
            disabled={busy || status?.state === 'running'}
            onclick={() => manage('start')}>Start agent</Button
          >
          <Button
            variant="stop"
            disabled={busy || status?.state !== 'running'}
            onclick={() => manage('stop')}>Stop agent</Button
          >
          <Button variant="secondary" disabled={busy} onclick={() => manage('repair')}
            >Repair service</Button
          >
        {/if}
      </div>
    {:else if !isDesktopShell && isLoopbackHost}
      <div class="card-heading">
        <span class="msc2-type-overline">Terminal controls</span>
        <span class="quiet-label">This computer</span>
      </div>
      {#if status?.state === 'not-installed'}
        <h2>Install the headless package first</h2>
        <p class="detail">
          This agent is not installed. Reinstall it with this platform’s headless package or
          installer, then return here.
        </p>
      {:else}
        <h2>Run one command in Terminal</h2>
        <p class="detail">
          These commands control the agent on this same machine. They do not stop any Minecraft
          server when this page closes.
        </p>
        <div class="command-list">
          {#each serviceCommands as command (command)}
            <div class="command-row">
              <Field value={command} />
              <Button size="sm" variant="secondary" onclick={() => void copyCommand(command)}>
                {copiedCommand === command ? 'Copied' : 'Copy'}
              </Button>
            </div>
          {/each}
        </div>
      {/if}
    {:else if isDesktopShell && onPairAgain}
      <div class="card-heading">
        <span class="msc2-type-overline">Fresh pairing</span>
        <span class="quiet-label">{hostLabel}</span>
      </div>
      <h2>Pair this host again</h2>
      <p class="detail">
        Run <span class="mono">msc pairing create</span> on {hostLabel}, then paste its one-use code
        here. Pairing replaces the old credential and reopens host setup.
      </p>
      <div class="pairing-row">
        <Field bind:value={pairingCode} placeholder="pair_…" />
        <Button
          variant="primary"
          disabled={pairingBusy || !pairingCode.trim()}
          onclick={() => void pairAgain()}>Pair Again</Button
        >
      </div>
    {:else}
      <div class="card-heading">
        <span class="msc2-type-overline">Service controls</span>
        <span class="quiet-label">Another computer</span>
      </div>
      <h2>Run service controls on {hostLabel}</h2>
      <p class="detail">
        The selected agent is on another computer. Run service controls there; this client cannot
        install, start, stop, or repair it.
      </p>
    {/if}
  </Card>

  {#if errorMessage || browserHandoffError}<p class="error" role="alert">
      {browserHandoffError || `Could not change the agent service: ${errorMessage}`}
    </p>{/if}
</div>

<style>
  .heading {
    display: grid;
    gap: 8px;
  }
  .breadcrumb,
  .msc2-type-overline {
    margin: 0;
  }
  .breadcrumb {
    color: var(--msc2-text-secondary);
    font-size: 13px;
    font-weight: 500;
  }
  .heading-row,
  .card-heading,
  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }
  .heading-row,
  .card-heading {
    justify-content: space-between;
  }
  h1,
  h2,
  p {
    margin: 0;
  }
  h1 {
    font-size: 22px;
    font-weight: 600;
  }
  h2 {
    margin-top: 14px;
    font-size: 15px;
    font-weight: 500;
  }
  .heading-row p,
  .detail,
  .quiet-label {
    color: var(--msc2-text-secondary);
  }
  .heading-row p,
  .detail {
    margin-top: 5px;
    line-height: 1.5;
  }
  .quiet-label {
    font-size: 12px;
  }
  .actions {
    margin-top: 14px;
  }
  .command-list {
    display: grid;
    gap: 8px;
    margin-top: 14px;
  }
  .command-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }
  .pairing-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    margin-top: 14px;
  }
  .mono {
    font-family: var(--msc2-font-mono, monospace);
    color: var(--msc2-text-primary);
  }
  .error {
    color: var(--msc2-status-error);
    font-size: 13px;
    line-height: 1.5;
  }
</style>
