<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, bytesLabel, dateLabel } from '../shared/types';

  export let api: ScreenProps['api'] = undefined;
  let snapshot: Schema['PerformanceSnapshotDTO'] = { ts: new Date().toISOString() };
  let refreshed = false;

  onMount(async () => {
    snapshot = await call(api, snapshot, '/v1/performance');
  });

  function metric(value: Schema['PerformanceMetricNumberDTO'] | undefined): string {
    return value ? `${value.value}` : '—';
  }
  function bar(value: Schema['PerformanceMetricNumberDTO'] | undefined, maximum: number): number {
    return Math.min(100, Math.max(0, ((value?.value ?? 0) / maximum) * 100));
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Live metrics"
    title="Performance"
    description="Small DOM/SVG-friendly charts keep performance useful on WebKitGTK and low-power hosts."
    actionLabel="Refresh"
    onAction={async () => {
      snapshot = await call(api, snapshot, '/v1/performance');
      refreshed = true;
    }}
    status={snapshot.runtime?.state === 'unavailable' ? 'Unavailable' : 'Snapshot ready'}
    statusTone={snapshot.runtime?.state === 'unavailable' ? 'warning' : 'positive'}
  />
  {#if snapshot.runtime?.state === 'unavailable'}<CapabilityNotice
      title="Runtime metrics unavailable"
      message={snapshot.runtime.message ?? 'The selected runtime cannot provide live metrics.'}
      helpId={snapshot.runtime.helpId}
    />{/if}
  <div class="screen-grid three">
    <section class="screen-card">
      <span class="metric-label">CPU</span>
      <p class="metric-large">{metric(snapshot.cpuPercent)}%</p>
      <div class="progress-bar" aria-label="CPU usage">
        <span style={`width: ${bar(snapshot.cpuPercent, 100)}%`}></span>
      </div>
    </section>
    <section class="screen-card">
      <span class="metric-label">TPS · 1 minute</span>
      <p class="metric-large">{metric(snapshot.tps1m)}</p>
      <div class="progress-bar" aria-label="TPS">
        <span style={`width: ${bar(snapshot.tps1m, 20)}%`}></span>
      </div>
    </section>
    <section class="screen-card">
      <span class="metric-label">Players online</span>
      <p class="metric-large">{snapshot.playersOnline ?? 0}</p>
      <p class="muted">Updated {dateLabel(snapshot.ts)}</p>
    </section>
  </div>
  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Memory and world size</h3>
      <span class="metric-label">Low-cost chart</span>
    </div>
    <div class="screen-grid">
      <div>
        <p class="metric-label">RAM used / max</p>
        <p class="metric-large">{metric(snapshot.ramUsedMB)} / {metric(snapshot.ramMaxMB)} MB</p>
        <div class="progress-bar">
          <span style={`width: ${bar(snapshot.ramUsedMB, snapshot.ramMaxMB?.value || 1)}%`}></span>
        </div>
      </div>
      <div>
        <p class="metric-label">World size</p>
        <p class="metric-large">{bytesLabel((snapshot.worldSizeMB?.value ?? 0) * 1024 ** 2)}</p>
        <p class="muted">
          Charts use regular elements and remain readable without GPU acceleration.
        </p>
      </div>
    </div>
  </section>
  {#if refreshed}<p class="muted" role="status">Performance snapshot refreshed.</p>{/if}
</div>
