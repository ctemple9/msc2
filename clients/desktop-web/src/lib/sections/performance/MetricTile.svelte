<script lang="ts">
  // MSC 1 DetailsPerformanceComponents.swift's enhancedMetricTile, rebuilt
  // to the S0 disciplined system (docs/msc2/antiAIslop.md): the oracle
  // colors a small status icon and a strokeBorder by tone (rule #11's
  // "no side rails/accent bars", and rule #3's "no colored icon-in-box" by
  // extension). The icon is dropped here; a colored *value* (the design
  // law's explicitly allowed "live-stat fill") and a quiet text label give
  // each tile its hierarchy. Value color is the only accent this tile spends.
  import type { Tone } from './model';
  import { toneLabel } from './model';

  export let title: string;
  export let value: string;
  export let tone: Tone = undefined;
  export let subtitle = '';

  const toneColor: Record<'ok' | 'warn' | 'error', string> = {
    ok: 'var(--msc2-status-ok)',
    warn: 'var(--msc2-status-warn)',
    error: 'var(--msc2-status-error)',
  };
</script>

<div class="tile">
  <div class="tile-header">
    <span class="title">{title}</span>
  </div>
  <p class="value" style={tone ? `color: ${toneColor[tone]};` : ''}>{value}</p>
  {#if tone}
    <span class="tone-label" style={`color: ${toneColor[tone]};`}>{toneLabel(tone)}</span>
  {/if}
  {#if subtitle}<p class="subtitle">{subtitle}</p>{/if}
</div>

<style>
  .tile {
    background: var(--msc2-tier-content);
    border-radius: 12px;
    box-shadow: var(--msc2-shadow-card);
    padding: 13px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .tile-header {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .title {
    font-size: 12px;
    font-weight: 500;
  }
  .value {
    margin: 2px 0 0;
    font-size: 21px;
    font-weight: 600;
    color: var(--msc2-text-primary);
    line-height: 1.2;
  }
  .subtitle {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .tone-label {
    font-size: 12px;
    font-weight: 500;
  }
</style>
