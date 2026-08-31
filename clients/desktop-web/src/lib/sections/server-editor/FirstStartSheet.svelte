<script lang="ts">
  // First-start is an agent-owned two-pass operation. This sheet only drives
  // the existing routes and renders their truth; credentials stay inside
  // PlayitSetupSheet and never enter this coordinator's state.
  import { onDestroy, onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import PlayitSetupSheet from './PlayitSetupSheet.svelte';
  import BroadcastAuthSheet from './BroadcastAuthSheet.svelte';
  import { errorMessage, mutate } from '../shared/types';
  import type { Schema, ScreenApi } from '../shared/types';
  import { pollOperation, serverEditorPaths } from './model';
  import { fleetMutationPaths } from '../fleet/model';
  import { livePaths, type ConsoleLine } from '../console/model';

  export let api: ScreenApi | undefined = undefined;
  export let serverName: string;
  export let serverType: 'java' | 'bedrock';
  export let localPort: number;
  export let localBedrockPort: number | undefined = undefined;
  export let playitEnabled = false;
  export let broadcastEnabled = false;
  export let hidden = false;
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

  let phase: Phase = 'eula';
  let operationId = '';
  let statusLine = '';
  let error = '';
  let showPlayitSetup = false;
  let showBroadcastAuth = false;
  let playitSetupVoiceOnly = false;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let broadcast: Schema['BroadcastStatusDTO'] | undefined;
  let broadcastAuth: Schema['BroadcastAuthPromptDTO'] | undefined;
  let activeServerId = '';
  let playitChoiceRequired = false;
  let broadcastChoiceRequired = broadcastEnabled;
  let playitAttempted = !playitEnabled;
  let playitAttemptedThisRound = false;
  let playitSetupSucceeded = false;
  let transport: Record<TransportKey, TransportState> = {
    playit: playitEnabled ? 'waiting' : 'not-applicable',
    broadcast: broadcastEnabled ? 'waiting' : 'not-applicable',
  };
  let passTwoStartedAt = 0;
  let stopRequested = false;
  let eulaAccepted = false;
  let consoleLines: ConsoleLine[] = [];
  let consoleTimer: ReturnType<typeof setInterval> | undefined;

  $: busy =
    phase === 'starting-pass-one' ||
    phase === 'starting-pass-two' ||
    phase === 'waiting' ||
    phase === 'stopping';
  $: title = phase === 'complete' ? 'First Start' : `Initiate ${serverName || 'server'}`;
  $: allTransportsResolved = Object.values(transport).every((state) => state !== 'waiting');
  $: serverReady = /ready/i.test(statusLine);
  $: broadcastUnlocked = !playitEnabled || playitAttempted;

  async function refreshConsole(): Promise<void> {
    if (!api) return;
    try {
      consoleLines = await api.get<ConsoleLine[]>(livePaths.tail);
    } catch {
      // The operation remains authoritative if the agent is briefly
      // unreachable; keep the last visible console tail in place.
    }
  }

  onMount(() => {
    consoleTimer = setInterval(() => void refreshConsole(), 1000);
    void refreshConsole();
  });

  onDestroy(() => {
    if (consoleTimer) clearInterval(consoleTimer);
  });

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
    if (serverType === 'java' && !eulaAccepted) {
      error = 'Accept the Minecraft EULA before continuing.';
      return;
    }
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
    if (
      operation.result &&
      typeof operation.result === 'object' &&
      'firstStartComplete' in operation.result
    ) {
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
      const jar = await api?.get<Schema['BroadcastJarStatusDTO']>(
        serverEditorPaths.broadcastJarStatus,
      );
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
          } else {
            statusLine = 'Xbox Broadcast helper is ready. Continue to start it with your server.';
          }
        } else {
          statusLine = 'Xbox Broadcast helper is ready. Continue to start it with your server.';
        }
      }
    }
    if (playitEnabled) {
      playit = await api?.get<Schema['PlayitStatusResponseDTO']>(serverEditorPaths.playit);
      playitAttempted = Boolean(playit?.hasSecretKey);
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

  function openPlayitSetup(): void {
    playitSetupSucceeded = false;
    showPlayitSetup = true;
    playitChoiceRequired = true;
  }

  function playitAttemptedNow(): void {
    playitAttempted = true;
    playitAttemptedThisRound = true;
    playitSetupSucceeded = false;
  }

  function playitSetupComplete(): void {
    playitAttempted = true;
    playitSetupSucceeded = true;
    showPlayitSetup = false;
    playitChoiceRequired = false;
    setTransport('playit', 'waiting');
    void refreshPlayit();
    maybeBeginPassTwo();
  }

  function closePlayitSetup(): void {
    showPlayitSetup = false;
    if (playitAttemptedThisRound && !playitSetupSucceeded) {
      setTransport('playit', 'failed');
      playitChoiceRequired = false;
      maybeBeginPassTwo();
    } else if (playitAttempted) {
      // Opening an already configured setup sheet is not itself a new
      // attempt; closing it must restore the prior ability to continue.
      playitChoiceRequired = false;
      maybeBeginPassTwo();
    }
    playitAttemptedThisRound = false;
  }

  function skipBroadcast(): void {
    const key: TransportKey = 'broadcast';
    if (busy || transport[key] === 'not-applicable') return;
    setTransport(key, 'skipped');
    broadcastChoiceRequired = false;
    maybeBeginPassTwo();
  }

  function continueTransportSetup(): void {
    if (phase !== 'transport-setup') return;
    if (playitEnabled && !playitAttempted) {
      error = 'Try Playit setup before continuing to Xbox Broadcast.';
      return;
    }
    error = '';
    broadcastChoiceRequired = false;
    maybeBeginPassTwo();
  }

  function maybeBeginPassTwo(): void {
    if (
      phase !== 'transport-setup' ||
      showPlayitSetup ||
      playitChoiceRequired ||
      broadcastChoiceRequired
    )
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
        const result = await mutate<Schema['PlayitActionResultDTO']>(
          api,
          serverEditorPaths.playitStart,
        );
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
      showBroadcastAuth = Boolean(broadcastAuth.isPresent);
    } catch (caught) {
      statusLine = errorMessage(caught);
    }
  }

  function closeBroadcastAuth(): void {
    showBroadcastAuth = false;
    broadcastAuth = undefined;
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
      const operation = await api.get<Schema['OperationDTO']>(
        `/v1/operations/${encodeURIComponent(operationId)}`,
      );
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
        setTransport(
          'broadcast',
          transport.broadcast === 'waiting' ? 'failed' : transport.broadcast,
        );
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
    if (api && broadcastEnabled) await refreshBroadcast();
    phase = 'complete';
    statusLine = 'First-start setup is complete.';
    onComplete();
  }
