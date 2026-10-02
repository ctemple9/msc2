<script lang="ts">
  import Button from '../../components/base/Button.svelte';
  import ConfirmDialog from '../../components/ConfirmDialog.svelte';
  import Field from '../../components/base/Field.svelte';
  import SegmentedControl from '../../components/base/SegmentedControl.svelte';
  import {
    getPlatform,
    type AgentReadiness,
    type AgentServiceAction,
    type AgentServiceStatus,
  } from '../../platform';
  import {
    LOCAL_HOST_ID,
    hostAddressSummary,
    type HostId,
    type HostRecord,
    type HostRoute,
    type RemoteHostConnectionInput,
  } from '../../hosts/types';
  import RemoteConnectionWizard from './connection/RemoteConnectionWizard.svelte';
  import type { Schema } from '../shared/types';
  import type { RemoteDesktopPairingResult } from '../../auth/desktop';
  import { formatConnectionFailure } from '../../hosts/connection-errors';

  export let readiness: AgentReadiness = 'starting';
  export let onAgentRetry: (() => void | Promise<void>) | undefined = undefined;
  export let hostId = '';
  export let active = true;
  export let servers: readonly Schema['ServerDTO'][] = [];
  export let serverId = '';
  export let serverRunning = false;
  export let onOpenServer: ((id: string) => Promise<void>) | undefined = undefined;
  export let onFleet: (() => void) | undefined = undefined;
  export let hostLabel = 'Local agent';
  export let hostBaseUrl = 'http://127.0.0.1:48001';
  export let hosts: readonly HostRecord[] = [];
  export let activeHostId: HostId = '';
  export let hostSummaries: ReadonlyMap<HostId, { connection: string; serverCount: number }> =
    new Map();
  export let isDesktopShell = false;
  export let isLocalHost = true;
  export let onPairAgain: ((pairingCode: string) => Promise<void>) | undefined = undefined;
  export let onConnectHost:
    ((input: RemoteHostConnectionInput) => Promise<RemoteDesktopPairingResult | void>) | undefined =
    undefined;
  export let onRemoveHost: (() => Promise<void>) | undefined = undefined;
  export let onSwitchHost: ((hostId: HostId) => void | Promise<void>) | undefined = undefined;
  export let onSelectRoute: ((hostId: HostId, route: HostRoute) => Promise<void>) | undefined =
    undefined;
  export let onRemoveSavedHost: ((hostId: HostId) => Promise<void>) | undefined = undefined;

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
    starting: 'The agent is starting. MSC will connect when it is ready.',
    ready: 'The local agent is connected and ready for server management.',
    incompatible: 'This agent cannot serve the current client. Install a compatible update.',
    unavailable:
      'MSC cannot reach or authenticate with the local agent. Repair the agent or stop and start it.',
  };

  const pairingCommand = 'msc pairing create --client-kind desktop';
  const linuxServiceName = '<agent-service-name>';
  const commonServiceCommands = [
    `msc service status --service-name ${linuxServiceName}`,
    `msc service start --service-name ${linuxServiceName}`,
    `msc service stop --service-name ${linuxServiceName}`,
  ];
  const linuxServiceNotes = [
    {
      label: 'Find the agent service name',
      command: "systemctl list-unit-files --type=service | grep -i 'msc.*agent'",
      note: 'Use the unit name shown by this command without its trailing .service suffix in place of <agent-service-name> below.',
    },
    { label: 'Check status', command: commonServiceCommands[0] },
    { label: 'Start', command: commonServiceCommands[1] },
    { label: 'Stop', command: commonServiceCommands[2] },
  ];

  let status: AgentServiceStatus | undefined;
  let busy = false;
  let errorMessage = '';
  let pairingCode = '';
  let pairingBusy = false;
  let removeHostOpen = false;
  let removeHostBusy = false;
  let copiedCommand = '';
  let remoteMenuOpen = false;
  let remoteTrigger: HTMLButtonElement | undefined;
  let remoteView: 'new' | 'saved' | undefined;
  let editingHostId: HostId | undefined;
  let openingServerId = '';
  let statusRequest = 0;
  let inspectedHostId: string | undefined;
  let refreshedReadyHostId: string | undefined;
  let removeHostId = '';
  let removeHostLabel = '';

  $: readinessTitle = readinessTitles[readiness];
  $: readinessMessage = readinessMessages[readiness];
  $: isLoopbackHost = loopbackHost(hostBaseUrl);
  $: isLocalDesktopHost = isDesktopShell && isLocalHost;
  $: isCurrentLocalAgent = isLocalHost && (isLocalDesktopHost || isLoopbackHost);
  $: serviceState = status?.state ?? 'checking';
  $: savedHosts = hosts.filter((host) => host.id !== LOCAL_HOST_ID);
  $: editingHost = editingHostId ? savedHosts.find((host) => host.id === editingHostId) : undefined;

  $: selectedServer = servers.find((server) => server.id === serverId);

  function selectRemoteView(view: 'new' | 'saved'): void {
    remoteMenuOpen = false;
    remoteView = view;
    editingHostId = undefined;
  }

  function editSavedHost(hostId: HostId): void {
    editingHostId = hostId;
    remoteView = 'new';
    remoteMenuOpen = false;
  }

  function closeRemoteView(): void {
    remoteView = undefined;
    editingHostId = undefined;
  }

  async function openServer(id: string): Promise<void> {
    if (!onOpenServer || openingServerId) return;
    openingServerId = id;
    errorMessage = '';
    try {
      await onOpenServer(id);
    } catch (error) {
      errorMessage = formatConnectionFailure(error, 'network');
    } finally {
      openingServerId = '';
    }
  }

  async function connectSavedHost(id: HostId): Promise<void> {
    errorMessage = '';
    try {
      if (id === activeHostId) await onAgentRetry?.();
      else await onSwitchHost?.(id);
    } catch (error) {
      errorMessage = formatConnectionFailure(error, 'network');
    }
  }

  $: {
    if (!isLocalDesktopHost || !active) {
      inspectedHostId = undefined;
      status = undefined;
      refreshedReadyHostId = undefined;
    } else if (inspectedHostId !== hostId) {
      inspectedHostId = hostId;
      status = undefined;
      refreshedReadyHostId = undefined;
      void refresh();
    }
  }

  // Reconnect changes the parent readiness state without remounting this
  // screen. Re-read the OS service once when that connection becomes ready so
  // the service card cannot keep the result from before reconnect.
  $: if (!isLocalDesktopHost || !active || readiness !== 'ready') {
    refreshedReadyHostId = undefined;
  } else if (refreshedReadyHostId !== hostId) {
    refreshedReadyHostId = hostId;
    void refresh();
  }

  async function refresh(): Promise<void> {
    if (!isLocalDesktopHost || busy) return;
    const requestedHostId = hostId;
    const request = ++statusRequest;
    try {
      const nextStatus = await (await getPlatform()).agentServiceStatus();
      if (request === statusRequest && active && isLocalDesktopHost && hostId === requestedHostId)
        status = nextStatus;
    } catch (error) {
      if (request === statusRequest && active && isLocalDesktopHost && hostId === requestedHostId) {
        errorMessage = formatConnectionFailure(error, 'msc-agent');
      }
    }
  }

  async function manage(action: AgentServiceAction): Promise<void> {
    if (busy || !isLocalDesktopHost) return;
    busy = true;
    statusRequest += 1;
    const requestedHostId = hostId;
    errorMessage = '';
    try {
      const nextStatus = await (await getPlatform()).manageAgentService(action);
      if (!isLocalDesktopHost || hostId !== requestedHostId) return;
      status = nextStatus;
      // Re-run the parent connection flow after every service change. A stop
      // must also clear the selected host's server snapshot immediately.
      await onAgentRetry?.();
    } catch (error) {
      errorMessage = formatConnectionFailure(error, 'msc-agent');
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
      errorMessage = formatConnectionFailure(error, 'authentication');
    } finally {
      pairingBusy = false;
    }
  }

  function openRemoveHost(hostId: HostId, label: string): void {
    removeHostId = hostId;
    removeHostLabel = label;
    removeHostOpen = true;
  }

  async function removeConfirmedHost(): Promise<void> {
    if (!removeHostId || removeHostBusy) return;
    removeHostBusy = true;
    errorMessage = '';
    try {
      if (removeHostId === activeHostId) await onRemoveHost?.();
      else await onRemoveSavedHost?.(removeHostId);
      removeHostOpen = false;
      removeHostId = '';
      removeHostLabel = '';
    } catch (error) {
      errorMessage = formatConnectionFailure(error, 'authentication');
    } finally {
      removeHostBusy = false;
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

  async function connectRemoteHost(
    input: RemoteHostConnectionInput,
  ): Promise<RemoteDesktopPairingResult | void> {
    return onConnectHost?.(input);
  }

  function savedHostStatus(host: HostRecord): string {
    if (host.id === activeHostId) return readinessTitle;
    const connection = hostSummaries.get(host.id)?.connection;
    if (connection === 'connected') return 'Connected';
    if (connection === 'error') return 'Needs attention';
    return 'Saved';
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

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape' && remoteMenuOpen) {
      remoteMenuOpen = false;
      remoteTrigger?.focus();
    }
  }}
