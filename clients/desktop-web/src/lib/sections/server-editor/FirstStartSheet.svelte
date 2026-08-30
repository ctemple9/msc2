<script lang="ts">
  // First-start is an agent-owned two-pass operation. This sheet only drives
  // the existing routes and renders their truth; credentials stay inside
  // PlayitSetupSheet and never enter this coordinator's state.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import PlayitSetupSheet from './PlayitSetupSheet.svelte';
  import { errorMessage, mutate } from '../shared/types';
  import type { Schema, ScreenApi } from '../shared/types';
  import { pollOperation, serverEditorPaths } from './model';
  import { fleetMutationPaths } from '../fleet/model';

  export let api: ScreenApi | undefined = undefined;
  export let serverName: string;
  export let serverType: 'java' | 'bedrock';
  export let localPort: number;
  export let localBedrockPort: number | undefined = undefined;
  export let playitEnabled = false;
  export let broadcastEnabled = false;
  export let onClose: () => void;
  export let onComplete: () => void = () => {};

  type Phase =
    | 'eula'
    | 'starting-pass-one'
    | 'transport-setup'
    | 'starting-pass-two'
    | 'waiting'
    | 'stopping'
    | 'complete'
    | 'failed';
  type TransportState = 'waiting' | 'ready' | 'skipped' | 'failed' | 'not-applicable';
  type TransportKey = 'playit' | 'broadcast';

  let phase: Phase = serverType === 'java' ? 'eula' : 'eula';
  let operationId = '';
  let statusLine = '';
  let error = '';
  let showPlayitSetup = false;
  let playitSetupVoiceOnly = false;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let broadcast: Schema['BroadcastStatusDTO'] | undefined;
  let broadcastAuth: Schema['BroadcastAuthPromptDTO'] | undefined;
  let activeServerId = '';
  let playitChoiceRequired = false;
  let broadcastChoiceRequired = broadcastEnabled;
  let transport: Record<TransportKey, TransportState> = {
    playit: playitEnabled ? 'waiting' : 'not-applicable',
    broadcast: broadcastEnabled ? 'waiting' : 'not-applicable',
  };
  let passTwoStartedAt = 0;
  let stopRequested = false;

  $: busy =
    phase === 'starting-pass-one' ||
    phase === 'starting-pass-two' ||
    phase === 'waiting' ||
    phase === 'stopping';
  $: title = phase === 'complete' ? 'First Start' : `Start ${serverName || 'server'}`;
  $: allTransportsResolved = Object.values(transport).every(
    (state) => state !== 'waiting',
  );
  $: serverReady = /ready/i.test(statusLine);

  function setTransport(key: TransportKey, state: TransportState): void {
    transport = { ...transport, [key]: state };
  }

  function transportLabel(state: TransportState): string {
    return {
      waiting: 'Waiting',
      ready: 'Ready',
      skipped: 'Skipped',
      failed: 'Timed out',
      'not-applicable': 'Not enabled',
    }[state];
  }

  function transportTone(state: TransportState): 'ok' | 'warn' | 'error' {
    if (state === 'ready') return 'ok';
    if (state === 'waiting') return 'warn';
    if (state === 'failed') return 'error';
    return 'warn';
  }

  async function fetchActiveServer(): Promise<void> {
    const status = await api?.get<Schema['RemoteAPIStatus']>(serverEditorPaths.status);
    activeServerId = status?.activeServerId ?? '';
    if (!activeServerId) throw new Error('The new server is not the active server yet.');
  }

  async function acceptEula(): Promise<void> {
    if (!api || busy) return;
    error = '';
    statusLine = 'Preparing the first start…';
    try {
      await fetchActiveServer();
      if (serverType === 'java') {
        await mutate<Schema['ServerEULAResultDTO']>(api, fleetMutationPaths.eula, {
          serverId: activeServerId,
        });
      }
      await startPassOne();
    } catch (caught) {
      error = errorMessage(caught);
      phase = 'failed';
    }
  }

  async function startPassOne(): Promise<void> {
    if (!api) return;
    phase = 'starting-pass-one';
    statusLine = 'Starting the server to create its configuration and world files…';
    const accepted = await mutate<Schema['SimpleResult']>(api, fleetMutationPaths.start);
    operationId = accepted.operationId ?? '';
    if (!operationId) throw new Error('The agent did not return a first-start operation.');
    const operation = await pollOperation(api, operationId, (tick) => {
      statusLine = tick.statusLine ?? statusLine;
    });
    if (operation?.state !== 'succeeded') {
      throw new Error(operation?.error?.message ?? 'The first server start did not complete.');
    }
    if (operation.result && typeof operation.result === 'object' && 'firstStartComplete' in operation.result) {
      await finishComplete();
      return;
    }
    await prepareTransportSetup();
  }

  async function prepareTransportSetup(): Promise<void> {
    phase = 'transport-setup';
    playitChoiceRequired = false;
    broadcastChoiceRequired = broadcastEnabled;
    statusLine = 'The first run is ready. Choose which connections to finish setting up.';
    if (broadcastEnabled) {
      const jar = await api?.get<Schema['BroadcastJarStatusDTO']>(serverEditorPaths.broadcastJarStatus);
      if (!jar?.installed) {
        statusLine = 'Downloading the Xbox Broadcast helper…';
        const downloaded = await mutate<Schema['BroadcastJarDownloadResultDTO']>(
          api,
          serverEditorPaths.broadcastDownloadJar,
        );
        if (downloaded.operationId) {
          const operation = await pollOperation(api, downloaded.operationId);
          if (operation?.state !== 'succeeded') {
            setTransport('broadcast', 'failed');
            statusLine = 'Xbox Broadcast helper download failed.';
          }
        }
      }
    }
    if (playitEnabled) {
      playit = await api?.get<Schema['PlayitStatusResponseDTO']>(serverEditorPaths.playit);
      playitSetupVoiceOnly = Boolean(
        playit?.hasSecretKey && playit.voiceChatEnabled && !playit.voiceAddress,
      );
      if (!playit?.hasSecretKey || playitSetupVoiceOnly) {
        playitChoiceRequired = true;
        showPlayitSetup = true;
      }
    }
    maybeBeginPassTwo();
  }

  function playitSetupComplete(): void {
    showPlayitSetup = false;
    playitChoiceRequired = false;
    setTransport('playit', 'waiting');
    void refreshPlayit();
    maybeBeginPassTwo();
  }

  function skip(key: TransportKey): void {
    if (busy || transport[key] === 'not-applicable') return;
    setTransport(key, 'skipped');
    if (key === 'playit') showPlayitSetup = false;
    if (key === 'playit') playitChoiceRequired = false;
    if (key === 'broadcast') broadcastChoiceRequired = false;
    maybeBeginPassTwo();
  }

  function continueTransportSetup(): void {
    if (phase !== 'transport-setup') return;
    broadcastChoiceRequired = false;
    maybeBeginPassTwo();
  }

  function maybeBeginPassTwo(): void {
    if (phase !== 'transport-setup' || showPlayitSetup || playitChoiceRequired || broadcastChoiceRequired)
      return;
    void startPassTwo();
  }

  async function startPassTwo(): Promise<void> {
    if (!api) return;
    phase = 'starting-pass-two';
    statusLine = 'Starting the server with the selected connections…';
    passTwoStartedAt = Date.now();
    stopRequested = false;
    try {
      if (transport.playit === 'waiting') {
        const result = await mutate<Schema['PlayitActionResultDTO']>(api, serverEditorPaths.playitStart);
        if (result.operationId) void pollOperation(api, result.operationId);
      }
      if (transport.broadcast === 'waiting') {
        const result = await mutate<Schema['BroadcastSimpleResultDTO']>(
          api,
          serverEditorPaths.broadcastStart,
        );
        if (result.operationId) {
          void monitorBroadcastOperation(result.operationId);
        }
      }
      const accepted = await mutate<Schema['SimpleResult']>(api, fleetMutationPaths.start);
      operationId = accepted.operationId ?? '';
      if (!operationId) throw new Error('The agent did not return the second-pass operation.');
      phase = 'waiting';
      void monitorPassTwo();
    } catch (caught) {
      error = errorMessage(caught);
      phase = 'failed';
    }
  }

  async function refreshPlayit(): Promise<void> {
    if (!api || !playitEnabled) return;
    try {
      playit = await api.get<Schema['PlayitStatusResponseDTO']>(serverEditorPaths.playit);
      const address = serverType === 'bedrock' ? playit.bedrockAddress : playit.javaAddress;
      if (
        transport.playit === 'waiting' &&
        ((playit.isRunning && address) || /timed out|failed/i.test(playit.note ?? ''))
      ) {
        setTransport('playit', address ? 'ready' : 'failed');
      }
    } catch (caught) {
      statusLine = errorMessage(caught);
    }
  }

  async function refreshBroadcast(): Promise<void> {
    if (!api || !broadcastEnabled) return;
    try {
      broadcast = await api.get<Schema['BroadcastStatusDTO']>('/v1/broadcast/status');
      broadcastAuth = await api.get<Schema['BroadcastAuthPromptDTO']>('/v1/broadcast/auth-prompt');
    } catch (caught) {
      statusLine = errorMessage(caught);
    }
  }

  async function monitorBroadcastOperation(id: string): Promise<void> {
    if (!api) return;
    const operation = await pollOperation(api, id);
    if (operation?.state === 'succeeded') {
      setTransport('broadcast', 'ready');
    } else if (operation?.state === 'failed' || operation?.state === 'cancelled') {
      setTransport('broadcast', 'failed');
    }
  }

  async function monitorPassTwo(): Promise<void> {
    if (!api) return;
    for (;;) {
      const operation = await api.get<Schema['OperationDTO']>(`/v1/operations/${encodeURIComponent(operationId)}`);
      statusLine = operation.statusLine ?? statusLine;
      await Promise.all([refreshPlayit(), refreshBroadcast()]);
      if (operation.state === 'failed' || operation.state === 'cancelled') {
        error = operation.error?.message ?? 'The second first-start pass did not complete.';
        phase = 'failed';
        return;
      }
      if (operation.state === 'succeeded') {
        await finishComplete();
        return;
      }
      if (!stopRequested && serverReady && allTransportsResolved) {
        await requestStop();
      }
      if (!stopRequested && Date.now() - passTwoStartedAt >= 600_000) {
        setTransport('playit', transport.playit === 'waiting' ? 'failed' : transport.playit);
        setTransport('broadcast', transport.broadcast === 'waiting' ? 'failed' : transport.broadcast);
        statusLine = 'The first-start safety limit was reached; stopping the server.';
        await requestStop();
      }
      await new Promise((resolve) => setTimeout(resolve, 900));
    }
  }

  async function requestStop(): Promise<void> {
    if (!api || stopRequested) return;
    stopRequested = true;
    phase = 'stopping';
    statusLine = 'Connections are recorded. Stopping the server…';
    await mutate<Schema['SimpleResult']>(api, fleetMutationPaths.stop);
  }

  async function finishComplete(): Promise<void> {
    if (api && playitEnabled) await refreshPlayit();
    phase = 'complete';
    statusLine = 'First-start setup is complete.';
    onComplete();
  }
