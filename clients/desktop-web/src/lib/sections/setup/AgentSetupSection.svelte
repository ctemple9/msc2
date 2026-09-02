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
  import type { Schema, ScreenApi } from '../shared/types';

  export let readiness: AgentReadiness = 'starting';
  export let onAgentRetry: (() => void) | undefined = undefined;
  export let hostId = '';
  export let hostLabel = 'Local agent';
  export let hostBaseUrl = 'http://127.0.0.1:48001';
  export let isDesktopShell = false;
  export let isLocalHost = true;
  export let browserHandoffError = '';
  export let onPairAgain: ((pairingCode: string) => Promise<void>) | undefined = undefined;
  export let onConnectHost:
    ((label: string, baseUrl: string, pairingCode: string) => Promise<void>) | undefined =
    undefined;
  export let api: ScreenApi | undefined = undefined;

  const readinessTitles: Record<AgentReadiness, string> = {
    missing: 'Agent not installed',
    stopped: 'Agent stopped',
    starting: 'Agent starting',
    ready: 'Agent connected',
    incompatible: 'Agent version incompatible',
    unavailable: 'Agent unavailable',
  };
  const readinessMessages: Record<AgentReadiness, string> = {
    missing: 'Install the local agent, then continue into host setup.',
    stopped: 'The installed agent is stopped. Start it, then continue into host setup.',
    starting: 'The agent is starting. Reconnect after its health endpoint responds.',
    ready: 'The local agent is connected and ready for server management.',
    incompatible: 'This agent cannot serve the current client. Install a compatible update.',
    unavailable: 'MSC cannot reach or authenticate with the local agent. Reconnect or repair it.',
  };

  let status: AgentServiceStatus | undefined;
  let busy = false;
  let errorMessage = '';
  let pairingCode = '';
  let pairingBusy = false;
  let localPairingCode = '';
  let localPairingBusy = false;
  let copiedPairingCode = false;
  let remoteHostLabel = '';
  let remoteHostUrl = '';
  let remotePairingCode = '';
  let remotePairingBusy = false;
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
      localPairingCode = '';
    } else if (inspectedHostId !== hostId) {
      inspectedHostId = hostId;
      localPairingCode = '';
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
      // Re-run the parent connection flow after every service change. A stop
      // must also clear the selected host's server snapshot immediately.
      await onAgentRetry?.();
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

  async function createPairingCode(): Promise<void> {
    if (!api || readiness !== 'ready' || localPairingBusy) return;
    localPairingBusy = true;
    errorMessage = '';
    try {
      const result = await api.post<Schema['PairingCreateResultDTO']>('/v1/auth/pairings', {
        clientKind: 'desktop',
        label: 'Desktop pairing',
        role: 'admin',
        permissions: [
          'serverControl',
          'players',
          'settings',
          'addons',
          'worlds',
          'broadcast',
          'networking',
          'fleet',
          'admin',
        ],
      });
      localPairingCode = result.pairingCode;
    } catch (error) {
      errorMessage = String(error);
    } finally {
      localPairingBusy = false;
    }
  }

  async function copyPairingCode(): Promise<void> {
    if (!localPairingCode) return;
    try {
      await navigator.clipboard.writeText(localPairingCode);
      copiedPairingCode = true;
      setTimeout(() => (copiedPairingCode = false), 1500);
    } catch {
      // The code remains selectable in its field when clipboard access is unavailable.
    }
  }

  async function connectHost(): Promise<void> {
    const label = remoteHostLabel.trim();
    const baseUrl = remoteHostUrl.trim();
    const code = remotePairingCode.trim();
    if (!onConnectHost || !label || !baseUrl || !code || remotePairingBusy) return;
    remotePairingBusy = true;
    errorMessage = '';
    try {
      await onConnectHost(label, baseUrl, code);
      remoteHostLabel = '';
      remoteHostUrl = '';
      remotePairingCode = '';
    } catch (error) {
      errorMessage = String(error);
    } finally {
      remotePairingBusy = false;
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
    <p class="breadcrumb">{hostLabel}</p>
    <div class="heading-row">
      <div>
        <h1>Background agent</h1>
      </div>
    </div>
  </div>

  <Card>
    {#if isLocalDesktopHost}
      <div class="card-heading">
        <span class="msc2-type-overline">Service controls</span>
        <span class="quiet-label">This computer</span>
      </div>
      <h2>Manage the agent service</h2>
      <p class="detail">
        Install, start, stop, or repair the agent. Closing this window does not stop it or any
        running Minecraft server.
      </p>
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
        {/if}
        <Button
          variant="secondary"
          disabled={busy}
          onclick={() => void (onAgentRetry ? onAgentRetry() : refresh())}
          >{onAgentRetry ? 'Reconnect' : 'Refresh status'}</Button
        >
        {#if readiness !== 'missing' && status?.state !== 'not-installed' && readiness !== 'stopped' && status?.state !== 'stopped' && readiness !== 'incompatible'}
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
        <div class="actions reconnect-only">
          <Button
            variant="secondary"
            onclick={() => void (onAgentRetry ? onAgentRetry() : refresh())}
            >{onAgentRetry ? 'Reconnect' : 'Refresh status'}</Button
          >
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
      <div class="actions reconnect-only">
        <Button variant="secondary" onclick={() => void (onAgentRetry ? onAgentRetry() : refresh())}
          >{onAgentRetry ? 'Reconnect' : 'Refresh status'}</Button
        >
      </div>
    {:else}
      <div class="card-heading">
        <span class="msc2-type-overline">Service controls</span>
        <span class="quiet-label">Another computer</span>
      </div>
      <h2>Manage the agent on {hostLabel}</h2>
      <p class="detail">Run service controls on the computer that hosts this agent.</p>
      <div class="actions reconnect-only">
        <Button variant="secondary" onclick={() => void (onAgentRetry ? onAgentRetry() : refresh())}
          >{onAgentRetry ? 'Reconnect' : 'Refresh status'}</Button
        >
      </div>
    {/if}
  </Card>

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

  {#if isDesktopShell}
    <Card>
      <div class="card-heading">
        <span class="msc2-type-overline">Host pairing</span>
        <span class="quiet-label">One-use code · 10 minutes</span>
      </div>
      <div class="pairing-grid">
        <div class="pairing-block">
          <h2>Show this agent’s pairing code</h2>
          <p class="detail">
            Start this agent first. Then create a code to connect another desktop, phone, or browser
            to {hostLabel}.
          </p>
          {#if localPairingCode}
            <div class="pairing-code-row">
              <Field value={localPairingCode} />
              <Button variant="secondary" onclick={() => void copyPairingCode()}>
                {copiedPairingCode ? 'Copied' : 'Copy'}
              </Button>
            </div>
            <p class="pairing-expiry">This code expires automatically and can be used once.</p>
          {:else}
            <Button
              variant="primary"
              disabled={readiness !== 'ready' || localPairingBusy}
              onclick={() => void createPairingCode()}
            >
              {readiness === 'ready' ? 'Create pairing code' : 'Start agent to create a code'}
            </Button>
          {/if}
        </div>

        <div class="pairing-block">
          <h2>Connect to another agent</h2>
          <p class="detail">
            Start the other agent, create its pairing code, then enter the code and address here.
          </p>
          <div class="remote-pairing-form">
            <Field bind:value={remoteHostLabel} placeholder="Label, e.g. Home server" />
            <Field bind:value={remoteHostUrl} placeholder="http://192.168.1.20:48001" />
            <Field bind:value={remotePairingCode} placeholder="Pairing code from that agent" />
            <Button
              variant="primary"
              disabled={remotePairingBusy ||
                !remoteHostLabel.trim() ||
                !remoteHostUrl.trim() ||
                !remotePairingCode.trim()}
              onclick={() => void connectHost()}
            >
              {remotePairingBusy ? 'Connecting…' : 'Connect agent'}
            </Button>
          </div>
        </div>
      </div>
    </Card>
  {/if}

  <Card as="section" padding="18px 20px">
    <div class="architecture">
      <div class="architecture-copy">
        <p class="msc2-type-overline">How MSC works</p>
        <h2>MSC has two parts</h2>
        <ol class="architecture-parts">
          <li>
            <strong>The control panel</strong> — this desktop window, browser page, phone app, or CLI.
            It sends commands and displays information.
          </li>
          <li>
            <strong>The agent</strong> — a small background service running on the computer that owns
            the Minecraft servers. It manages server files, starts processes, reads logs, and keeps servers
            running.
          </li>
        </ol>
        <p class="architecture-analogy">
          The control panel is like a remote control. The agent is the machinery doing the work.
        </p>
      </div>

      <div class="architecture-diagram" aria-label="The control panel communicates with the agent">
        <div class="architecture-node">
          <span class="node-kicker">What you use</span>
          <strong>Control panel</strong>
          <span>Desktop · phone · CLI</span>
        </div>
        <div class="architecture-link" aria-hidden="true">
          <span>communicates with</span>
          <strong>↔</strong>
        </div>
        <div class="architecture-node">
          <span class="node-kicker">Where Minecraft runs</span>
          <strong>MSC agent</strong>
          <span>{hostLabel}</span>
        </div>
      </div>
    </div>

    <div class="architecture-means">
      <p class="msc2-type-overline">That means</p>
      <ul>
        <li>Closing the control panel does not stop the agent.</li>
        <li>Closing the control panel does not stop a running Minecraft server.</li>
        <li>A browser or phone can manage a server without running the server itself.</li>
        <li>One control panel can connect to multiple agents on different computers.</li>
        <li>Installing or starting the agent must happen on the agent’s computer.</li>
      </ul>
    </div>
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
  .card-heading {
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
  .detail,
  .quiet-label {
    color: var(--msc2-text-secondary);
  }
  .detail {
    margin-top: 5px;
    line-height: 1.5;
  }
  .architecture {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(260px, 0.9fr);
    gap: 24px;
    align-items: center;
  }
  .architecture h2 {
    margin-top: 7px;
    font-size: 17px;
  }
  .architecture-parts,
  .architecture-means ul {
    margin: 12px 0 0;
    padding-left: 20px;
    color: var(--msc2-text-secondary);
    font-size: 13px;
    line-height: 1.55;
  }
  .architecture-parts li + li,
  .architecture-means li + li {
    margin-top: 7px;
  }
  .architecture-parts strong {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .architecture-analogy {
    margin: 14px 0 0;
    color: var(--msc2-text-primary);
    font-size: 13px;
    line-height: 1.5;
  }
  .architecture-diagram {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    gap: 10px;
    align-items: center;
  }
  .architecture-node {
    display: grid;
    gap: 5px;
    min-width: 0;
    padding: 13px 12px;
    background: var(--msc2-tier-chrome);
    border-radius: 9px;
    justify-items: center;
    text-align: center;
  }
  .architecture-node strong {
    color: var(--msc2-text-primary);
    font-size: 13px;
    font-weight: 500;
  }
  .architecture-node span:last-child {
    overflow-wrap: anywhere;
    color: var(--msc2-text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .node-kicker,
  .architecture-link span {
    color: var(--msc2-text-tertiary);
    font-size: 9px;
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }
  .architecture-link {
    display: grid;
    justify-items: center;
    gap: 2px;
    color: var(--msc2-text-tertiary);
  }
  .architecture-link strong {
    font-size: 22px;
    font-weight: 400;
    line-height: 1;
  }
  .architecture-means {
    margin-top: 18px;
    padding-top: 16px;
    border-top: 1px solid var(--msc2-hairline-faint);
  }
  .architecture-means ul {
    columns: 2;
    column-gap: 28px;
    margin-top: 8px;
  }
  .architecture-means li {
    break-inside: avoid;
  }
  .quiet-label {
    font-size: 12px;
  }
  .actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-top: 14px;
  }
  .actions :global(.btn) {
    width: 100%;
  }
  .actions.reconnect-only {
    grid-template-columns: minmax(0, 1fr);
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
  .pairing-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 24px;
    margin-top: 14px;
  }
  .pairing-block {
    display: grid;
    align-content: start;
    gap: 10px;
  }
  .pairing-block h2 {
    margin: 0;
  }
  .pairing-code-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }
  .pairing-expiry {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }
  .remote-pairing-form {
    display: grid;
    gap: 8px;
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
  @media (max-width: 760px) {
    .architecture {
      grid-template-columns: 1fr;
    }
    .architecture-means ul {
      columns: 1;
    }
    .pairing-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
