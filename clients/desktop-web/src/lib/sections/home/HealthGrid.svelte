<script lang="ts">
  // MSC 1 HealthCardsGridView, rebuilt to the locked status card
  // (docs/msc2/renderings/status-card.html): no side rail, no colored
  // icon-in-box, no 3D flip — status is dot + label only, and the detail
  // line + repair action sit right on the one card face. The backend's real
  // card ids (directory/java/ram/lastStartup/portReachability/componentJars,
  // crates/msc-agent/src/routes/health.rs) differ from MSC 1's, and several
  // report "gray — not yet implemented" honestly; this grid renders whatever
  // the agent actually returns rather than assuming MSC 1's id set.
  import StatusDot from '../../components/base/StatusDot.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema } from '../shared/types';

  export let cards: readonly Schema['HealthCardDTO'][] = [];

  // The only card action codes the agent emits today are "locateFolder"
  // (needs a native folder picker this web/Tauri build doesn't wire yet)
  // and "openURL:<url>" (crates/msc-application/src/diagnostics.rs). Only
  // the second is something this client can honestly fulfill.
  function urlFor(actionCode: string | undefined): string | undefined {
    return actionCode?.startsWith('openURL:') ? actionCode.slice('openURL:'.length) : undefined;
  }

  const iconFor: Record<
    string,
    'folder' | 'cup' | 'chip' | 'seal-check' | 'network' | 'grid' | 'world'
  > = {
    directory: 'folder',
    java: 'cup',
    ram: 'chip',
    lastStartup: 'seal-check',
    portReachability: 'network',
    componentJars: 'grid',
    bedrockWorldData: 'world',
    vmRuntime: 'chip',
  };

  function icon(
    id: string,
  ): 'folder' | 'cup' | 'chip' | 'seal-check' | 'network' | 'grid' | 'world' {
    return iconFor[id] ?? 'grid';
  }

  function tone(severity: string): 'ok' | 'warn' | 'error' | undefined {
    if (severity === 'green') return 'ok';
    if (severity === 'yellow') return 'warn';
    if (severity === 'red') return 'error';
    return undefined;
  }

  function label(severity: string): string {
    if (severity === 'green') return 'OK';
    if (severity === 'yellow') return 'Warn';
    if (severity === 'red') return 'Error';
    return 'Not checked';
  }

  function firstLine(detail: string | undefined): string {
    if (!detail) return '';
    return detail.split('\n')[0]?.trim() ?? '';
  }
</script>

<div class="overline">
  <span class="msc2-type-overline">Server Health</span>
</div>

<div class="grid">
  {#each cards as card (card.id)}
    <div class="tile">
      <div class="tile-header">
        <Icon name={icon(card.id)} size={15} />
        <span class="title">{card.title}</span>
      </div>
      {#if tone(card.severity)}
        <StatusDot tone={tone(card.severity)} label={label(card.severity)} />
      {:else}
        <span class="neutral-status">
          <span class="neutral-dot"></span>
          <span class="neutral-label">{label(card.severity)}</span>
        </span>
      {/if}
      {#if firstLine(card.detail)}
        <p class="detail">{firstLine(card.detail)}</p>
      {/if}
      {#if card.actionLabel && urlFor(card.actionCode)}
        <Button
          variant="secondary"
          size="sm"
          onclick={() => window.open(urlFor(card.actionCode), '_blank', 'noopener')}
        >
          {card.actionLabel}
        </Button>
      {/if}
    </div>
  {:else}
    <div class="tile placeholder">
      <p class="detail">Waiting for the agent to report health data.</p>
    </div>
  {/each}
</div>

<style>
  .overline {
    margin-bottom: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
  }
  .tile {
    background: var(--msc2-tier-content);
    border-radius: 12px;
    padding: 13px 14px;
    box-shadow: var(--msc2-shadow-card);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .tile-header {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--msc2-text-tertiary);
    margin-bottom: 2px;
  }
  .title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
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
  .detail {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    line-height: 1.4;
  }
</style>
