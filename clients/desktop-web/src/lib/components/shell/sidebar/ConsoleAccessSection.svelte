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
  import { onDestroy, onMount } from 'svelte';
  import Button from '../../base/Button.svelte';
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call, errorMessage, mutate } from '../../../sections/shared/types';
  import { serverEditorPaths } from '../../../sections/server-editor/model';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: string | undefined = undefined;
  export let activeServerId: string | undefined = undefined;
  export let running = false;
  export let canControl = true;
  export let showXboxBroadcast = false;

  let status: Schema['BroadcastStatusDTO'] | undefined;
  let jarStatus: Schema['BroadcastJarStatusDTO'] | undefined;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let broadcastBusy = false;
  let playitBusy = false;
  let notice = '';
  let loadedForServerId: string | undefined;
  let loadedRunning: boolean | undefined;
  let loadVersion = 0;
  let refreshTimer: ReturnType<typeof setInterval> | undefined;

  $: isBedrock = serverType === 'bedrock';
  $: broadcastRunning = isBedrock
    ? (status?.bedrockBroadcastRunning ?? false)
    : (status?.xboxBroadcastRunning ?? false);
  // Bedrock's Xbox Broadcast ships as a background process alongside BDS --
  // no separate "helper installed" concept the way Java's downloadable JAR
  // has, so it always shows controls; Java hides them behind the same
  // jar-installed gate the oracle's own XboxBroadcastSidebarRow uses.
  $: showBroadcastControls = isBedrock || (jarStatus?.installed ?? false);

  $: if (activeServerId !== loadedForServerId || running !== loadedRunning) {
    loadedForServerId = activeServerId;
    loadedRunning = running;
    void load();
  }

  async function load(): Promise<void> {
    const version = ++loadVersion;
    const nextValues = await Promise.all([
      call(api, status, serverEditorPaths.broadcastStatus),
      call(api, jarStatus, serverEditorPaths.broadcastJarStatus),
      call(api, playit, serverEditorPaths.playit),
    ]);
    if (version !== loadVersion) return;
    [status, jarStatus, playit] = nextValues;
  }

  onMount(() => {
    // Helper startup is asynchronous and is owned by the agent, so the first
    // read after the Minecraft process starts can still be stale. This keeps
    // the sidebar's Start/Stop labels reconciled without a manual refresh.
    refreshTimer = setInterval(() => void load(), 1000);
  });

  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
    loadVersion += 1;
  });

  async function toggleBroadcast(): Promise<void> {
    if (broadcastBusy) return;
    broadcastBusy = true;
    try {
      const path = broadcastRunning
        ? serverEditorPaths.broadcastStop
        : serverEditorPaths.broadcastStart;
      await mutate<Schema['BroadcastSimpleResultDTO']>(api, path);
      await load();
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      broadcastBusy = false;
    }
  }

  async function togglePlayit(): Promise<void> {
    if (playitBusy || !playit) return;
    playitBusy = true;
    try {
      const path = playit.isRunning ? serverEditorPaths.playitStop : serverEditorPaths.playitStart;
      await mutate(api, path);
      await load();
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      playitBusy = false;
    }
  }
</script>

<div class="console-access">
  <div class="service">
    <div class="service-header">
      <p class="msc2-type-overline">playit.gg</p>
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
      <div class="service-header">
        <p class="msc2-type-overline">Xbox Broadcast</p>
        {#if showBroadcastControls}
          <Button
            variant="secondary"
            size="sm"
            disabled={broadcastBusy || !canControl}
            onclick={toggleBroadcast}
          >
            {broadcastRunning ? 'Stop' : 'Start'}
          </Button>
        {/if}
      </div>
      {#if !showBroadcastControls}
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
    gap: 2px;
  }
  .service :global(.msc2-type-overline) {
    line-height: 1;
    margin: 0;
  }
  .service-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
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
