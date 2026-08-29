<script lang="ts">
  // Shown in the sidebar under "Services" (renamed from "Console Access" --
  // Xbox Broadcast's own MSC 1 name -- once this became a general panel).
  // MSC2's own external-services quick control -- the two helper processes
  // the agent manages that are not the Minecraft server process itself
  // (crates/msc-infrastructure/src/helper_process.rs's HelperKey has
  // exactly two call sites: "xbox-broadcast" and "playit").
  // Xbox Broadcast started here as a port of MSC 1 SidebarView.swift's
  // BedrockCrossPlatformSidebarSection/CrossPlatformAccessSidebarSection;
  // Playit is Cameron's own addition on top of that, not an oracle port --
  // the oracle has no sidebar Playit control at all. Neither service is the
  // deeper setup surface (that stays BroadcastTab.svelte's job, per the
  // oracle's own contextual-help copy: "use Edit Server for the deeper
  // setup path"). Reuses the same routes and status shapes BroadcastTab.svelte
  // already calls for both.
  //
  // Playit applies to any server type/flavor (it's a generic tunnel, not
  // crossplay-specific), so it always renders here. Xbox Broadcast's own
  // crossplay visibility rule lives in ControlSidebar and is passed down as
  // showXboxBroadcast.
  import Button from '../../base/Button.svelte';
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call, errorMessage, mutate } from '../../../sections/shared/types';
  import { serverEditorPaths } from '../../../sections/server-editor/model';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: string | undefined = undefined;
  export let activeServerId: string | undefined = undefined;
  export let canControl = true;
  export let showXboxBroadcast = false;

  let status: Schema['BroadcastStatusDTO'] | undefined;
  let autostart: Schema['BroadcastAutoStartDTO'] | undefined;
  let jarStatus: Schema['BroadcastJarStatusDTO'] | undefined;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let broadcastBusy = false;
  let playitBusy = false;
  let notice = '';
  let loadedForServerId: string | undefined;

  $: isBedrock = serverType === 'bedrock';
  $: broadcastRunning = isBedrock
    ? (status?.bedrockBroadcastRunning ?? false)
    : (status?.xboxBroadcastRunning ?? false);
  // Bedrock's Xbox Broadcast ships as a background process alongside BDS --
  // no separate "helper installed" concept the way Java's downloadable JAR
  // has, so it always shows controls; Java hides them behind the same
  // jar-installed gate the oracle's own XboxBroadcastSidebarRow uses.
  $: showBroadcastControls = isBedrock || (jarStatus?.installed ?? false);

  $: if (activeServerId !== loadedForServerId) {
    loadedForServerId = activeServerId;
    void load();
  }

  async function load(): Promise<void> {
    [status, autostart, jarStatus, playit] = await Promise.all([
      call(api, status, serverEditorPaths.broadcastStatus),
      call(api, autostart, serverEditorPaths.broadcastAutostart),
      call(api, jarStatus, serverEditorPaths.broadcastJarStatus),
      call(api, playit, serverEditorPaths.playit),
    ]);
  }

  async function toggleBroadcast(): Promise<void> {
    if (broadcastBusy) return;
    broadcastBusy = true;
    try {
      const path = broadcastRunning
        ? serverEditorPaths.broadcastStop
        : serverEditorPaths.broadcastStart;
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

  async function togglePlayit(): Promise<void> {
    if (playitBusy || !playit) return;
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

<div class="console-access">
  <div class="service">
    <p class="msc2-type-overline">playit.gg</p>
    <div class="row">
      <span class="dot" class:online={playit?.isRunning}></span>
      <span class="status-label">{playit?.isRunning ? 'Running' : 'Stopped'}</span>
      <Button
        variant="secondary"
        size="sm"
        disabled={playitBusy || !playit?.playitEnabled || !canControl}
        onclick={togglePlayit}
      >
        {playit?.isRunning ? 'Stop' : 'Start'}
      </Button>
    </div>
    {#if !playit?.playitEnabled}
      <p class="hint">Enable in Edit Server → Broadcast</p>
    {/if}
  </div>

  {#if showXboxBroadcast}
    <div class="service">
      <p class="msc2-type-overline">Xbox Broadcast</p>
      {#if showBroadcastControls}
        <div class="row">
          <span class="dot" class:online={broadcastRunning}></span>
          <span class="status-label">{broadcastRunning ? 'Running' : 'Stopped'}</span>
          <label class="auto-check" title="Start Xbox broadcast automatically">
            <input
              type="checkbox"
              checked={autostart?.enabled ?? false}
              disabled={!canControl}
              onchange={(event) =>
                toggleAutostart((event.currentTarget as HTMLInputElement).checked)}
            />
            Auto-start
          </label>
          <Button
            variant="secondary"
            size="sm"
            disabled={broadcastBusy || !canControl}
            onclick={toggleBroadcast}
          >
            {broadcastRunning ? 'Stop' : 'Start'}
          </Button>
        </div>
      {:else}
        <p class="hint">Set up in Edit Server → Broadcast</p>
      {/if}
    </div>
  {/if}

  {#if notice}<p class="notice" role="status">{notice}</p>{/if}
</div>

<style>
  .console-access {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .service {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
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
  .auto-check {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: var(--msc2-text-secondary);
    white-space: nowrap;
    cursor: pointer;
  }
  .auto-check input {
    width: 12px;
    height: 12px;
    margin: 0;
    accent-color: var(--msc2-status-ok);
    cursor: pointer;
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
