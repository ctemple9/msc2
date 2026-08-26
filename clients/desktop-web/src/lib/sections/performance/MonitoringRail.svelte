<script lang="ts">
  // MSC 1 DetailsPerformanceTabContent.swift's rightSidePanel: a
  // collapsible rail with Monitoring (live/paused state), Quick Actions,
  // and a Health Summary mirroring MSC 1's healthSummaryRow -- ported to
  // the shared StatusDot vocabulary (docs/msc2/renderings/status-card.html)
  // instead of MSC 1's own colored status icon + bold colored text.
  import Button from '../../components/base/Button.svelte';
  import ShellIcon from '../../components/shell/ShellIcon.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import HelpLink from '../../help/HelpLink.svelte';
  import type { Tone } from './model';
  import { toneLabel } from './model';

  export let collapsed = false;
  export let onToggleCollapse: () => void;
  export let serverRunning: boolean;
  export let paused: boolean;
  export let onTogglePause: () => void;
  export let onRefreshWorldSize: () => void;
  export let healthRows: readonly { label: string; tone: Tone }[];
  export let hostId: string;
  export let serverId: string;

  $: monitoringTone = (!serverRunning ? 'error' : paused ? 'warn' : 'ok') as
    'ok' | 'warn' | 'error';
  $: monitoringLabel = !serverRunning ? 'Offline' : paused ? 'Paused' : 'Active (5s)';
</script>

<div class="rail" class:collapsed>
  <button
    class="collapse-toggle"
    onclick={onToggleCollapse}
    aria-label={collapsed ? 'Show monitoring panel' : 'Hide monitoring panel'}
    title={collapsed ? 'Show panel' : 'Hide panel'}
  >
    <ShellIcon name="sidebar" size={13} />
  </button>

  {#if !collapsed}
    <div class="rail-body">
      <section class="rail-section">
        <h4>Monitoring</h4>
        <StatusDot tone={monitoringTone} label={monitoringLabel} />
        <Button size="sm" variant="secondary" disabled={!serverRunning} onclick={onTogglePause}>
          {paused ? 'Resume' : 'Pause'}
        </Button>
      </section>

      <div class="divider"></div>

      <section class="rail-section">
        <h4>Quick Actions</h4>
        <Button
          size="sm"
          variant="secondary"
          disabled={!serverRunning}
          onclick={onRefreshWorldSize}
        >
          Refresh World Size
        </Button>
        <div class="explain-link">
          <ShellIcon name="help" size={12} />
          <HelpLink helpId="handbook.ram-performance" {hostId} {serverId} />
        </div>
      </section>

      <div class="divider"></div>

      <section class="rail-section">
        <h4>Health Summary</h4>
        <div class="health-rows">
          {#each healthRows as row (row.label)}
            <div class="health-row">
              <span class="health-label">{row.label}</span>
              {#if row.tone}
                <StatusDot tone={row.tone} label={toneLabel(row.tone)} />
              {:else}
                <span class="neutral-status">
                  <span class="neutral-dot"></span>
                  <span class="neutral-label">—</span>
                </span>
              {/if}
            </div>
          {/each}
        </div>
      </section>
    </div>
  {/if}
</div>

<style>
  .rail {
    width: 190px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }
  .rail.collapsed {
    width: auto;
  }
  .collapse-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: var(--msc2-tier-content);
    color: var(--msc2-text-secondary);
    cursor: pointer;
    margin-bottom: 8px;
  }
  .collapse-toggle:hover {
    background: var(--msc2-neutral-elevated);
  }
  .rail-body {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .rail-section {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
  .rail-section h4 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    color: var(--msc2-text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .divider {
    width: 100%;
    height: 1px;
    background: var(--msc2-hairline-subtle);
  }
  .explain-link {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--msc2-text-tertiary);
  }
  .health-rows {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .health-row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .health-label {
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .neutral-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .neutral-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--msc2-neutral-muted);
    display: inline-block;
  }
  .neutral-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-tertiary);
  }
</style>
