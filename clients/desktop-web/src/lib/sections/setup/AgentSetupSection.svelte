<script lang="ts">
  import { onMount } from 'svelte';
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

  const serviceCommands = [
    'msc service status --service-name msc-agent',
    'msc service start --service-name msc-agent',
    'msc service stop --service-name msc-agent',
  ];
  const howItWorksStorageKey = 'msc2.agents.how-it-works-expanded';
  const pairingCommand = 'msc pairing create --client-kind desktop';
  const sshTunnelCommand = 'ssh -N -L 48002:127.0.0.1:48001 username@ip-address';

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
  let howItWorksExpanded = true;
  let readinessTone: 'ok' | 'warn' | 'error' = 'warn';
  let statusTone: 'ok' | 'warn' | 'error' = 'warn';
  let inspectedHostId: string | undefined;
  let refreshedReadyHostId: string | undefined;

  $: readinessTitle = readinessTitles[readiness];
  $: readinessMessage = readinessMessages[readiness];
  $: isLoopbackHost = loopbackHost(hostBaseUrl);
  $: isLocalDesktopHost = isDesktopShell && isLocalHost;
  $: isCurrentLocalAgent = isLocalHost && (isLocalDesktopHost || isLoopbackHost);
  $: readinessTone = readiness === 'ready' ? 'ok' : readiness === 'incompatible' ? 'error' : 'warn';
  $: statusTone =
    status?.state === 'running' ? 'ok' : status?.state === 'unavailable' ? 'error' : 'warn';
  $: serviceState = status?.state ?? 'checking';

  onMount(() => {
    const stored = localStorage.getItem(howItWorksStorageKey);
    if (stored === 'true' || stored === 'false') howItWorksExpanded = stored === 'true';
  });

  function toggleHowItWorks(): void {
    howItWorksExpanded = !howItWorksExpanded;
    localStorage.setItem(howItWorksStorageKey, String(howItWorksExpanded));
  }

  $: {
    if (!isLocalDesktopHost) {
      inspectedHostId = undefined;
      status = undefined;
      localPairingCode = '';
      refreshedReadyHostId = undefined;
    } else if (inspectedHostId !== hostId) {
      inspectedHostId = hostId;
      status = undefined;
      localPairingCode = '';
      refreshedReadyHostId = undefined;
      void refresh();
    }
  }

  // Reconnect changes the parent readiness state without remounting this
  // screen. Re-read the OS service once when that connection becomes ready so
  // the service card cannot keep the result from before reconnect.
  $: if (!isLocalDesktopHost || readiness !== 'ready') {
    refreshedReadyHostId = undefined;
  } else if (refreshedReadyHostId !== hostId) {
    refreshedReadyHostId = hostId;
    void refresh();
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
      <div class="heading-copy">
        <h1>Connect MSC 2 to an agent</h1>
        <p class="heading-intro">
          The control panel sends commands. The agent runs on the computer that owns your Minecraft
          servers.
        </p>
      </div>
      <Badge variant="status" tone={readinessTone}>{readinessTitle}</Badge>
    </div>
  </div>

  <Card as="section" padding="0">
    <button
      type="button"
      class="disclosure-header"
      aria-expanded={howItWorksExpanded}
      aria-controls="how-msc-works"
      onclick={toggleHowItWorks}
    >
      <span class="disclosure-title">
        <span class="msc2-type-overline">How MSC works</span>
        <strong>MSC has two parts</strong>
      </span>
      <span class="disclosure-action">{howItWorksExpanded ? 'Hide' : 'Show'}</span>
    </button>

    {#if howItWorksExpanded}
      <div id="how-msc-works" class="how-content">
        <p class="detail">
          The control panel is the app you use. The agent is the background service that manages the
          Minecraft servers.
        </p>
        <div
          class="architecture-diagram"
          aria-label="The control panel communicates with the agent"
        >
          <div class="architecture-node">
            <span class="node-kicker">What you use</span>
            <strong>Control panel</strong>
            <span>Desktop · browser · phone · CLI</span>
          </div>
          <div class="architecture-link" aria-hidden="true">
            <span>connects to</span>
            <strong>↔</strong>
          </div>
          <div class="architecture-node">
            <span class="node-kicker">Where servers run</span>
            <strong>MSC agent</strong>
            <span>{hostLabel}</span>
          </div>
        </div>
        <div class="means-grid">
          <p>
            <strong>The agent owns the work.</strong> It manages files, processes, logs, and servers.
          </p>
          <p>
            <strong>Pairing grants permission.</strong> The address is how this client reaches the agent.
          </p>
          <p>
            <strong>Closing this window is safe.</strong> It does not stop the agent or a running server.
          </p>
        </div>
      </div>
    {/if}
  </Card>

  {#if isCurrentLocalAgent}
    <Card as="section">
      <div class="path-heading">
        <div>
          <p class="msc2-type-overline">Agent on this computer</p>
          <h2>Manage the local agent</h2>
        </div>
        <Badge variant="status" tone={readinessTone}>{readinessTitle}</Badge>
      </div>
      <p class="detail">
        Use this path when the Minecraft servers and this MSC control panel are on the same
        computer.
      </p>

      {#if isLocalDesktopHost}
        <p class="service-explanation">
          Install, start, stop, or repair the background service. The service continues running when
          you close this window.
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
      {:else if !isDesktopShell}
        {#if status?.state === 'not-installed'}
          <h3>Install the headless package first</h3>
          <p class="detail">Install the agent package, then return here and reconnect.</p>
        {:else}
          <p class="service-explanation">
            This browser can reach the local agent. Run service controls in Terminal on this
            computer.
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
      {/if}

      {#if isDesktopShell}
        <details class="secondary-disclosure">
          <summary>Pair another client with this agent</summary>
          <div class="secondary-content">
            <p class="detail">
              Start the agent first, then create a one-use code for another desktop, browser, or
              phone to connect to {hostLabel}.
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
                variant="secondary"
                disabled={readiness !== 'ready' || localPairingBusy}
                onclick={() => void createPairingCode()}
              >
                {readiness === 'ready' ? 'Create pairing code' : 'Start agent to create a code'}
              </Button>
            {/if}
          </div>
        </details>
      {/if}
    </Card>
  {:else}
    <Card as="section">
      <div class="path-heading">
        <div>
          <p class="msc2-type-overline">Current agent</p>
          <h2>Manage the agent on {hostLabel}</h2>
        </div>
        <Badge variant="status" tone={readinessTone}>{readinessTitle}</Badge>
      </div>
      <p class="detail">
        This agent runs on another computer. Start, stop, and repair it on that computer; this
        client can manage its Minecraft servers once the connection is available.
      </p>
      {#if isDesktopShell && onPairAgain}
        <p class="service-explanation">
          To replace this client’s credential, run <span class="mono">{pairingCommand}</span> on
          {hostLabel}, then paste the new code here.
        </p>
        <div class="pairing-row">
          <Field bind:value={pairingCode} placeholder="pair_…" />
          <Button
            variant="primary"
            disabled={pairingBusy || !pairingCode.trim()}
            onclick={() => void pairAgain()}>Pair again</Button
          >
        </div>
      {/if}
      <div class="actions reconnect-only">
        <Button variant="secondary" onclick={() => void (onAgentRetry ? onAgentRetry() : refresh())}
          >{onAgentRetry ? 'Reconnect' : 'Refresh status'}</Button
        >
      </div>
    </Card>
  {/if}

  <Card as="section">
    <div class="path-heading">
      <div>
        <p class="msc2-type-overline">Agent on another computer</p>
        <h2>Connect another agent</h2>
      </div>
      {#if isDesktopShell}<span class="quiet-label">Desktop app</span>{/if}
    </div>
    <p class="detail">
      Use this path when the Minecraft servers live somewhere else. The other computer must have the
      agent installed and running.
    </p>

    {#if isDesktopShell}
      <ol class="connection-steps">
        <li>
          <details class="connection-step" open>
            <summary>
              <span class="step-number">1</span>
              <span class="step-title">Start the agent on the other computer</span>
            </summary>
            <div class="step-content">
              <p class="detail">
                Go to the computer where the Minecraft servers will run. The agent must be running
                there before this computer can connect to it.
              </p>
              <p class="detail">
                If that computer has the MSC app, open it and click <strong>Start agent</strong>. If
                it is running the headless agent, open Terminal there and run:
              </p>
              <div class="command-row">
                <Field value={serviceCommands[1]} />
                <Button
                  size="sm"
                  variant="secondary"
                  onclick={() => void copyCommand(serviceCommands[1])}
                >
                  {copiedCommand === serviceCommands[1] ? 'Copied' : 'Copy'}
                </Button>
              </div>
              <p class="detail">
                If the agent is not installed yet, install the headless agent package first.
              </p>
            </div>
          </details>
        </li>
        <li>
          <details class="connection-step">
            <summary>
              <span class="step-number">2</span>
              <span class="step-title">Make the agent reachable from this computer</span>
            </summary>
            <div class="step-content">
              <p class="detail">
                Run the following command on this computer—the one where you are using the MSC
                desktop app:
              </p>
              <div class="command-row">
                <Field value={sshTunnelCommand} />
                <Button
                  size="sm"
                  variant="secondary"
                  onclick={() => void copyCommand(sshTunnelCommand)}
                >
                  {copiedCommand === sshTunnelCommand ? 'Copied' : 'Copy'}
                </Button>
              </div>
              <p class="detail">
                Keep this Terminal window open while using MSC. The tunnel carries this computer’s
                local address <span class="mono">127.0.0.1:48002</span> to the agent’s address on the
                other computer.
              </p>
              <p class="detail">
                Replace <span class="mono">username@ip-address</span> with the username you use to
                sign in to the other computer, followed by <span class="mono">@</span> and that
                computer’s IP address. For example:
                <span class="mono">camerontemple@10.0.0.156</span>.
              </p>
              <p class="detail">
                If you do not know them, run <span class="mono">whoami</span> on the other computer
                to find its username and <span class="mono">hostname -I</span> to find its network address.
              </p>
            </div>
          </details>
        </li>
        <li>
          <details class="connection-step">
            <summary>
              <span class="step-number">3</span>
              <span class="step-title">Create a pairing code on the other computer</span>
            </summary>
            <div class="step-content">
              <p class="detail">
                Open Terminal on the computer running the agent—or SSH into it—and run this command
                there:
              </p>
              <div class="command-row">
                <Field value={pairingCommand} />
                <Button
                  size="sm"
                  variant="secondary"
                  onclick={() => void copyCommand(pairingCommand)}
                >
                  {copiedCommand === pairingCommand ? 'Copied' : 'Copy'}
                </Button>
              </div>
              <p class="detail">
                Copy the one-use code that appears. Do not run this command on the computer running
                this MSC desktop app. The code expires automatically and is exchanged for a lasting
                client credential.
              </p>
            </div>
          </details>
        </li>
        <li>
          <details class="connection-step">
            <summary>
              <span class="step-number">4</span>
              <span class="step-title">Enter the address and pairing code below</span>
            </summary>
            <div class="step-content">
              <p class="detail">
                For the SSH tunnel above, enter
                <span class="mono">http://127.0.0.1:48002</span> as the agent address. This is the local
                end of the tunnel, not the other computer’s IP address.
              </p>
              <p class="detail">
                Enter a name for the host, paste the pairing code from step 3, and click
                <strong>Connect agent</strong>. The name is how this computer will identify the host
                in MSC.
              </p>
            </div>
          </details>
        </li>
      </ol>

      <div class="remote-pairing-form">
        <label class="field-label">
          Host name
          <Field bind:value={remoteHostLabel} placeholder="Home server" />
        </label>
        <label class="field-label">
          Agent address
          <Field bind:value={remoteHostUrl} placeholder="http://127.0.0.1:48002" />
        </label>
        <label class="field-label">
          Pairing code
          <Field bind:value={remotePairingCode} placeholder="Code from the other computer" />
        </label>
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
    {:else}
      <p class="service-explanation">
        Connecting to another agent is available from the desktop app. This browser can manage the
        current host once it is connected.
      </p>
    {/if}
  </Card>

  <Card as="section">
    <div class="status-summary">
      <div class="status-item">
        <div class="card-heading">
          <span class="msc2-type-overline">Connection</span>
          <Badge variant="status" tone={readinessTone}>{readinessTitle}</Badge>
        </div>
        <StatusDot tone={readinessTone} label={readinessTitle} />
        <p class="detail">{readinessMessage}</p>
      </div>
      <div class="status-item">
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
            This browser can reach the local agent, but Terminal manages its service.
          </p>
        {:else}
          <StatusDot tone="warn" label={`Managed on ${hostLabel}`} />
          <p class="detail">
            This client can manage Minecraft, but the service is managed on {hostLabel}.
          </p>
        {/if}
      </div>
    </div>
  </Card>

  {#if errorMessage || browserHandoffError}
    <p class="error" role="alert">
      {browserHandoffError || `Could not change the agent service: ${errorMessage}`}
    </p>
  {/if}
</div>

<style>
  .heading {
    display: grid;
    gap: 8px;
  }
  .breadcrumb,
  .msc2-type-overline,
  h1,
  h2,
  h3,
  p {
    margin: 0;
  }
  .breadcrumb {
    color: var(--msc2-text-secondary);
    font-size: 13px;
    font-weight: 500;
  }
  .heading-row,
  .path-heading,
  .card-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }
  .heading-copy {
    display: grid;
    gap: 6px;
  }
  h1 {
    font-size: 22px;
    font-weight: 600;
  }
  h2 {
    margin-top: 6px;
    font-size: 17px;
    font-weight: 500;
  }
  h3 {
    margin-top: 14px;
    font-size: 15px;
    font-weight: 500;
  }
  .heading-intro,
  .detail,
  .quiet-label,
  .service-explanation {
    color: var(--msc2-text-secondary);
  }
  .heading-intro {
    max-width: 640px;
    font-size: 13px;
    line-height: 1.5;
  }
  .detail,
  .service-explanation {
    margin-top: 8px;
    font-size: 13px;
    line-height: 1.5;
  }
  .quiet-label {
    font-size: 12px;
  }
  .disclosure-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 15px 16px;
    color: var(--msc2-text-primary);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
  }
  .disclosure-header:hover {
    background: rgba(255, 255, 255, 0.03);
  }
  .disclosure-header:focus-visible,
  summary:focus-visible {
    outline: 2px solid var(--msc2-hairline-field-focus);
    outline-offset: -2px;
  }
  .disclosure-title {
    display: grid;
    gap: 4px;
  }
  .disclosure-title strong {
    font-size: 15px;
    font-weight: 500;
  }
  .disclosure-action {
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .how-content {
    display: grid;
    gap: 16px;
    padding: 0 16px 16px;
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
    padding: 12px;
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
    color: var(--msc2-text-secondary);
    font-size: 11px;
    line-height: 1.4;
    overflow-wrap: anywhere;
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
    gap: 2px;
    justify-items: center;
    color: var(--msc2-text-tertiary);
  }
  .architecture-link strong {
    font-size: 22px;
    font-weight: 400;
    line-height: 1;
  }
  .means-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--msc2-hairline-faint);
  }
  .means-grid p {
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }
  .means-grid strong {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .path-heading {
    align-items: flex-start;
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
  .command-list,
  .remote-pairing-form {
    display: grid;
    gap: 10px;
    margin-top: 14px;
  }
  .command-row,
  .pairing-row,
  .pairing-code-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }
  .field-label {
    display: grid;
    gap: 5px;
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .connection-steps {
    display: grid;
    gap: 7px;
    margin: 16px 0 0;
    padding-left: 0;
    color: var(--msc2-text-secondary);
    font-size: 13px;
    line-height: 1.5;
    list-style: none;
  }
  .connection-step {
    padding-left: 4px;
  }
  .connection-step summary {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    color: var(--msc2-text-primary);
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    list-style: none;
  }
  .connection-step summary::-webkit-details-marker {
    display: none;
  }
  .step-number {
    display: inline-grid;
    width: 20px;
    height: 20px;
    align-items: center;
    justify-content: center;
    color: var(--msc2-text-secondary);
    background: var(--msc2-tier-chrome);
    border-radius: 50%;
    font-size: 11px;
    font-weight: 600;
  }
  .connection-step[open] .step-number {
    color: var(--msc2-neutral-fill-ink);
    background: var(--msc2-neutral-fill);
  }
  .step-title {
    min-width: 0;
  }
  .step-content {
    display: grid;
    gap: 10px;
    padding: 4px 0 8px 30px;
  }
  .step-content .detail {
    margin-top: 0;
  }
  .connection-steps .mono,
  .mono {
    color: var(--msc2-text-primary);
    font-family: var(--msc2-font-mono, monospace);
    font-size: 12px;
  }
  .secondary-disclosure {
    margin-top: 16px;
    border-top: 1px solid var(--msc2-hairline-faint);
  }
  .secondary-disclosure summary {
    padding: 12px 0 0;
    color: var(--msc2-text-primary);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
  }
  .secondary-content {
    display: grid;
    gap: 12px;
    padding-top: 10px;
  }
  .pairing-expiry {
    margin-top: 7px;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }
  .status-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 24px;
  }
  .status-item {
    display: grid;
    align-content: start;
    gap: 8px;
  }
  .status-item .detail {
    margin-top: 0;
  }
  .error {
    color: var(--msc2-status-error);
    font-size: 13px;
    line-height: 1.5;
  }
  @media (max-width: 760px) {
    .architecture-diagram,
    .means-grid,
    .status-summary {
      grid-template-columns: 1fr;
    }
    .architecture-link {
      transform: rotate(90deg);
    }
  }
</style>
