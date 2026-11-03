<script lang="ts">
  // Window chrome: title, the running terrain banner, header actions, and the
  // tour actions when the guided-tour card is hidden. Tauri provides the titlebar.
  // docs/msc2/renderings/shell.html, MSC 1 ContentView.swift top banner.
  import ShellIcon from './ShellIcon.svelte';
  import RunningBannerGame from './RunningBannerGame.svelte';
  import Button from '../base/Button.svelte';
  import { hiddenTourActions } from '../../help/onboarding';

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
    <span class="subtitle">by ctemple9</span>
  </div>

  {#if running}
    <RunningBannerGame {running} {bannerColor} />
  {:else}
    <div class="spacer"></div>
  {/if}

  <div class="actions" class:has-tour-actions={$hiddenTourActions !== null}>
    {#if $hiddenTourActions}
      <div class="tour-actions" role="group" aria-label="Tour actions">
        <Button variant="secondary" size="sm" onclick={$hiddenTourActions.showTip}>Show tip</Button>
        <Button variant="secondary" size="sm" onclick={$hiddenTourActions.skip}
          >{$hiddenTourActions.skipLabel}</Button
        >
      </div>
    {/if}
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
  .spacer {
    flex: 1;
    min-width: 0;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .tour-actions {
    display: flex;
    gap: 4px;
    padding-right: 8px;
    margin-right: 4px;
    border-right: 1px solid var(--msc2-hairline-faint);
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
  @media (max-width: 600px) {
    .topbar {
      height: auto;
      min-height: 52px;
      flex-wrap: wrap;
      padding: 8px 12px;
    }
    .spacer {
      display: none;
    }
    .actions.has-tour-actions {
      flex-basis: 100%;
      flex-wrap: wrap;
      justify-content: flex-end;
    }
    .tour-actions {
      order: 2;
      flex-basis: 100%;
      justify-content: flex-end;
      padding-right: 0;
      margin-right: 0;
      border-right: 0;
    }
    .actions.has-tour-actions > .icon-btn {
      order: 1;
    }
  }
</style>
