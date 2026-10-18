<script lang="ts">
  // Ports ServerEditorBroadcastTab.swift (Java) -- Xbox broadcast runtime
  // controls plus the per-server Playit runtime panel. Every route this tab calls
  // (crates/msc-agent/src/routes/networking.rs) resolves a single
  // agent-wide "active server", so -- like GeneralTab's Memory block -- the
  // whole tab is gated on `isActive` rather than risking a mutation landing
  // on the wrong server.
  //
  // Left out, not silently dropped: IP Mode (auto/public/private) and the
  // computed "transfers to host:port" preview have no backing route or field
  // at all (no XboxBroadcastIPMode get/set anywhere in the contract); "Reset
  // Xbox Sign-In" has no dedicated route either -- /v1/broadcast/restart
  // restarts the helper but doesn't clear cached credentials the way MSC 1's
  // reset does, so mapping "Reset Sign-In" to it would be dishonest. Host/
  // Port below are ServerDTO's own real hostAddress/gamePort fields instead
  // of the IP-mode-aware preview.
  import Card from '../../components/base/Card.svelte';
  import { onDestroy, onMount } from 'svelte';
  import Button from '../../components/base/Button.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import PlayitSetupSheet from './PlayitSetupSheet.svelte';
  import BroadcastAuthSheet from './BroadcastAuthSheet.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { pollOperation, serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let server: Schema['ServerDTO'];
  export let isActive = false;
  export let canControl = true;
  export let onServersChanged: () => Promise<void> = async () => {};
  export let onRequestActivate: () => void;

  $: isJava = server.serverType !== 'bedrock';

  let status: Schema['BroadcastStatusDTO'] | undefined;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let serverStatus: Schema['RemoteAPIStatus'] = { running: false };

  let broadcastBusy = false;
  let broadcastSetupBusy = false;
  let broadcastJarBusy = false;
  let broadcastAutostart: Schema['BroadcastAutoStartDTO'] | undefined;
  let broadcastJar: Schema['BroadcastJarStatusDTO'] | undefined;
  let broadcastAuth: Schema['BroadcastAuthPromptDTO'] = { isPresent: false };
  let broadcastAuthKey = '';
  let broadcastAuthHidden = false;
  let showBroadcastAuth = false;
  let broadcastPromptLoadBusy = false;
  let broadcastPromptTimer: ReturnType<typeof setInterval> | undefined;
  let xboxEnabled = server.xboxBroadcastEnabled === true;
  let playitBusy = false;
  let showPlayitSetup = false;

  let notice = '';
  let loaded = false;
  let playitLoadVersion = 0;

  $: broadcastRunning = isJava
    ? (status?.xboxBroadcastRunning ?? false)
    : (status?.bedrockBroadcastRunning ?? false);
  $: if (isActive && !loaded) {
    loaded = true;
    void loadAll();
  }
  $: if (!isActive) {
    loaded = false;
    showBroadcastAuth = false;
    // Invalidate requests started for the former host/server boundary. The
    // shared API client can change targets while an awaited request is out.
    playitLoadVersion += 1;
  }

  onMount(() => {
    broadcastPromptTimer = setInterval(() => {
      if (isActive && xboxEnabled) void refreshBroadcastAuthPrompt();
    }, 1000);
  });

  onDestroy(() => {
    if (broadcastPromptTimer) clearInterval(broadcastPromptTimer);
  });

  async function loadAll(): Promise<void> {
    const loadVersion = ++playitLoadVersion;
    const nextValues = await Promise.all([
      call(api, status, serverEditorPaths.broadcastStatus),
      call(api, playit, serverEditorPaths.playit),
      call(api, serverStatus, serverEditorPaths.status),
      call<Schema['BroadcastAutoStartDTO']>(
        api,
        { enabled: true },
        serverEditorPaths.broadcastAutostart,
      ),
      call<Schema['BroadcastJarStatusDTO']>(
        api,
        { installed: false, downloading: false },
        serverEditorPaths.broadcastJarStatus,
      ),
    ]);
    if (!isActive || loadVersion !== playitLoadVersion) return;
    [status, playit, serverStatus, broadcastAutostart, broadcastJar] = nextValues;
    await refreshBroadcastAuthPrompt();
  }

  async function refreshBroadcastAuthPrompt(): Promise<void> {
    if (broadcastPromptLoadBusy || !api || !isActive || !xboxEnabled) return;
    broadcastPromptLoadBusy = true;
    try {
      const prompt = await api.get<Schema['BroadcastAuthPromptDTO']>(
        serverEditorPaths.broadcastAuthPrompt,
      );
      broadcastAuth = prompt;
      if (!prompt.isPresent) {
        broadcastAuthKey = '';
        broadcastAuthHidden = false;
        showBroadcastAuth = false;
        status = await call(api, status, serverEditorPaths.broadcastStatus);
        return;
      }
      const promptKey = `${prompt.code ?? ''}|${prompt.linkURL ?? ''}`;
      if (promptKey !== broadcastAuthKey) {
        broadcastAuthKey = promptKey;
        broadcastAuthHidden = false;
      }
      if (!broadcastAuthHidden) showBroadcastAuth = true;
    } catch {
      // Keep the last known prompt during a temporary host disconnect.
    } finally {
      broadcastPromptLoadBusy = false;
    }
  }

  async function startBroadcastSignIn(): Promise<void> {
    if (broadcastSetupBusy || !canControl || !isActive) return;
    if (!xboxEnabled) {
      notice = 'Enable Xbox Broadcast for this server before signing in.';
      return;
    }
    broadcastSetupBusy = true;
    notice = '';
    try {
      await refreshBroadcastAuthPrompt();
      if (broadcastAuth.isPresent) return;
      status = await call(api, status, serverEditorPaths.broadcastStatus);
      if (status?.authenticated) {
        notice = status.gamertag
          ? `Xbox Broadcast is signed in as ${status.gamertag}.`
          : 'Xbox Broadcast is signed in.';
        return;
      }
      if (status?.xboxBroadcastRunning || status?.bedrockBroadcastRunning) {
        notice = 'Xbox Broadcast is already running. Waiting for its sign-in code.';
        return;
      }
      await mutate<Schema['BroadcastSimpleResultDTO']>(api, serverEditorPaths.broadcastStart);
      notice = 'Xbox Broadcast is starting. Its sign-in code will appear here.';
      await refreshBroadcastAuthPrompt();
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      broadcastSetupBusy = false;
    }
  }

  async function updateBroadcastAutostart(enabled: boolean): Promise<void> {
    try {
      broadcastAutostart = await mutate<Schema['BroadcastAutoStartDTO']>(
        api,
        serverEditorPaths.broadcastAutostart,
        { enabled },
      );
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function downloadBroadcastJar(): Promise<void> {
    if (broadcastJarBusy) return;
    broadcastJarBusy = true;
    notice = '';
    try {
      const result = await mutate<Schema['BroadcastJarDownloadResultDTO']>(
        api,
        serverEditorPaths.broadcastDownloadJar,
      );
      if (result.operationId) await pollOperation(api, result.operationId);
      notice = result.message;
      broadcastJar = await call(api, broadcastJar, serverEditorPaths.broadcastJarStatus);
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      broadcastJarBusy = false;
    }
  }

  async function toggleBroadcast(): Promise<void> {
    if (broadcastBusy) return;
    broadcastBusy = true;
    try {
      const path = broadcastRunning
        ? serverEditorPaths.broadcastStop
        : serverEditorPaths.broadcastStart;
      const result = await mutate<Schema['BroadcastSimpleResultDTO']>(api, path);
      if (result.operationId) await pollOperation(api, result.operationId);
      status = await call(api, status, serverEditorPaths.broadcastStatus);
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      broadcastBusy = false;
    }
  }

  async function setXboxEnabled(enabled: boolean): Promise<void> {
    if (!canControl || broadcastBusy) return;
    broadcastBusy = true;
    try {
      await mutate<{ enabled: boolean }>(api, serverEditorPaths.xboxBroadcast, {
        serverId: server.id,
        enabled,
      });
      xboxEnabled = enabled;
      await onServersChanged();
      if (enabled && !broadcastRunning && serverStatus.running) {
        const result = await mutate<Schema['BroadcastSimpleResultDTO']>(
          api,
          serverEditorPaths.broadcastStart,
        );
        if (result.operationId) await pollOperation(api, result.operationId);
        status = await call(api, status, serverEditorPaths.broadcastStatus);
      }
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      broadcastBusy = false;
    }
  }

  async function togglePlayit(): Promise<void> {
    if (!isActive || playitBusy || !playit) return;
    playitBusy = true;
    try {
      const path = playit.isRunning ? serverEditorPaths.playitStop : serverEditorPaths.playitStart;
      await mutate(api, path);
      playit = { ...playit, isRunning: !playit.isRunning };
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      playitBusy = false;
    }
  }

  async function refreshPlayit(): Promise<void> {
    playit = await call(api, playit, serverEditorPaths.playit);
  }

  async function setPlayitEnabled(enabled: boolean): Promise<void> {
    if (!canControl || playitBusy || !playit) return;
    playitBusy = true;
    try {
      await mutate<{ enabled: boolean }>(api, serverEditorPaths.playitEnabled, {
        serverId: server.id,
        enabled,
      });
      playit = { ...playit, playitEnabled: enabled };
      await onServersChanged();
      await refreshPlayit();
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      playitBusy = false;
    }
  }
</script>

<div class="tab">
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  {#if !isActive}
    <Card>
      <div class="notice-row">
        <div class="notice-text">
          <span class="name">Set as active to configure broadcast</span>
          <p class="hint">
            Xbox Broadcast and Playit runtime controls are only editable for the currently active
            server.
          </p>
        </div>
        <Button variant="secondary" size="sm" onclick={onRequestActivate}>Set as Active</Button>
      </div>
    </Card>
  {:else}
    <section class="zone">
      <p class="msc2-type-overline">Xbox Broadcast</p>
      <Card padding="0">
        <div class="row">
          <StatusDot
            tone={broadcastRunning ? 'ok' : 'warn'}
            label={broadcastRunning ? 'Running' : 'Stopped'}
          />
          <Button
            variant="secondary"
            size="sm"
            disabled={broadcastBusy || !canControl}
            onclick={toggleBroadcast}>{broadcastRunning ? 'Stop' : 'Start'}</Button
          >
        </div>
        <div class="row bordered">
          <div class="toggle-info">
            <span class="name">Xbox Broadcast settings</span>
            <span class="setup-state">Allow this server to use Xbox Broadcast.</span>
          </div>
          <Toggle
            checked={xboxEnabled}
            label="Enable Xbox Broadcast"
            onchange={(enabled) => void setXboxEnabled(enabled)}
          />
        </div>
        <div class="row bordered">
          <div class="row-text">
            <span class="name">Start Xbox Broadcast automatically</span>
            <span class="setup-state">Applies to eligible servers on this computer.</span>
          </div>
          <Toggle
            checked={broadcastAutostart?.enabled ?? false}
            label="Start Xbox Broadcast automatically"
            disabled={!canControl}
            onchange={updateBroadcastAutostart}
          />
        </div>
        <div class="row bordered">
          <div class="row-text">
            <span class="name">Microsoft sign-in</span>
            <span class="setup-state">
              {status?.authenticated
                ? status.gamertag
                  ? `Signed in as ${status.gamertag}`
                  : 'Signed in'
                : 'Sign in to Xbox Broadcast for this server.'}
            </span>
          </div>
          <Button
            variant="secondary"
            size="sm"
            disabled={broadcastSetupBusy || !canControl}
            onclick={() => void startBroadcastSignIn()}
          >
            {broadcastSetupBusy ? 'Starting…' : status?.authenticated ? 'Check sign-in' : 'Set up…'}
          </Button>
        </div>
        <div class="row bordered">
          <div class="row-text">
            <span class="name">Helper JAR</span>
            <span class="setup-state">
              {broadcastJar?.installed ? (broadcastJar.filename ?? 'Installed') : 'Not installed'}
            </span>
          </div>
          <Button
            variant="secondary"
            size="sm"
            disabled={broadcastJarBusy || !canControl}
            onclick={() => void downloadBroadcastJar()}
          >
            {broadcastJarBusy ? 'Downloading…' : broadcastJar?.installed ? 'Update…' : 'Download…'}
          </Button>
        </div>
      </Card>
    </section>

    <section class="zone">
      <p class="msc2-type-overline">Playit</p>
      <Card padding="0">
        <div class="row">
          <div class="toggle-info">
            <span class="name">Playit for this server</span>
            <span class="setup-state"
              >Adds Playit tunnels without changing manual port forwarding.</span
            >
          </div>
          <Toggle
            checked={playit?.playitEnabled ?? false}
            label="Enable Playit for this server"
            disabled={playitBusy || !canControl || !playit}
            onchange={(enabled) => void setPlayitEnabled(enabled)}
          />
        </div>
        <div class="row">
          <div class="playit-status">
            <StatusDot
              tone={playit?.isRunning ? 'ok' : 'warn'}
              label={playit?.isRunning ? 'Running' : 'Stopped'}
            />
            <span class="setup-state"
              >{playit?.hasSecretKey ? 'Account configured' : 'Setup required'}</span
            >
          </div>
          <div class="control">
            <Button
              variant="secondary"
              size="sm"
              disabled={!playit?.playitEnabled || !canControl || playitBusy}
              onclick={() => (showPlayitSetup = true)}
              >{playit?.hasSecretKey ? 'Manage setup…' : 'Set up…'}</Button
            >
            <Button
              variant="secondary"
              size="sm"
              disabled={playitBusy ||
                !playit?.playitEnabled ||
                !playit?.hasSecretKey ||
                !canControl}
              onclick={togglePlayit}>{playit?.isRunning ? 'Stop' : 'Start'}</Button
            >
          </div>
        </div>
      </Card>
      <p class="hint">
        {playit?.note ?? 'MSC reuses one shared Java, Bedrock, and voice tunnel set.'}
      </p>
      {#if playit?.voiceAddress}
        <p class="hint">
          Voice tunnel: <code>{playit.voiceAddress}</code>.{#if serverStatus.running}
            The server is running; Simple Voice Chat reads this address when the server restarts.{/if}
        </p>
      {/if}
    </section>
  {/if}
</div>

{#if showPlayitSetup}
  <PlayitSetupSheet
    {api}
    {playit}
    context="settings"
    onClose={() => (showPlayitSetup = false)}
    onComplete={() => void loadAll()}
    onReset={() => void refreshPlayit()}
  />
{/if}

{#if showBroadcastAuth && broadcastAuth.isPresent}
  <BroadcastAuthSheet
    {api}
    prompt={broadcastAuth}
    onClose={() => {
      showBroadcastAuth = false;
      broadcastAuthHidden = true;
    }}
  />
{/if}

<style>
  .tab {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .notice {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .notice-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }
  .notice-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
  }
  .row.bordered {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .toggle-info {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .row-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .playit-status {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .setup-state {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
    line-height: 1.5;
  }
</style>
