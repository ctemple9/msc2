<script lang="ts">
  // Ports ServerEditorBroadcastTab.swift (Java) -- Xbox broadcast runtime
  // controls plus the per-server Playit runtime panel. Host-wide Xbox account and
  // helper settings live in MSC Settings; this tab only controls this server.
  // assigned here rather than the host-level Manage sheet. Every route this tab calls
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
  import Button from '../../components/base/Button.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { pollOperation, serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let server: Schema['ServerDTO'];
  export let isActive = false;
  export let canControl = true;
  export let onRequestActivate: () => void;

  $: isJava = server.serverType !== 'bedrock';

  let status: Schema['BroadcastStatusDTO'] | undefined;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let serverStatus: Schema['RemoteAPIStatus'] = { running: false };

  let broadcastBusy = false;
  let playitBusy = false;

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
    // Invalidate requests started for the former host/server boundary. The
    // shared API client can change targets while an awaited request is out.
    playitLoadVersion += 1;
  }

  async function loadAll(): Promise<void> {
    const loadVersion = ++playitLoadVersion;
    const nextValues = await Promise.all([
      call(api, status, serverEditorPaths.broadcastStatus),
      call(api, playit, serverEditorPaths.playit),
      call(api, serverStatus, serverEditorPaths.status),
    ]);
    if (!isActive || loadVersion !== playitLoadVersion) return;
    [status, playit, serverStatus] = nextValues;
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
          <div class="toggle-info">
            <span class="name">Xbox Broadcast settings</span>
            <span class="setup-state">Manage the account and helper in MSC Settings.</span>
          </div>
        </div>
        <div class="row bordered">
          <span class="name">Join address</span>
          <span class="mono"
            >{server.hostAddress ?? '—'}{server.gamePort ? `:${server.gamePort}` : ''}</span
          >
        </div>
        <div class="row bordered">
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
      </Card>
    </section>

    <section class="zone">
      <p class="msc2-type-overline">Playit</p>
      <Card padding="0">
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
              disabled={playitBusy || !playit?.playitEnabled || !canControl}
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
  .mono {
    font-size: 12px;
    font-family: var(--msc2-font-mono, monospace);
    color: var(--msc2-text-tertiary);
  }
  .toggle-info {
    display: flex;
    align-items: center;
    gap: 10px;
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