</script>

<Sheet {title} size="md" visible={!hidden} {onClose}>
  {#if phase === 'eula'}
    <div class="stack">
      <div>
        <p class="msc2-type-overline">One-time setup</p>
        <h2>Prepare {serverName || 'your server'}</h2>
        <p class="copy">
          MSC will start the server once to create its real configuration and world files, then stop
          it while you choose optional connections.
        </p>
      </div>
      {#if serverType === 'java'}
        <p class="notice">
          Accept Mojang's Minecraft server EULA here before MSC creates the server files. You can
          review it in Server Settings afterward.
        </p>
        <label class="eula-check">
          <input type="checkbox" bind:checked={eulaAccepted} />
          <span>I accept the Minecraft server EULA.</span>
        </label>
      {/if}
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <div class="actions">
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={onClose}>Later</Button>
        <Button
          variant="primary"
          disabled={serverType === 'java' && !eulaAccepted}
          onclick={() => void acceptEula()}>Continue</Button
        >
      </div>
    </div>
  {:else if phase === 'starting-pass-one'}
    <div class="stack">
      <p class="msc2-type-overline">Pass 1 of 2</p>
      <h2>Creating the server files</h2>
      <p class="copy">
        The agent is waiting for the server-ready signal before it stops the first run.
      </p>
      <p class="status-line" role="status" aria-live="polite">{statusLine}</p>
    </div>
  {:else if phase === 'transport-setup' || phase === 'starting-pass-two' || phase === 'waiting' || phase === 'stopping'}
    <div class="stack">
      <div>
        <p class="msc2-type-overline">
          {phase === 'transport-setup' ? 'Pass 1 complete' : 'Pass 2 of 2'}
        </p>
        <h2>{phase === 'transport-setup' ? 'Choose connections' : 'Checking connections'}</h2>
        <p class="copy">
          MSC waits for the server and every enabled connection, then stops the server. Setup time
          spent entering Playit credentials is not counted against the technical wait.
        </p>
      </div>

      <div class="transport-list" aria-label="First-start connections">
        <div class="transport-row">
          <div>
            <strong>Playit</strong><span
              >Public connection for {serverType === 'bedrock' ? 'Bedrock' : 'Java'} players</span
            >
          </div>
          <StatusDot
            tone={transportTone(transport.playit)}
            label={transportLabel(transport.playit)}
          />
          {#if phase === 'transport-setup' && (transport.playit === 'waiting' || transport.playit === 'failed')}
            <Button variant="secondary" size="sm" onclick={openPlayitSetup}
              >{transport.playit === 'failed' ? 'Try again' : 'Set up'}</Button
            >
          {/if}
        </div>
        <div class="transport-row">
          <div><strong>Xbox Broadcast</strong><span>Xbox discovery helper</span></div>
          <StatusDot
            tone={transportTone(transport.broadcast)}
            label={transportLabel(transport.broadcast)}
          />
          {#if phase === 'transport-setup' && transport.broadcast === 'waiting'}
            {#if broadcastUnlocked}
              <Button variant="secondary" size="sm" onclick={skipBroadcast}>Skip</Button>
            {:else}
              <span class="locked-transport">Set up Playit first</span>
            {/if}
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
        <p class="copy">
          MSC created the server files and recorded the connections that actually resolved.
        </p>
      </div>
      <StatusDot tone="ok" label="One-time setup complete" />

      <section class="section">
        <p class="section-label">How people connect</p>
        <div class="connection-list">
          <div>
            <span>{serverType === 'bedrock' ? 'Bedrock — same Wi-Fi' : 'Java — same Wi-Fi'}</span
            ><code>local address:{localPort}</code>
          </div>
          {#if serverType === 'java' && localBedrockPort}
            <div>
              <span>Bedrock — same Wi-Fi</span><code>local address:{localBedrockPort}</code>
            </div>
          {/if}
          {#if playit?.javaAddress && serverType === 'java'}
            <div><span>Java — anywhere (playit.gg)</span><code>{playit.javaAddress}</code></div>
          {/if}
          {#if playit?.bedrockAddress && serverType === 'bedrock'}
            <div>
              <span>Bedrock — anywhere (playit.gg)</span><code>{playit.bedrockAddress}</code>
            </div>
          {/if}
          {#if broadcastEnabled}
            <div>
              <span>Xbox Broadcast</span>
              {#if broadcast?.gamertag}
                <span class="connection-note"
                  >Friend name to add: <strong>{broadcast.gamertag}</strong></span
                >
              {:else}
                <span class="connection-note"
                  >Friend name to add: unavailable until Broadcast authenticates.</span
                >
              {/if}
            </div>
          {/if}
        </div>
      </section>

      <section class="section">
        <p class="section-label">What MSC created</p>
        <p class="copy">
          The server configuration, first world, enabled helper setup, and any resolved public
          addresses.
        </p>
      </section>
      <section class="section">
        <p class="section-label">Next</p>
        <p class="copy">
          Open Server Settings to review the EULA, ports, world settings, and optional helpers.
          Future starts are manual.
        </p>
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
      <p class="copy">
        {error || 'The first run stopped before it finished. Choose Initiate to try it again.'}
      </p>
      <div class="actions">
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={onClose}>Close</Button>
        <Button
          variant="primary"
          onclick={() => {
            phase = 'eula';
            eulaAccepted = false;
            error = '';
          }}>Try again</Button
        >
      </div>
    </div>
  {/if}
  {#if phase !== 'eula'}
    <section class="console-panel" aria-label="First-start console">
      <div class="console-header">
        <span>Console</span>
        <span class="console-live">Live</span>
      </div>
      <div class="console-body" aria-live="polite">
        {#if consoleLines.length}
          {#each consoleLines.slice(-80) as line, index (line.ts + '-' + index)}
            <p
              class="console-line"
              class:error={line.level === 'error'}
              class:warn={line.level === 'warn'}
            >
              {line.text}
            </p>
          {/each}
        {:else}
          <p class="console-empty">Waiting for console output…</p>
        {/if}
      </div>
    </section>
  {/if}
</Sheet>

{#if showPlayitSetup}
  <PlayitSetupSheet
    {api}
    {playit}
    context="initiation"
    voiceOnly={playitSetupVoiceOnly}
    visible={!hidden}
    onClose={closePlayitSetup}
    onAttempted={playitAttemptedNow}
    onComplete={playitSetupComplete}
  />
{/if}

{#if showBroadcastAuth && broadcastAuth?.isPresent}
  <BroadcastAuthSheet {api} prompt={broadcastAuth} visible={!hidden} onClose={closeBroadcastAuth} />
{/if}

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .copy,
  .notice,
  .status-line {
    margin: 0;
    font-size: 12px;
    line-height: 1.55;
    color: var(--msc2-text-tertiary);
  }
  .notice {
    padding: 10px 12px;
    background: var(--msc2-tier-chrome);
    border-left: 2px solid var(--msc2-status-warn);
  }
  .eula-check {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }
  .eula-check input {
    margin: 2px 0 0;
    accent-color: var(--msc2-status-ok);
  }
  .status-line {
    color: var(--msc2-text-secondary);
  }
  .error {
    margin: 0;
    color: var(--msc2-status-error);
    font-size: 12px;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .action-spacer {
    flex: 1;
  }
  .transport-list,
  .connection-list {
    display: flex;
    flex-direction: column;
    background: var(--msc2-tier-chrome);
    border-radius: 10px;
    overflow: hidden;
  }
  .transport-row,
  .connection-list > div {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .transport-row:first-child,
  .connection-list > div:first-child {
    border-top: none;
  }
  .transport-row > div:first-child {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
  }
  .locked-transport {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
    font-style: italic;
  }
  strong,
  .connection-list span {
    color: var(--msc2-text-primary);
    font-size: 12px;
    font-weight: 500;
  }
  .transport-row span {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .section-label {
    margin: 0;
    color: var(--msc2-text-secondary);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  code {
    margin-left: auto;
    color: var(--msc2-text-primary);
    font-size: 11px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .connection-note {
    flex: 1;
    color: var(--msc2-text-secondary);
    font-size: 11px;
    line-height: 1.4;
    text-align: right;
  }
  .connection-note strong {
    font-size: inherit;
  }
  .console-panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 14px;
  }
  .console-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--msc2-text-secondary);
    font-size: 11px;
    font-weight: 600;
  }
  .console-live {
    color: var(--msc2-status-ok);
    font-size: 10px;
    font-weight: 500;
  }
  .console-body {
    max-height: 180px;
    min-height: 72px;
    overflow-y: auto;
    padding: 8px 10px;
    background: var(--msc2-tier-terminal);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 8px;
    font-family: var(--msc2-font-mono);
    font-size: 10px;
    line-height: 1.45;
  }
  .console-line {
    margin: 0;
    color: var(--msc2-text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .console-line.error {
    color: var(--msc2-status-error);
  }
  .console-line.warn {
    color: var(--msc2-status-warn);
  }
  .console-empty {
    margin: 0;
    color: var(--msc2-text-tertiary);
  }
</style>
