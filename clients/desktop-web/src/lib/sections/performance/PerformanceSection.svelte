<script lang="ts">
  // MSC 1 DetailsPerformanceTabView / DetailsPerformanceTabContent.swift,
  // rebuilt to the S0 disciplined system (docs/msc2/antiAIslop.md). Same
  // shared-component pattern HomeSection/WorldsSection use (D-003).
  //
  // Two real gaps found while wiring this, handled like P12.4a-e/f's
  // backend-drift fixes rather than faked or silently skipped:
  //  1. TPS 5m/15m: msc-domain's /tps parser already extracts Paper's real
  //     t5/t15 rolling averages (crates/msc-domain/src/tps.rs), and the
  //     lifecycle layer already kept them in `latest_tps: Option<Sample>` --
  //     but `PerformanceSnapshot`/`PerformanceSnapshotDTO` only ever
  //     exposed t1, silently dropping t5/t15 before they reached the wire.
  //     Added `tps5m`/`tps15m` to the contract, DTO, and both snapshot
  //     constructors (msc-application/src/lifecycle.rs,
  //     msc-agent/src/routes/{lifecycle,status,performance}.rs) so the
  //     TPS (5m avg)/(15m avg) tiles show the same real figures MSC 1 does,
  //     instead of a fabricated or honestly-blank pair of headline cards.
  //  2. Bedrock's Load 1m/5m/15m and both "Over Time" charts are NOT
  //     separate backend fields even in MSC 1 -- they're client-side
  //     rolling windows over repeatedly-polled instantaneous values
  //     (AppViewModel+BedrockPerformance.swift's bedrockCpuHistory /
  //     rollingAverage, AppViewModel+OutputHandling.swift's
  //     tpsHistory1m/playerCountHistory). Reproduced the same way here,
  //     purely client-side (see model.ts) -- no backend gap.
  //
  // Uptime is one deliberate departure from a literal port, not an
  // oversight: MSC 1 only ever sets `serverStartTime` when its own running
  // app process issues the Start action, because MSC 1's server is a child
  // process of the app itself -- a client can never observe an
  // already-running server it didn't start. MSC 2's agent is a persistent
  // background service multiple clients can connect to after the fact
  // (Phase 9/11), so porting that guard literally would show "Offline" in
  // the Uptime tile while Status says "Online" for any client that simply
  // reconnected. Instead: uptime is tracked from the moment *this* client
  // session observes a real not-running -> running transition; if the
  // server was already running when this session first loaded, the tile
  // reads "Running" (no fabricated duration) rather than "Offline" or a
  // guessed number.
  import { onDestroy, onMount } from 'svelte';
  import Icon from '../../components/base/Icon.svelte';
  import MetricTile from './MetricTile.svelte';
  import PerformanceChart from './PerformanceChart.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { bytesLabel, call } from '../shared/types';
  import {
    CHART_CAPACITY,
    HISTORY_CAPACITY,
    RAM_POLL_INTERVAL_MS,
    POLL_INTERVAL_MS,
    cpuTone,
    formatPercent,
    formatRamCompact,
    formatRamSubtitle,
    formatTps,
    formatUptime,
    pushCapped,
    ramTone,
    rollingAverage,
    tpsTone,
  } from './model';

  export let api: ScreenProps['api'] = undefined;
  export let hostId = 'local-agent';
  export let serverId = 'survival';
  export let active = true;

  let snapshot: Schema['PerformanceSnapshotDTO'] = { ts: new Date().toISOString() };
  let health: Schema['HealthResponseDTO'] = {
    cards: [],
    overallSeverity: 'gray',
    serverName: '',
    serverRunning: false,
    serverType: '',
  };
  let servers: Schema['ServerDTO'][] = [];

  $: activeServer = servers.find((server) => server.id === serverId);
  $: isBedrock = activeServer?.serverType === 'bedrock';

  let cpuHistory: number[] = [];
  let tpsChartHistory: number[] = [];
  let ramHistory: number[] = [];

  let runStartMs: number | undefined;
  let previousRunning: boolean | undefined;
  let nowMs = Date.now();

  async function poll(): Promise<void> {
    snapshot = await call(api, snapshot, '/v1/performance');
    health = await call(api, health, '/v1/health');
    if (servers.length === 0) servers = await call(api, servers, '/v1/servers');

    const cpu = snapshot.cpuPercent?.value;
    if (cpu !== undefined) cpuHistory = pushCapped(cpuHistory, cpu, HISTORY_CAPACITY);
    const tps = snapshot.tps1m?.value;
    if (tps !== undefined) tpsChartHistory = pushCapped(tpsChartHistory, tps, CHART_CAPACITY);
    recordRamSample();

    if (health.serverRunning !== previousRunning) {
      if (health.serverRunning && previousRunning === false) runStartMs = Date.now();
      if (!health.serverRunning) runStartMs = undefined;
      previousRunning = health.serverRunning;
    }
  }

  async function pollRam(): Promise<void> {
    const nextSnapshot = await call(api, snapshot, '/v1/performance');
    snapshot = {
      ...snapshot,
      ts: nextSnapshot.ts,
      ramUsedMB: nextSnapshot.ramUsedMB,
      ramMaxMB: nextSnapshot.ramMaxMB,
    };
    recordRamSample();
  }

  let pollTimer: ReturnType<typeof setInterval> | undefined;
  let ramPollTimer: ReturnType<typeof setInterval> | undefined;
  let clockTimer: ReturnType<typeof setInterval> | undefined;
  let mounted = false;

  let lastRamSampleMs = 0;

  function recordRamSample(): void {
    const now = Date.now();
    if (now - lastRamSampleMs < RAM_POLL_INTERVAL_MS) return;
    lastRamSampleMs = now;
    const ram = snapshot.ramUsedMB?.value;
    if (ram !== undefined) ramHistory = pushCapped(ramHistory, ram, CHART_CAPACITY);
  }

  function startPolling(): void {
    void poll();
    pollTimer = setInterval(() => void poll(), POLL_INTERVAL_MS);
    ramPollTimer = setInterval(() => void pollRam(), RAM_POLL_INTERVAL_MS);
  }
  function stopPolling(): void {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = undefined;
    if (ramPollTimer) clearInterval(ramPollTimer);
    ramPollTimer = undefined;
  }

  onMount(() => {
    mounted = true;
    if (active) {
      startPolling();
      clockTimer = setInterval(() => (nowMs = Date.now()), 1000);
    }
  });
  onDestroy(() => {
    mounted = false;
    stopPolling();
    if (clockTimer) clearInterval(clockTimer);
  });

  $: if (mounted && active && pollTimer === undefined) {
    startPolling();
    clockTimer = setInterval(() => (nowMs = Date.now()), 1000);
  }
  $: if (mounted && !active) {
    stopPolling();
    if (clockTimer) clearInterval(clockTimer);
    clockTimer = undefined;
  }

  // Bedrock Load 1m/5m/15m: rolling averages over the same repeatedly-
  // polled cpuPercent (12/60/180 samples at the 5s poll cadence == 1m/5m/15m).
  $: load1m = rollingAverage(cpuHistory, 12);
  $: load5m = rollingAverage(cpuHistory, 60);
  $: load15m = rollingAverage(cpuHistory, 180);

  $: tps1m = snapshot.tps1m?.value;
  $: tps5m = snapshot.tps5m?.value;
  $: tps15m = snapshot.tps15m?.value;

  $: cpuValue = snapshot.cpuPercent?.value;
  $: ramUsed = snapshot.ramUsedMB?.value;
  $: ramMax = snapshot.ramMaxMB?.value;

  $: uptimeMs = health.serverRunning && runStartMs !== undefined ? nowMs - runStartMs : undefined;
  $: uptimeValue = health.serverRunning
    ? runStartMs !== undefined
      ? formatUptime(uptimeMs)
      : 'Running'
    : 'Offline';

  $: cpuDomainMax = Math.max(25, Math.ceil(Math.max(...cpuHistory, 0) / 25) * 25);
  $: ramDomainMax = Math.max(1024, ramMax ?? 0, ...ramHistory);

  const toneColor: Record<'ok' | 'warn' | 'error', string> = {
    ok: 'var(--msc2-status-ok)',
    warn: 'var(--msc2-status-warn)',
    error: 'var(--msc2-status-error)',
  };
  function css(tone: 'ok' | 'warn' | 'error' | undefined): string {
    return tone ? `color: ${toneColor[tone]};` : '';
  }
