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
  import type { PrimaryTab } from '../navigation/primaryTabs';
  import type { Schema } from '../sections/shared/types';

  export let hostLabel = 'No host selected';
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
  export let onManage: () => void;
  export let onHelp: (() => void) | undefined = undefined;
  export let onSettings: (() => void) | undefined = undefined;
  export let onRefresh: (() => void) | undefined = undefined;

  const SIDEBAR_KEY = 'msc2.sidebarCollapsed';
  const CONSOLE_KEY = 'msc2.consoleHidden';

  let sidebarCollapsed =
    typeof localStorage !== 'undefined' && localStorage.getItem(SIDEBAR_KEY) === '1';
  let consoleCollapsed =
    typeof localStorage !== 'undefined' && localStorage.getItem(CONSOLE_KEY) === '1';

  function toggleSidebar(): void {
    sidebarCollapsed = !sidebarCollapsed;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(SIDEBAR_KEY, sidebarCollapsed ? '1' : '0');
    }
  }

  function toggleConsole(): void {
    consoleCollapsed = !consoleCollapsed;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(CONSOLE_KEY, consoleCollapsed ? '1' : '0');
    }
  }

  $: activeServer = servers.find((server) => server.id === activeServerId);
</script>

<div class="shell">
  <TopBar
    {bannerColor}
    {running}
    {sidebarCollapsed}
    onToggleSidebar={toggleSidebar}
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
        {servers}
        {activeServerId}
        {running}
        {connected}
        {canControl}
        {bannerColor}
        {onSelectServer}
        {onLifecycle}
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

      <ConsoleDock collapsed={consoleCollapsed} onToggle={toggleConsole} />
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
</style>
