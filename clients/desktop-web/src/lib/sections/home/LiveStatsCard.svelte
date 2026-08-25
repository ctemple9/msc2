<script lang="ts">
  // MSC 1 OverviewJavaLivePanel — three vertical fill gauges (CPU/RAM/TPS),
  // ported without the gradient fill (flat status color instead, S0's
  // "no gradients" rule) and without the tick-mark decoration (it carried
  // no information beyond the fill itself).
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { Schema } from '../shared/types';

  export let snapshot: Schema['PerformanceSnapshotDTO'] | undefined = undefined;

  type Gauge = { label: string; value: string; fraction: number; hasData: boolean; tone: string };

  function toneFor(kind: 'cpu' | 'ram' | 'tps', v: number): string {
    if (kind === 'tps') {
      if (v >= 19.5) return 'ok';
      if (v >= 18) return 'warn';
      return 'error';
    }
    const pct = kind === 'cpu' ? v : v;
    if (pct < (kind === 'cpu' ? 50 : 70)) return 'ok';
    if (pct < (kind === 'cpu' ? 80 : 85)) return 'warn';
    return 'error';
  }

  function ramLabel(mb: number): string {
    return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb)} MB`;
  }

  $: unavailable = snapshot?.runtime?.state === 'unavailable';

  $: cpu = snapshot?.cpuPercent?.value;
  $: ramUsed = snapshot?.ramUsedMB?.value;
  $: ramMax = snapshot?.ramMaxMB?.value;
  $: tps = snapshot?.tps1m?.value;

  $: gauges = [
    {
      label: 'CPU',
      value: cpu !== undefined ? `${Math.round(cpu)}%` : '--',
      fraction: cpu !== undefined ? Math.min(Math.max(cpu / 100, 0), 1) : 0,
      hasData: cpu !== undefined,
      tone: cpu !== undefined ? toneFor('cpu', cpu) : 'ok',
    },
    {
      label: 'RAM',
      value: ramUsed !== undefined ? ramLabel(ramUsed) : '--',
      fraction: ramUsed !== undefined && ramMax ? Math.min(Math.max(ramUsed / ramMax, 0), 1) : 0,
      hasData: ramUsed !== undefined,
      tone: ramUsed !== undefined && ramMax ? toneFor('ram', (ramUsed / ramMax) * 100) : 'ok',
    },
    {
      label: 'TPS',
      value: tps !== undefined ? tps.toFixed(1) : '--',
      fraction: tps !== undefined ? Math.min(tps / 20, 1) : 0,
      hasData: tps !== undefined,
      tone: tps !== undefined ? toneFor('tps', tps) : 'ok',
    },
  ] satisfies Gauge[];

  const toneColor: Record<string, string> = {
    ok: 'var(--msc2-status-ok)',
    warn: 'var(--msc2-status-warn)',
    error: 'var(--msc2-status-error)',
  };
</script>

<Card padding="14px 16px">
  <div class="stats-body">
    <div class="overline">
      <Icon name="waveform" size={12} />
      <span class="msc2-type-overline">Live Stats</span>
    </div>

    {#if unavailable}
      <p class="unavailable">
        {snapshot?.runtime?.message ?? 'This runtime does not report live metrics.'}
      </p>
    {:else}
      <div class="gauges">
        {#each gauges as gauge (gauge.label)}
          <div class="gauge">
            <div class="track">
              {#if gauge.hasData && gauge.fraction > 0}
                <div
                  class="fill"
                  style="height: {gauge.fraction * 100}%; background: {toneColor[gauge.tone]};"
                ></div>
              {/if}
              <div class="readout">
                <span class="v">{gauge.value}</span>
                <span class="l">{gauge.label}</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</Card>

<style>
  .stats-body {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 12px;
    color: var(--msc2-text-tertiary);
  }
  .unavailable {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    line-height: 1.5;
  }
  .gauges {
    display: flex;
    gap: 8px;
    flex: 1;
    min-height: 0;
  }
  .gauge {
    flex: 1;
  }
  .track {
    position: relative;
    height: 100%;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
    overflow: hidden;
    display: flex;
    align-items: flex-end;
  }
  .fill {
    width: 100%;
    transition: height 300ms ease;
  }
  .readout {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .v {
    font-size: 11px;
    font-weight: 600;
    color: #fff;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.6);
  }
  .l {
    font-size: 9px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.6);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.5);
  }
</style>
