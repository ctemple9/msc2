<script lang="ts">
  // The S1 shell skeleton, rebuilt to docs/msc2/renderings/shell.html and MSC 1's
  // ContentView/SidebarView/DetailsHeaderSectionView/MSCTabBar. Governed by
  // docs/msc2/antiAIslop.md. bannerColor appears in exactly four places: the
  // running terrain banner, the header wash, the selected tab pill, and (later)
  // sidebar selection.
  import TopBar from './shell/TopBar.svelte';
  import ControlSidebar from './shell/ControlSidebar.svelte';
  import DetailsHeader from './shell/DetailsHeader.svelte';
  import TabStrip from './shell/TabStrip.svelte';
  import ConsoleDock from './shell/ConsoleDock.svelte';
  import ShellIcon from './shell/ShellIcon.svelte';
  import { onboardingAnchor } from '../help/tourAnchors';
  import type { PrimaryTab } from '../navigation/primaryTabs';
  import type { HostId, HostRecord } from '../hosts/types';
  import type { Schema, ScreenApi } from '../sections/shared/types';

  export let hostLabel = 'No host selected';
  export let hosts: readonly HostRecord[] = [];
  export let activeHostId: HostId = '';
  export let isDesktopShell = false;
  export let onSwitchHost: (id: HostId) => void = () => undefined;
  // Threaded straight through to ConsoleDock (P12.10) — the docked console is the
  // one piece of shell chrome that needs to call the agent directly, the same `api`
  // every section already receives.
  export let api: ScreenApi | undefined = undefined;
  export let servers: readonly Schema['ServerDTO'][] = [];
  export let activeServerId: string | undefined = undefined;
  export let running = false;
  export let connected = false;
  export let canControl = true;
  export let bannerColor: string;
  export let tabs: readonly (PrimaryTab & { available: boolean })[] = [];
  export let activeSection = '';
  export let selectSection: (id: string) => void;
  export let onSelectServer: (id: string) => void;
  export let onLifecycle: (action: 'start' | 'stop') => void;
  export let onOpenAgentSetup: () => void;
  export let onManage: () => void;
  export let onHelp: (() => void) | undefined = undefined;
  export let onOpenBrowser: (() => void) | undefined = undefined;
  export let onSettings: (() => void) | undefined = undefined;
  export let onRefresh: (() => void) | undefined = undefined;

  const SIDEBAR_KEY = 'msc2.sidebarCollapsed';
  const CONSOLE_KEY = 'msc2.consoleHidden';

  // Mirrors MSC 1 ContentView.swift's consoleDivider: drag resizes within a
  // floor, and releasing well past that floor collapses the console instead.
  const MIN_CONSOLE_HEIGHT = 120;
  const MAX_CONSOLE_HEIGHT = 560;
  const DEFAULT_CONSOLE_HEIGHT = 220;
  const COLLAPSE_BELOW = MIN_CONSOLE_HEIGHT * 0.5;

  let sidebarCollapsed =
    typeof localStorage !== 'undefined' && localStorage.getItem(SIDEBAR_KEY) === '1';
  let consoleCollapsed =
    typeof localStorage !== 'undefined' && localStorage.getItem(CONSOLE_KEY) === '1';
  let consoleHeight = DEFAULT_CONSOLE_HEIGHT;

  let dragStartY: number | null = null;
  let dragStartHeight = consoleHeight;
  let dragRawHeight = consoleHeight;

  function toggleSidebar(): void {
    sidebarCollapsed = !sidebarCollapsed;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(SIDEBAR_KEY, sidebarCollapsed ? '1' : '0');
    }
  }

  function setConsoleCollapsed(value: boolean): void {
    consoleCollapsed = value;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(CONSOLE_KEY, consoleCollapsed ? '1' : '0');
    }
  }

  function toggleConsole(): void {
    setConsoleCollapsed(!consoleCollapsed);
  }

  function startConsoleResize(event: PointerEvent): void {
    dragStartY = event.clientY;
    dragStartHeight = consoleHeight;
    dragRawHeight = consoleHeight;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function onConsoleResize(event: PointerEvent): void {
    if (dragStartY === null) return;
    dragRawHeight = dragStartHeight + (dragStartY - event.clientY);
    consoleHeight = Math.min(MAX_CONSOLE_HEIGHT, Math.max(MIN_CONSOLE_HEIGHT, dragRawHeight));
  }

  function endConsoleResize(): void {
    if (dragStartY === null) return;
    dragStartY = null;
    if (dragRawHeight < COLLAPSE_BELOW) setConsoleCollapsed(true);
  }

  $: activeServer = servers.find((server) => server.id === activeServerId);
</script>

<div class="shell">
  <TopBar
    {bannerColor}
    {running}
    {sidebarCollapsed}
    onToggleSidebar={toggleSidebar}
    {consoleCollapsed}
    onToggleConsole={toggleConsole}
    {onOpenBrowser}
    {onHelp}
    {onSettings}
    {onRefresh}
  />

  <div class="body">
    {#if sidebarCollapsed}
      <button type="button" class="sidebar-rail" aria-label="Show sidebar" onclick={toggleSidebar}>
        <ShellIcon name="chevron-right" size={11} />
      </button>
    {:else}
      <ControlSidebar
        {hostLabel}
        {hosts}
        {activeHostId}
        {isDesktopShell}
        {servers}
        {activeServerId}
        {running}
        {connected}
        {canControl}
        {bannerColor}
        {onSelectServer}
        {onSwitchHost}
        {onLifecycle}
        {onOpenAgentSetup}
        {onManage}
      />
    {/if}

    <div class="main">
      <DetailsHeader
        serverName={activeServer?.name ?? 'No server selected'}
        serverType={activeServer?.serverType}
        javaFlavor={activeServer?.javaFlavor}
        directory={activeServer?.directory}
        {running}
        {bannerColor}
      />

      <div class="tabs-row">
        <TabStrip {tabs} activeId={activeSection} {bannerColor} onSelect={selectSection} />
      </div>

      <section class="content" aria-live="polite">
        <slot />
      </section>

      {#if !consoleCollapsed}
        <div
          class="console-resize"
          role="separator"
          aria-orientation="horizontal"
          aria-label="Resize console"
          onpointerdown={startConsoleResize}
          onpointermove={onConsoleResize}
          onpointerup={endConsoleResize}
          use:onboardingAnchor={'ob_console_divider_handle'}
        >
          <span class="handle" aria-hidden="true"></span>
        </div>
      {/if}

      <ConsoleDock
        collapsed={consoleCollapsed}
        onToggle={toggleConsole}
        height={consoleCollapsed ? undefined : consoleHeight}
        {api}
        serverType={activeServer?.serverType}
      />
    </div>
  </div>
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--msc2-tier-atmosphere);
    color: var(--msc2-text-primary);
    font-family: var(--msc2-font-sans);
    overflow: hidden;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .sidebar-rail {
    width: 18px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.4);
    background: var(--msc2-tier-chrome);
    border: none;
    border-right: 1px solid var(--msc2-hairline-faint);
    cursor: pointer;
  }
  .sidebar-rail:hover {
    color: rgba(255, 255, 255, 0.7);
  }
  .sidebar-rail:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .tabs-row {
    flex-shrink: 0;
    padding: 10px 14px 0;
    background: var(--msc2-tier-atmosphere);
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 14px;
    background: var(--msc2-tier-atmosphere);
  }
  .console-resize {
    flex-shrink: 0;
    height: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--msc2-tier-atmosphere);
    cursor: row-resize;
    touch-action: none;
  }
  .console-resize .handle {
    width: 36px;
    height: 4px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.14);
  }
  .console-resize:hover .handle,
  .console-resize:active .handle {
    background: rgba(255, 255, 255, 0.3);
  }
</style>