/>

<div class="agent-home">
  <header class="home-heading">
    <p class="msc2-type-overline">MSC 2</p>
    <h1>Your server home.</h1>
    <p>
      {readiness === 'ready'
        ? 'Manage Minecraft here. Your agent takes care of the work.'
        : 'First, choose where your Minecraft servers will run.'}
    </p>
  </header>

  <div class="home-grid">
    <section class="agent-panel" aria-label="Agent connection and setup">
      <div class="section-heading">
        <span class="msc2-type-overline"
          >{readiness === 'ready' ? 'Current agent' : 'Get connected'}</span
        >
        <span class="connection-state" role="status"
          >{readiness === 'ready' && isCurrentLocalAgent
            ? 'Local agent connected'
            : readinessTitle}</span
        >
      </div>
      {#if readiness === 'ready'}
        <h2>{isCurrentLocalAgent ? 'This computer' : hostLabel}</h2>
        <p class="detail">
          {isCurrentLocalAgent
            ? readinessMessage
            : `Connected to the agent on ${hostLabel}. Your servers and their files live on that computer.`}
        </p>
        <dl class="agent-facts">
          <div>
            <dt>Background service</dt>
            <dd>{isLocalDesktopHost ? serviceState : `Managed on ${hostLabel}`}</dd>
          </div>
          <div>
            <dt>Agent address</dt>
            <dd>{hostBaseUrl}</dd>
          </div>
          {#if selectedServer}<div>
              <dt>Selected server</dt>
              <dd>
                {selectedServer.name} · {selectedServer.serverType === 'bedrock'
                  ? 'Bedrock'
                  : selectedServer.javaFlavor || 'Java'}
              </dd>
            </div>{/if}
        </dl>
        {#if selectedServer && onOpenServer}
          <Button
            variant="primary"
            disabled={!!openingServerId}
            onclick={() => void openServer(selectedServer.id)}>Open server →</Button
          >
        {/if}
      {:else}
        <h2>{isCurrentLocalAgent ? 'Where will you host?' : hostLabel}</h2>
        <p class="detail">
          {isCurrentLocalAgent
            ? 'MSC needs an agent on the computer that runs your servers. Choose the setup that fits yours.'
            : `This app is not connected to the agent on ${hostLabel}. Its service is managed on that computer.`}
        </p>
        {#if readiness === 'incompatible' || readiness === 'unavailable'}
          <p class="recovery" role="status">
            {isCurrentLocalAgent
              ? readinessMessage
              : `${readinessTitle}. Check the agent on ${hostLabel}, or edit its saved connection below.`}
          </p>
        {/if}
      {/if}

      <section class="setup-choice" aria-label="Local agent">
        <h3>On this computer</h3>
        {#if isLocalDesktopHost}
          {#if status?.state === 'not-installed' || (!status && readiness === 'missing')}
            <p class="detail">
              MSC installs the local background agent for you. Your Minecraft servers and their
              files will live here.
            </p>
            <div class="actions">
              <Button variant="start" disabled={busy} onclick={() => void manage('install')}
                >{busy ? 'Installing…' : 'Install local agent →'}</Button
              >
            </div>
            <p class="small-detail">Local agent not installed.</p>
          {:else}
            <p class="detail">
              The local agent manages Minecraft on this computer. Starting it connects this app;
              stopping it ends the connection.
            </p>
            <div class="actions">
              <Button
                variant={status?.state === 'running' ? 'secondary' : 'start'}
                disabled={busy || !status || status.state === 'unavailable'}
                onclick={() => void manage(status?.state === 'running' ? 'stop' : 'start')}
              >
                {busy ? 'Working…' : status?.state === 'running' ? 'Stop agent' : 'Start agent'}
              </Button>
              <Button disabled={busy || !status} onclick={() => void manage('repair')}
                >Repair agent</Button
              >
            </div>
            <p class="small-detail">{status?.detail ?? 'Checking the local agent service…'}</p>
          {/if}
          {#if status && status.state !== 'not-installed'}
            <details class="service-details">
              <summary>Service details &amp; pairing another desktop</summary>
              <div class="expanded">
                <p class="small-detail">
                  {status.serviceName}{#if status.pid}
                    · Service process: {status.pid}{/if}
                </p>
                <p class="detail">
                  Run this on this computer, then enter the one-use code in the other desktop app.
                </p>
                <div class="command-row">
                  <code>{pairingCommand}</code><Button
                    size="sm"
                    onclick={() => void copyCommand(pairingCommand)}
                    >{copiedCommand === pairingCommand ? 'Copied' : 'Copy'}</Button
                  >
                </div>
              </div>
            </details>
          {/if}
        {:else}
          <p class="detail">
            Use the local agent when your Minecraft servers should run on this computer.
          </p>
          <div class="actions">
            <Button disabled={!onSwitchHost} onclick={() => void connectSavedHost(LOCAL_HOST_ID)}
              >Use local agent</Button
            >
          </div>
        {/if}
      </section>

      {#if isDesktopShell}
        <section class="setup-choice" aria-label="Remote agents">
          <h3>On another computer</h3>
          <p class="detail">
            Connect to an agent on your server computer. You can manage it from this app while
            Minecraft runs there.
          </p>
          <div
            class="remote-picker"
            onfocusout={(event) => {
              if (!event.currentTarget.contains(event.relatedTarget as Node | null))
                remoteMenuOpen = false;
            }}
          >
            <button
              bind:this={remoteTrigger}
              type="button"
              class="remote-trigger"
              aria-expanded={remoteMenuOpen}
              aria-controls="remote-agent-options"
              onclick={() => (remoteMenuOpen = !remoteMenuOpen)}
              >Connect remote agent <span aria-hidden="true">▾</span></button
            >
            {#if remoteMenuOpen}
              <div id="remote-agent-options" class="remote-options">
                <button type="button" onclick={() => selectRemoteView('new')}
                  >Connect new host</button
                >
                <button type="button" onclick={() => selectRemoteView('saved')}
                  >View saved hosts{#if savedHosts.length}
                    ({savedHosts.length}){/if}</button
                >
              </div>
            {/if}
          </div>
        </section>
      {/if}
      <p class="service-note">
        The agent runs independently of this window. Closing MSC does not stop the agent or a
        running Minecraft server.
      </p>
    </section>

    <section class="how-msc-works" aria-label="How MSC works">
      <h2>One app. An agent where you play.</h2>
      <ol class="architecture">
        <li>
          <h3>You use the control panel</h3>
          <p>This desktop app, or the CLI, is where you change settings and manage your servers.</p>
        </li>
        <li>
          <h3>The agent does the work</h3>
          <p>
            It starts Minecraft, manages files and keeps logs on the computer hosting your servers.
          </p>
        </li>
        <li>
          <h3>Minecraft runs on that computer</h3>
          <p>
            The agent can be here, or on another computer. Your servers and worlds stay with their
            agent.
          </p>
        </li>
      </ol>
      <div class="connection-explanation">
        <h3>A connection is control, not the server itself.</h3>
        <p>
          Closing the app leaves the agent running. Pairing gives this app permission to manage it.
        </p>
      </div>
      {#if !isCurrentLocalAgent && onPairAgain}
        <details class="service-details">
          <summary>Replace this app’s pairing</summary>
          <div class="expanded">
            <p class="detail">
              To replace this client’s credential, run <code>{pairingCommand}</code> on {hostLabel},
              then paste the new one-use code here.
            </p>
            <div class="pairing-row">
              <Field type="password" bind:value={pairingCode} placeholder="pair_…" /><Button
                disabled={pairingBusy || !pairingCode.trim()}
                onclick={() => void pairAgain()}>Pair again</Button
              >
            </div>
          </div>
        </details>
      {/if}
    </section>
  </div>

  {#if remoteView === 'new'}
    <section class="remote-content" aria-label="Connect remote host">
      <div class="section-heading">
        <h2>{editingHost ? `Edit ${editingHost.displayName}` : 'Connect your server computer'}</h2>
        <Button size="sm" onclick={closeRemoteView}>Cancel</Button>
      </div>
      <p class="detail">An MSC agent must be installed on that computer. Tailscale is optional.</p>
      {#key editingHost?.id ?? 'new'}<RemoteConnectionWizard
          hosts={savedHosts}
          initialHost={editingHost}
          onConnect={connectRemoteHost}
        />{/key}
      <details class="service-details">
        <summary>Remote service commands</summary>
        <div class="expanded">
          <p class="detail">
            Start, stop, or repair a remote agent on its own computer. Find the installed service
            name before using these commands.
          </p>
          {#each linuxServiceNotes as item (item.command)}
            <div class="extra-note">
              <h3>{item.label}</h3>
              <div class="command-row">
                <code>{item.command}</code><Button
                  size="sm"
                  onclick={() => void copyCommand(item.command)}
                  >{copiedCommand === item.command ? 'Copied' : 'Copy'}</Button
                >
              </div>
              {#if item.note}<p class="small-detail">{item.note}</p>{/if}
            </div>
          {/each}
        </div>
      </details>
    </section>
  {:else if remoteView === 'saved'}
    <section class="remote-content" aria-label="Saved remote hosts">
      <div class="section-heading">
        <h2>Saved remote hosts</h2>
        <Button size="sm" onclick={closeRemoteView}>Close</Button>
      </div>
      <p class="detail">
        Select a saved computer to connect to its agent. Credentials stay in secure storage.
      </p>
      {#each savedHosts as savedHost (savedHost.id)}
        <div class="saved-host-row">
          <div class="saved-host-info">
            <h3>{savedHost.displayName}</h3>
            <p class="small-detail">
              {savedHostStatus(savedHost)} · {hostAddressSummary(savedHost)} · {hostSummaries.get(
                savedHost.id,
              )?.serverCount ?? 0} servers known
            </p>
            {#if savedHost.lanAddresses.length && savedHost.tailscaleAddresses.length}
              <div class="saved-host-route">
                <span>Direct route</span><SegmentedControl
                  options={[
                    { value: 'lan', label: 'LAN' },
                    { value: 'tailscale', label: 'Tailscale' },
                  ]}
                  value={savedHost.preferredRouteOrder[0] ?? 'lan'}
                  onchange={(value) => void onSelectRoute?.(savedHost.id, value as HostRoute)}
                />
              </div>
            {/if}
          </div>
          <div class="actions">
            {#if savedHost.id === activeHostId && readiness === 'ready'}<span class="current-host"
                >Current</span
              >{:else}<Button
                size="sm"
                disabled={!onSwitchHost}
                onclick={() => void connectSavedHost(savedHost.id)}>Connect</Button
              >{/if}
            <Button size="sm" onclick={() => editSavedHost(savedHost.id)}>Edit</Button>
            <Button
              size="sm"
              disabled={removeHostBusy}
              onclick={() => openRemoveHost(savedHost.id, savedHost.displayName)}>Remove</Button
            >
          </div>
        </div>
      {:else}
        <p class="empty-message">
          No saved remote hosts yet. Connect a new host to remember it here.
        </p>
        <Button onclick={() => selectRemoteView('new')}>Connect new host</Button>
      {/each}
      <p class="small-detail">
        An existing SSH tunnel still needs to be open before its saved host can connect.
      </p>
    </section>
  {/if}

  <section class="servers-section">
    <div class="section-heading">
      <h2>{readiness === 'ready' ? 'On this agent' : 'Your servers will appear here'}</h2>
      {#if readiness === 'ready' && onFleet}<Button size="sm" onclick={onFleet}
          >Manage servers</Button
        >{/if}
    </div>
    {#if readiness === 'ready'}
      {#each servers as server (server.id)}
        <div class="server-row">
          <div>
            <h3>{server.name}</h3>
            <p class="small-detail">
              {server.serverType === 'bedrock'
                ? 'Bedrock'
                : server.javaFlavor || 'Java'}{#if server.id === serverId}
                · Selected · {serverRunning ? 'Running' : 'Stopped'}{/if}
            </p>
          </div>
          <Button
            size="sm"
            disabled={!onOpenServer || !!openingServerId}
            onclick={() => void openServer(server.id)}>Overview →</Button
          >
        </div>
      {:else}<p class="empty-message">
          No servers on this agent yet. Manage servers to create or import one.
        </p>{/each}
    {:else}<p class="detail">
        Connect an agent to see the servers it manages. No server information is available yet.
      </p>{/if}
  </section>
  {#if errorMessage}<p class="error" role="alert">
      Could not complete the agent action: {errorMessage}
    </p>{/if}
</div>

<ConfirmDialog
  open={removeHostOpen}
  context={`Host: ${removeHostLabel}`}
  title="Remove this paired host?"
  message="This removes the saved connection from this desktop and forgets its credential. Nothing on the other computer or its Minecraft servers will be deleted."
  confirmLabel={removeHostBusy ? 'Removing…' : 'Remove host'}
  onConfirm={() => void removeConfirmedHost()}
  onClose={removeHostBusy ? undefined : () => (removeHostOpen = false)}
/>

<style>
  .agent-home {
    display: grid;
    gap: 28px;
    max-width: 1160px;
    width: 100%;
    margin: 0 auto;
  }
  h1,
  h2,
  h3,
  p {
    margin: 0;
  }
  h1 {
    font-size: 30px;
    font-weight: 600;
    letter-spacing: -0.8px;
    margin: 3px 0 6px;
  }
  h2 {
    font-size: 19px;
    font-weight: 500;
    letter-spacing: -0.3px;
  }
  h3 {
    font-size: 15px;
    font-weight: 500;
  }
  .home-heading > p:last-child,
  .detail,
  .small-detail {
    color: var(--msc2-text-secondary);
  }
  .home-heading > p:last-child,
  .detail {
    font-size: 13px;
    line-height: 1.5;
  }
  .detail {
    margin-top: 6px;
  }
  .home-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(0, 1fr);
    gap: 30px;
    align-items: start;
  }
  .agent-panel {
    min-width: 0;
    padding: 23px;
    border-radius: 8px;
    background: var(--msc2-tier-content);
  }
  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .connection-state {
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .agent-panel > h2 {
    font-size: 24px;
    margin: 12px 0 4px;
  }
  .agent-facts {
    display: grid;
    gap: 9px;
    margin: 20px 0;
    font-size: 12px;
  }
  .agent-facts > div {
    display: flex;
    justify-content: space-between;
    gap: 15px;
  }
  dt {
    color: var(--msc2-text-secondary);
  }
  dd {
    margin: 0;
    text-align: right;
    overflow-wrap: anywhere;
  }
  .setup-choice {
    padding-top: 21px;
    margin-top: 20px;
    border-top: 1px solid var(--msc2-hairline);
  }
  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 14px;
  }
  .small-detail {
    font-size: 12px;
    margin-top: 9px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .service-note {
    margin-top: 18px;
    padding-top: 15px;
    border-top: 1px solid var(--msc2-hairline);
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-secondary);
  }
  .how-msc-works {
    min-width: 0;
    padding-top: 5px;
  }
  .architecture {
    list-style: none;
    counter-reset: architecture;
    margin: 21px 0 0;
    padding: 0;
    display: grid;
    gap: 19px;
  }
  .architecture li {
    position: relative;
    padding-left: 41px;
    counter-increment: architecture;
  }
  .architecture li::before {
    content: '0' counter(architecture);
    position: absolute;
    left: 0;
    top: 2px;
    font: 12px/1.6 var(--msc2-font-mono, monospace);
    color: var(--msc2-text-tertiary);
  }
  .architecture p,
  .connection-explanation p {
    font-size: 12px;
    line-height: 1.5;
    margin-top: 3px;
    color: var(--msc2-text-secondary);
  }
  .connection-explanation {
    border-top: 1px solid var(--msc2-hairline);
    padding-top: 14px;
    margin-top: 20px;
  }
  .connection-explanation h3 {
    font-size: 12px;
  }
  .remote-picker {
    margin-top: 14px;
  }
  .remote-trigger,
  .remote-options button {
    font: inherit;
    font-size: 13px;
    color: var(--msc2-text-primary);
    cursor: pointer;
  }
  .remote-trigger {
    background: transparent;
    border: 1px solid var(--msc2-hairline);
    border-radius: 8px;
    padding: 8px 16px;
  }
  .remote-trigger span {
    margin-left: 6px;
  }
  .remote-options {
    display: grid;
    gap: 2px;
    margin-top: 7px;
    padding: 5px;
    border-radius: 5px;
    background: var(--msc2-tier-chrome);
    max-width: 240px;
  }
  .remote-options button {
    text-align: left;
    padding: 9px 10px;
    background: transparent;
    border: 0;
    border-radius: 4px;
  }
  .remote-trigger:hover,
  .remote-options button:hover {
    background: var(--msc2-neutral-elevated);
  }
  .remote-content {
    border-top: 1px solid var(--msc2-hairline);
    padding-top: 20px;
    min-width: 0;
  }
  .remote-content > :global(.wizard) {
    margin-top: 20px;
  }
  .service-details {
    margin-top: 18px;
  }
  summary {
    font-size: 12px;
    color: var(--msc2-text-secondary);
    cursor: pointer;
  }
  .expanded {
    padding-top: 14px;
  }
  .command-row,
  .pairing-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
  }
  code {
    font: 12px/1.5 var(--msc2-font-mono, monospace);
    overflow-wrap: anywhere;
  }
  .command-row code {
    padding: 12px;
    background: var(--msc2-tier-terminal);
  }
  .extra-note {
    margin-top: 16px;
  }
  .extra-note h3 {
    font-size: 12px;
  }
  .saved-host-row,
  .server-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 17px 0;
    border-bottom: 1px solid var(--msc2-hairline);
  }
  .saved-host-info {
    min-width: 0;
  }
  .saved-host-row .actions {
    margin-top: 0;
    flex-shrink: 0;
  }
  .saved-host-route {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .current-host {
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .servers-section {
    border-top: 1px solid var(--msc2-hairline);
    padding-top: 18px;
  }
  .empty-message {
    margin: 14px 0;
    font-size: 13px;
    color: var(--msc2-text-secondary);
  }
  .error,
  .recovery {
    color: var(--msc2-status-error);
    font-size: 13px;
    line-height: 1.5;
  }
  .recovery {
    margin-top: 12px;
  }
  @media (max-width: 760px) {
    .home-grid {
      grid-template-columns: 1fr;
      gap: 25px;
    }
    .agent-panel {
      padding: 20px;
    }
    .saved-host-row {
      align-items: flex-start;
      flex-direction: column;
    }
    .saved-host-route {
      flex-wrap: wrap;
    }
    .agent-facts > div {
      flex-wrap: wrap;
    }
  }
</style>