</script>

<Sheet {title} size="md" onClose={busy ? undefined : onClose}>
  {#if phase === 'eula'}
    <div class="stack">
      <div>
        <p class="msc2-type-overline">One-time setup</p>
        <h2>Prepare {serverName || 'your server'}</h2>
        <p class="copy">
          MSC will start the server once to create its real configuration and world files, then
          stop it while you choose optional connections.
        </p>
      </div>
      {#if serverType === 'java'}
        <p class="notice">
          Starting confirms that you accept Mojang's Minecraft server EULA. You can review it in
          Server Settings afterward.
        </p>
      {/if}
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <div class="actions">
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={onClose}>Later</Button>
        <Button variant="primary" onclick={() => void acceptEula()}>Start first run</Button>
      </div>
    </div>
  {:else if phase === 'starting-pass-one'}
    <div class="stack">
      <p class="msc2-type-overline">Pass 1 of 2</p>
      <h2>Creating the server files</h2>
      <p class="copy">The agent is waiting for the server-ready signal before it stops the first run.</p>
      <p class="status-line" role="status" aria-live="polite">{statusLine}</p>
    </div>
  {:else if phase === 'transport-setup' || phase === 'starting-pass-two' || phase === 'waiting' || phase === 'stopping'}
    <div class="stack">
      <div>
        <p class="msc2-type-overline">{phase === 'transport-setup' ? 'Pass 1 complete' : 'Pass 2 of 2'}</p>
        <h2>{phase === 'transport-setup' ? 'Choose connections' : 'Checking connections'}</h2>
        <p class="copy">
          MSC waits for the server and every enabled connection, then stops the server. Setup time
          spent entering Playit credentials is not counted against the technical wait.
        </p>
      </div>

      <div class="transport-list" aria-label="First-start connections">
        <div class="transport-row">
          <div><strong>Playit</strong><span>Public connection for {serverType === 'bedrock' ? 'Bedrock' : 'Java'} players</span></div>
          <StatusDot tone={transportTone(transport.playit)} label={transportLabel(transport.playit)} />
          {#if phase === 'transport-setup' && transport.playit === 'waiting'}
            <Button variant="secondary" size="sm" onclick={() => (showPlayitSetup = true)}>Set up</Button>
            <Button variant="ghost-icon" size="sm" label="Skip Playit" onclick={() => skip('playit')}>×</Button>
          {/if}
        </div>
        <div class="transport-row">
          <div><strong>Xbox Broadcast</strong><span>Xbox discovery helper</span></div>
          <StatusDot tone={transportTone(transport.broadcast)} label={transportLabel(transport.broadcast)} />
          {#if phase === 'transport-setup' && transport.broadcast === 'waiting'}
            <Button variant="secondary" size="sm" onclick={() => skip('broadcast')}>Skip</Button>
          {/if}
        </div>
      </div>

      {#if broadcastAuth?.isPresent}
        <p class="notice" role="status">
          Xbox Broadcast needs sign-in: enter code {broadcastAuth.code ?? 'shown by the helper'} at
          {broadcastAuth.linkURL ?? 'the Microsoft device sign-in page'}.
        </p>
      {/if}
      <p class="status-line" role="status" aria-live="polite">{statusLine}</p>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      {#if phase === 'transport-setup'}
        <div class="actions">
          <span class="action-spacer"></span>
          <Button variant="primary" onclick={continueTransportSetup}>Continue</Button>
        </div>
      {/if}
    </div>
  {:else if phase === 'complete'}
    <div class="stack">
      <div>
        <p class="msc2-type-overline">First Start</p>
        <h2>{serverName || 'Server'} is ready</h2>
        <p class="copy">MSC created the server files and recorded the connections that actually resolved.</p>
      </div>
      <StatusDot tone="ok" label="One-time setup complete" />

      <section class="section">
        <p class="section-label">How people connect</p>
        <div class="connection-list">
          <div><span>{serverType === 'bedrock' ? 'Bedrock — same Wi-Fi' : 'Java — same Wi-Fi'}</span><code>local address:{localPort}</code></div>
          {#if serverType === 'java' && localBedrockPort}
            <div><span>Bedrock — same Wi-Fi</span><code>local address:{localBedrockPort}</code></div>
          {/if}
          {#if playit?.javaAddress && serverType === 'java'}
            <div><span>Java — anywhere (playit.gg)</span><code>{playit.javaAddress}</code></div>
          {/if}
          {#if playit?.bedrockAddress && serverType === 'bedrock'}
            <div><span>Bedrock — anywhere (playit.gg)</span><code>{playit.bedrockAddress}</code></div>
          {/if}
          {#if playit?.voiceAddress && playit.voiceChatEnabled}
            <div><span>Voice — Simple Voice Chat</span><code>{playit.voiceAddress}</code></div>
          {/if}
          {#if broadcastEnabled}
            <div><span>Xbox</span><span class="muted">Use Xbox discovery when Broadcast is Ready.</span></div>
          {/if}
        </div>
      </section>

      <section class="section">
        <p class="section-label">What MSC created</p>
        <p class="copy">The server configuration, first world, enabled helper setup, and any resolved public addresses.</p>
      </section>
      <section class="section">
        <p class="section-label">Next</p>
        <p class="copy">Open Server Settings to review the EULA, ports, world settings, and optional helpers. Future starts are manual.</p>
      </section>
      <div class="actions">
        <Button variant="secondary" onclick={onClose}>Open settings later</Button>
        <span class="action-spacer"></span>
        <Button variant="primary" onclick={onClose}>Done</Button>
      </div>
    </div>
  {:else}
    <div class="stack">
      <p class="msc2-type-overline">First Start</p>
      <h2>Setup needs another try</h2>
      <StatusDot tone="error" label="Not completed" />
      <p class="copy">{error || 'The first run stopped before it finished. Your next Start will try initiation again.'}</p>
      <div class="actions">
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={onClose}>Close</Button>
        <Button variant="primary" onclick={() => { phase = 'eula'; error = ''; }}>Try again</Button>
      </div>
    </div>
  {/if}
</Sheet>

{#if showPlayitSetup}
  <PlayitSetupSheet
    {api}
    {playit}
    context="initiation"
    voiceOnly={playitSetupVoiceOnly}
    onClose={() => (showPlayitSetup = false)}
    onComplete={playitSetupComplete}
  />
{/if}

<style>
  .stack { display: flex; flex-direction: column; gap: 14px; }
  h2 { margin: 0; font-size: 15px; font-weight: 600; color: var(--msc2-text-primary); }
  .copy, .notice, .status-line { margin: 0; font-size: 12px; line-height: 1.55; color: var(--msc2-text-tertiary); }
  .notice { padding: 10px 12px; background: var(--msc2-tier-chrome); border-left: 2px solid var(--msc2-status-warn); }
  .status-line { color: var(--msc2-text-secondary); }
  .error { margin: 0; color: var(--msc2-status-error); font-size: 12px; line-height: 1.5; }
  .actions { display: flex; align-items: center; gap: 8px; padding-top: 10px; border-top: 1px solid var(--msc2-hairline-subtle); }
  .action-spacer { flex: 1; }
  .transport-list, .connection-list { display: flex; flex-direction: column; background: var(--msc2-tier-chrome); border-radius: 10px; overflow: hidden; }
  .transport-row, .connection-list > div { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-top: 1px solid var(--msc2-hairline-subtle); }
  .transport-row:first-child, .connection-list > div:first-child { border-top: none; }
  .transport-row > div:first-child { display: flex; flex: 1; flex-direction: column; gap: 2px; }
  strong, .connection-list span { color: var(--msc2-text-primary); font-size: 12px; font-weight: 500; }
  .transport-row span, .muted { color: var(--msc2-text-tertiary); font-size: 11px; }
  .section { display: flex; flex-direction: column; gap: 6px; }
  .section-label { margin: 0; color: var(--msc2-text-secondary); font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; }
  code { margin-left: auto; color: var(--msc2-text-primary); font-size: 11px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  .muted { margin-left: auto; }
</style>
