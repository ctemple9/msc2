<script lang="ts">
  // MSC 1 HealthCardsGridView, rebuilt to the locked status card
  // (docs/msc2/renderings/status-card.html): no side rail, no colored
  // icon-in-box — status is a quiet label. Restores MSC 1's real
  // flip interaction (HealthGridCardTile's rotation3DEffect) that an
  // earlier pass had flattened away: the front face is icon + title +
  // status only, tapping flips to a back face carrying the detail line
  // and any repair action, one card flipped at a time (matching MSC 1's
  // single `flippedCardID`). Server Directory is dropped per Cameron's
  // 2026-08-26 call -- not useful enough to earn a card. The backend's
  // real card ids (crates/msc-agent/src/routes/health.rs) differ from
  // MSC 1's id set, and several report "gray -- not yet implemented"
  // honestly; this grid renders whatever the agent actually returns.
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema } from '../shared/types';

  export let cards: readonly Schema['HealthCardDTO'][] = [];

  $: visibleCards = cards.filter((card) => card.id !== 'directory');

  let flippedId: string | null = null;

  function toggle(id: string): void {
    flippedId = flippedId === id ? null : id;
  }
  function onTileKeydown(event: KeyboardEvent, id: string): void {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    toggle(id);
  }

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

  function label(severity: string): string {
    if (severity === 'green') return 'OK';
    if (severity === 'yellow') return 'Warn';
    if (severity === 'red') return 'Error';
    return 'Not checked';
  }
</script>

<div class="overline">
  <span class="msc2-type-overline">Server Health</span>
</div>

<div class="grid">
  {#each visibleCards as card (card.id)}
    {@const flipped = flippedId === card.id}
    <div
      class="tile-flip"
      role="button"
      tabindex="0"
      aria-pressed={flipped}
      aria-label="{card.title}, {label(card.severity)}. {flipped
        ? 'Showing details. Activate to go back.'
        : 'Activate for details.'}"
      onclick={() => toggle(card.id)}
      onkeydown={(event) => onTileKeydown(event, card.id)}
    >
      <div class="tile-inner" class:flipped>
        <div class="face front">
          <div class="tile-header">
            <Icon name={icon(card.id)} size={15} />
            <span class="title">{card.title}</span>
            <span class="hint" aria-hidden="true"><Icon name="chevron" size={11} /></span>
          </div>
          <span class="status-label">{label(card.severity)}</span>
        </div>
        <div class="face back">
          <div class="tile-header">
            <Icon name={icon(card.id)} size={13} />
            <span class="title">{card.title}</span>
            <span class="hint back-hint" aria-hidden="true"><Icon name="chevron" size={11} /></span>
          </div>
          <span class="status-label">{label(card.severity)}</span>
          {#if card.detail}
            <p class="detail">{card.detail}</p>
          {/if}
          {#if card.actionLabel && urlFor(card.actionCode)}
            <Button
              variant="secondary"
              size="sm"
              onclick={(event) => {
                event.stopPropagation();
                window.open(urlFor(card.actionCode), '_blank', 'noopener');
              }}
            >
              {card.actionLabel}
            </Button>
          {/if}
        </div>
      </div>
    </div>
  {:else}
    <div class="tile-flip placeholder">
      <div class="tile-inner">
        <div class="face front">
          <p class="detail">Waiting for the agent to report health data.</p>
        </div>
      </div>
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
  .tile-flip {
    perspective: 900px;
    height: 110px;
    cursor: pointer;
  }
  .tile-flip.placeholder {
    cursor: default;
  }
  .tile-inner {
    position: relative;
    width: 100%;
    height: 100%;
    transform-style: preserve-3d;
    transition: transform 320ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .tile-inner.flipped {
    transform: rotateY(180deg);
  }
  .face {
    position: absolute;
    inset: 0;
    backface-visibility: hidden;
    background: var(--msc2-tier-content);
    border-radius: 12px;
    padding: 13px 14px;
    box-shadow: var(--msc2-shadow-card);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .face.back {
    transform: rotateY(180deg);
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
  .hint {
    margin-left: auto;
    display: inline-flex;
    color: var(--msc2-text-tertiary);
    opacity: 0.6;
  }
  .back-hint {
    transform: rotate(180deg);
  }
  .status-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-tertiary);
  }
  .detail {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    line-height: 1.4;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
  }
</style>
