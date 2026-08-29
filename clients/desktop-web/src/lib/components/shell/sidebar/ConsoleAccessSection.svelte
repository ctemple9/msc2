<script lang="ts">
  // Ports MSC 1 SidebarView.swift's BedrockCrossPlatformSidebarSection /
  // CrossPlatformAccessSidebarSection -- a condensed Xbox Broadcast
  // quick-control, not the deeper setup surface (that stays
  // BroadcastTab.svelte's job, per the oracle's own contextual-help copy:
  // "use Edit Server for the deeper setup path"). Reuses the same three
  // routes and status/autostart shape BroadcastTab.svelte already calls.
  //
  // Visibility (who renders this component at all) lives in ControlSidebar,
  // since it depends on the server-picker's flavor/category, not on
  // anything this component itself fetches.
  import Button from '../../base/Button.svelte';
  import Toggle from '../../base/Toggle.svelte';
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call, errorMessage, mutate } from '../../../sections/shared/types';
  import { serverEditorPaths } from '../../../sections/server-editor/model';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: string | undefined = undefined;
  export let activeServerId: string | undefined = undefined;
  export let canControl = true;

  let status: Schema['BroadcastStatusDTO'] | undefined;
  let autostart: Schema['BroadcastAutoStartDTO'] | undefined;
  let jarStatus: Schema['BroadcastJarStatusDTO'] | undefined;
  let broadcastBusy = false;
  let notice = '';
  let loadedForServerId: string | undefined;

  $: isBedrock = serverType === 'bedrock';
  $: running = isBedrock
    ? (status?.bedrockBroadcastRunning ?? false)
    : (status?.xboxBroadcastRunning ?? false);
  // Bedrock's Xbox Broadcast ships as a background process alongside BDS --
  // no separate "helper installed" concept the way Java's downloadable JAR
  // has, so it always shows controls; Java hides them behind the same
  // jar-installed gate the oracle's own XboxBroadcastSidebarRow uses.
  $: showControls = isBedrock || (jarStatus?.installed ?? false);

  $: if (activeServerId !== loadedForServerId) {
    loadedForServerId = activeServerId;
    void load();
  }

  async function load(): Promise<void> {
    [status, autostart, jarStatus] = await Promise.all([
      call(api, status, serverEditorPaths.broadcastStatus),
      call(api, autostart, serverEditorPaths.broadcastAutostart),
      call(api, jarStatus, serverEditorPaths.broadcastJarStatus),
    ]);
  }

  async function toggleBroadcast(): Promise<void> {
    if (broadcastBusy) return;
    broadcastBusy = true;
    try {
      const path = running ? serverEditorPaths.broadcastStop : serverEditorPaths.broadcastStart;
      await mutate<Schema['BroadcastSimpleResultDTO']>(api, path);
      status = await call(api, status, serverEditorPaths.broadcastStatus);
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      broadcastBusy = false;
    }
  }

  async function toggleAutostart(enabled: boolean): Promise<void> {
    try {
      autostart = await mutate<Schema['BroadcastAutoStartDTO']>(
        api,
        serverEditorPaths.broadcastAutostart,
        { enabled },
      );
    } catch (error) {
      notice = errorMessage(error);
    }
  }
</script>

<div class="console-access">
  {#if showControls}
    <div class="row">
      <span class="dot" class:online={running}></span>
      <span class="status-label">{running ? 'Running' : 'Stopped'}</span>
      <Button
        variant="secondary"
        size="sm"
        disabled={broadcastBusy || !canControl}
        onclick={toggleBroadcast}
      >
        {running ? 'Stop' : 'Start'}
      </Button>
    </div>
    <div class="row toggle-row">
      <Toggle
        checked={autostart?.enabled ?? false}
        label="Start Xbox broadcast automatically"
        disabled={!canControl}
        onchange={toggleAutostart}
      />
      <span class="label">Auto-start with server</span>
    </div>
  {:else}
    <p class="hint">Set up in Edit Server → Broadcast</p>
  {/if}
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}
</div>

<style>
  .console-access {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--msc2-neutral-muted);
    flex-shrink: 0;
  }
  .dot.online {
    background: var(--msc2-status-ok);
  }
  .status-label {
    flex: 1;
    font-size: 11px;
    color: var(--msc2-text-secondary);
  }
  .toggle-row {
    gap: 8px;
  }
  .label {
    font-size: 11px;
    color: var(--msc2-text-secondary);
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    line-height: 1.5;
  }
  .notice {
    margin: 0;
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }
</style>