</script>

<div class="performance">
  <div class="main">
    {#if snapshot.runtime?.state === 'unavailable'}
      <p class="unavailable">
        {snapshot.runtime.message ?? 'The selected runtime cannot provide live metrics.'}
      </p>
    {/if}

    <div class="metrics-grid">
      {#if isBedrock}
        <MetricTile
          icon="waveform"
          title="Load (1m)"
          value={formatPercent(load1m)}
          tone={cpuTone(load1m)}
          subtitle="Rolling average"
        />
        <MetricTile
          icon="waveform"
          title="Load (5m avg)"
          value={formatPercent(load5m)}
          tone={cpuTone(load5m)}
          subtitle="Medium-term"
        />
        <MetricTile
          icon="waveform"
          title="Load (15m avg)"
          value={formatPercent(load15m)}
          tone={cpuTone(load15m)}
          subtitle="Long-term health"
        />
      {:else}
        <MetricTile
          icon="waveform"
          title="TPS (1m)"
          value={formatTps(tps1m)}
          tone={tpsTone(tps1m)}
          subtitle="Target: 20.00"
        />
        <MetricTile
          icon="waveform"
          title="TPS (5m avg)"
          value={formatTps(tps5m)}
          tone={tpsTone(tps5m)}
          subtitle="Medium-term"
        />
        <MetricTile
          icon="waveform"
          title="TPS (15m avg)"
          value={formatTps(tps15m)}
          tone={tpsTone(tps15m)}
          subtitle="Long-term health"
        />
      {/if}
      <MetricTile
        icon="people"
        title="Players"
        value={`${snapshot.playersOnline ?? 0}`}
        subtitle="Currently online"
      />
      <MetricTile
        icon="chip"
        title="CPU Usage"
        value={formatPercent(cpuValue)}
        tone={cpuTone(cpuValue)}
        subtitle={isBedrock ? 'Bedrock runtime' : 'Java process'}
      />
      <MetricTile
        icon="box"
        title="Memory"
        value={formatRamCompact(ramUsed)}
        tone={ramTone(ramUsed, ramMax)}
        subtitle={formatRamSubtitle(ramMax)}
      />
    </div>

    <div class="charts-row">
      <div class="chart-panel">
        <div class="panel-header">
          <Icon name="waveform" size={13} />
          <span class="panel-title">{isBedrock ? 'CPU Over Time' : 'TPS Over Time'}</span>
          <span class="panel-value" style={css(isBedrock ? cpuTone(cpuValue) : tpsTone(tps1m))}>
            {isBedrock ? formatPercent(cpuValue) : formatTps(tps1m)}
          </span>
        </div>
        {#if isBedrock}
          <PerformanceChart
            samples={cpuHistory}
            domainMax={cpuDomainMax}
            color={toneColor[cpuTone(cpuValue) ?? 'ok']}
            referenceValue={70}
            valueLabel={(v) => `${Math.round(v)}%`}
            sampleIntervalMs={POLL_INTERVAL_MS}
            emptyIcon="waveform"
            emptyMessage="Start server to collect Docker metrics"
          />
        {:else}
          <PerformanceChart
            samples={tpsChartHistory}
            domainMax={22}
            color={toneColor[tpsTone(tps1m) ?? 'ok']}
            referenceValue={20}
            valueLabel={(v) => `${Math.round(v)}`}
            sampleIntervalMs={POLL_INTERVAL_MS}
            emptyIcon="waveform"
            emptyMessage="Start server to collect TPS data"
          />
        {/if}
      </div>

      <div class="chart-panel">
        <div class="panel-header">
          <Icon name="box" size={13} />
          <span class="panel-title">Memory Over Time</span>
          <span class="panel-value" style={css(ramTone(ramUsed, ramMax))}>
            {formatRamCompact(ramUsed)}
          </span>
        </div>
        <PerformanceChart
          samples={ramHistory}
          domainMax={ramDomainMax}
          color={toneColor[ramTone(ramUsed, ramMax) ?? 'ok']}
          referenceValue={ramMax}
          valueLabel={(v) => `${(v / 1024).toFixed(1)} GB`}
          sampleIntervalMs={RAM_POLL_INTERVAL_MS}
          emptyIcon="box"
          emptyMessage="Collecting memory data"
        />
      </div>
    </div>

    <div class="footer-grid">
      <MetricTile
        icon="world"
        title="World Size"
        value={bytesLabel((snapshot.worldSizeMB?.value ?? 0) * 1024 ** 2)}
        subtitle="3 dimensions"
      />
      <MetricTile
        icon="clock"
        title="Uptime"
        value={uptimeValue}
        subtitle={health.serverRunning ? 'Since start' : ''}
      />
      <MetricTile
        icon="seal-check"
        title="Status"
        value={health.serverRunning ? 'Online' : 'Offline'}
        tone={health.serverRunning ? 'ok' : 'error'}
        subtitle={health.serverRunning
          ? isBedrock
            ? 'Bedrock runtime active'
            : 'Accepting connections'
          : ''}
      />
    </div>
  </div>
</div>

<style>
  .performance {
    width: 100%;
  }
  .main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .unavailable {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .metrics-grid,
  .footer-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  .charts-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .chart-panel {
    background: var(--msc2-tier-content);
    border-radius: 12px;
    box-shadow: var(--msc2-shadow-card);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .panel-header {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .panel-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .panel-value {
    margin-left: auto;
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }

  @media (max-width: 900px) {
    .metrics-grid,
    .footer-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .charts-row {
      grid-template-columns: 1fr;
    }
  }
</style>
