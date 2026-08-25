<script lang="ts">
  // Server-control rail: host-aware picker, Start/Stop, Manage…, and MSC 1's
  // four collapsible sections. Their content is real shape (labels, collapse
  // behavior); their data-carrying content becomes real as its own screen is
  // rebuilt later (Console Access -> P12.7, Maintenance/Quick Commands -> a
  // later step), matching how the console dock is frame-only here too.
  // docs/msc2/renderings/shell.html, MSC 1 SidebarView.swift.
  import Button from '../base/Button.svelte';
  import ShellIcon from './ShellIcon.svelte';
  import { bannerColorAccent } from '../../styles/bannerColor';
  import type { Schema } from '../../sections/shared/types';

  export let hostLabel: string;
  export let servers: readonly Schema['ServerDTO'][] = [];
  export let activeServerId: string | undefined = undefined;
  export let running = false;
  export let connected = false;
  export let canControl = true;
  export let bannerColor: string;
  export let onSelectServer: (id: string) => void;
  export let onLifecycle: (action: 'start' | 'stop') => void;
  export let onManage: () => void;

  const DISCLOSURE_SECTIONS = [
    'Console access',
    'How to connect',
    'Maintenance',
    'Quick commands',
  ] as const;
  let expanded: Record<(typeof DISCLOSURE_SECTIONS)[number], boolean> = {
    'Console access': false,
    'How to connect': false,
    Maintenance: false,
    'Quick commands': false,
  };

  function toggle(section: (typeof DISCLOSURE_SECTIONS)[number]): void {
    expanded = { ...expanded, [section]: !expanded[section] };
  }

  $: activeServer = servers.find((server) => server.id === activeServerId);
</script>

<aside class="sidebar" aria-label="Server controls">
  <div class="scroll">
    <div class="block">
      <p class="overline">Server controls</p>

      <div class="picker" style="background: {bannerColorAccent(bannerColor, 0.12)};">
        <span class="sr-only">{connected ? 'Connected' : 'Disconnected'}</span>
        <span class="picker-label">{hostLabel} ▸ {activeServer?.name ?? 'No server'}</span>
        <ShellIcon name="selector" size={14} />
        {#if servers.length}
          <select
            class="picker-input"
            aria-label="Select server"
            value={activeServerId}
            onchange={(event) => onSelectServer((event.currentTarget as HTMLSelectElement).value)}
          >
            {#each servers as server (server.id)}
              <option value={server.id}>{server.name}</option>
            {/each}
          </select>
        {/if}
      </div>

      <div class="control-row">
        <Button
          variant={running ? 'stop' : 'start'}
          size="sm"
          disabled={!canControl || !activeServer}
          onclick={() => onLifecycle(running ? 'stop' : 'start')}
        >
          <ShellIcon name="play" size={13} />
          {running ? 'Stop' : 'Start'}
        </Button>
        <Button variant="secondary" size="sm" onclick={onManage}>Manage…</Button>
      </div>
    </div>

    {#each DISCLOSURE_SECTIONS as section (section)}
      <div class="disclosure">
        <button
          type="button"
          class="disclosure-header"
          aria-expanded={expanded[section]}
          onclick={() => toggle(section)}
        >
          <ShellIcon name={expanded[section] ? 'chevron-down' : 'chevron-right'} size={11} />
          <span class="overline">{section}</span>
        </button>
        {#if expanded[section]}
          <p class="placeholder">Rebuilt in a later Phase 12 step.</p>
        {/if}
      </div>
    {/each}
  </div>

  <div class="avatar-row">
    <span class="avatar"><ShellIcon name="user" size={15} /></span>
    <span class="avatar-label">Your avatar</span>
  </div>
</aside>

<style>
  .sidebar {
    width: 240px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--msc2-tier-chrome);
    border-right: 1px solid var(--msc2-hairline-faint);
    min-height: 0;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 14px 12px 8px;
  }
  .overline {
    margin: 0;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.8px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .block {
    margin-bottom: 14px;
  }
  .picker {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    padding: 7px 9px;
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    color: var(--msc2-text-secondary);
  }
  .picker:focus-within {
    border-color: var(--msc2-hairline-field-focus);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .picker-label {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--msc2-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .picker-input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }
  .control-row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .control-row :global(.btn.secondary) {
    flex-shrink: 0;
  }
  .disclosure {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .disclosure-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px 2px;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    text-align: left;
  }
  .disclosure-header:hover {
    color: rgba(255, 255, 255, 0.8);
  }
  .disclosure-header:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
  }
  .placeholder {
    margin: 0 0 10px 19px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.35);
    line-height: 1.5;
  }
  .avatar-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    padding: 10px 12px 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .avatar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: var(--msc2-neutral-elevated);
    color: rgba(255, 255, 255, 0.5);
  }
  .avatar-label {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
  }
</style>
