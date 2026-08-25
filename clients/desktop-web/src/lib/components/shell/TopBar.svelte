<script lang="ts">
  // Window chrome: title + the one sanctioned flourish (the terrain banner, shown
  // only while running) + four icon actions. No fake OS traffic lights — the real
  // window chrome comes from the browser or the Tauri titlebar.
  // docs/msc2/renderings/shell.html, MSC 1 ContentView.swift top banner.
  import ShellIcon from './ShellIcon.svelte';
  import { bannerColorAccent } from '../../styles/bannerColor';

  export let bannerColor: string;
  export let running = false;
  export let sidebarCollapsed = false;
  export let onToggleSidebar: () => void;
  export let consoleCollapsed = false;
  export let onToggleConsole: () => void;
  export let onHelp: (() => void) | undefined = undefined;
  export let onSettings: (() => void) | undefined = undefined;
  export let onRefresh: (() => void) | undefined = undefined;
</script>

<div class="topbar">
  <div class="titles">
    <span class="title">Minecraft Server Controller</span>
    <span class="subtitle">by TempleTech</span>
  </div>

  {#if running}
    <div
      class="terrain"
      style="background: {bannerColorAccent(bannerColor, 1)};"
      aria-hidden="true"
    >
      <div class="ground"></div>
      <div class="grass"></div>
    </div>
  {:else}
    <div class="spacer"></div>
  {/if}

  <div class="actions">
    <button
      type="button"
      class="icon-btn"
      aria-label={sidebarCollapsed ? 'Show sidebar' : 'Hide sidebar'}
      onclick={onToggleSidebar}
    >
      <ShellIcon name="sidebar" />
    </button>
    <button
      type="button"
      class="icon-btn"
      aria-label={consoleCollapsed ? 'Show console' : 'Hide console'}
      onclick={onToggleConsole}
    >
      <ShellIcon name="console" />
    </button>
    <button
      type="button"
      class="icon-btn"
      aria-label="Help & guides"
      disabled={!onHelp}
      onclick={onHelp}
    >
      <ShellIcon name="help" />
    </button>
    <button
      type="button"
      class="icon-btn"
      aria-label="Preferences"
      disabled={!onSettings}
      onclick={onSettings}
    >
      <ShellIcon name="settings" />
    </button>
    <button
      type="button"
      class="icon-btn"
      aria-label="Refresh"
      disabled={!onRefresh}
      onclick={onRefresh}
    >
      <ShellIcon name="refresh" />
    </button>
  </div>
</div>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 52px;
    flex-shrink: 0;
    padding: 0 16px;
    background: var(--msc2-tier-chrome);
    border-bottom: 1px solid var(--msc2-hairline-faint);
  }
  .titles {
    display: flex;
    flex-direction: column;
    gap: 1px;
    white-space: nowrap;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .subtitle {
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }
  .spacer,
  .terrain {
    flex: 1;
    min-width: 0;
    height: 30px;
    border-radius: 7px;
    overflow: hidden;
    position: relative;
  }
  .terrain .ground {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 40%;
    background: rgba(0, 0, 0, 0.28);
  }
  .terrain .grass {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 40%;
    height: 14%;
    background: rgba(255, 255, 255, 0.18);
  }
  .actions {
    display: flex;
    gap: 4px;
  }
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    color: rgba(255, 255, 255, 0.6);
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }
  .icon-btn:hover:not(:disabled) {
    color: rgba(255, 255, 255, 0.9);
    background: rgba(255, 255, 255, 0.08);
  }
  .icon-btn:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
    outline-offset: 1px;
  }
  .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
</style>
