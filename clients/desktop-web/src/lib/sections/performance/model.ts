// Pure helpers for the Performance tab. Thresholds ported from MSC 1's
// DetailsPerformanceHelpers.swift (tpsHealthStatus/cpuHealthStatus/
// ramHealthStatus) -- same cutoffs, remapped onto the shared MSC 2 status
// ramp (ok/warn/error) instead of MSC 1's own MetricStatus enum.

export type Tone = 'ok' | 'warn' | 'error' | undefined;

export function tpsTone(tps: number | undefined): Tone {
  if (tps === undefined) return undefined;
  if (tps >= 19.5) return 'ok';
  if (tps >= 18) return 'warn';
  return 'error';
}

export function cpuTone(percent: number | undefined): Tone {
  if (percent === undefined) return undefined;
  if (percent < 70) return 'ok';
  if (percent < 90) return 'warn';
  return 'error';
}

export function ramTone(usedMb: number | undefined, maxMb: number | undefined): Tone {
  if (usedMb === undefined || !maxMb) return undefined;
  const percent = (usedMb / maxMb) * 100;
  if (percent < 75) return 'ok';
  if (percent < 90) return 'warn';
  return 'error';
}

export function toneLabel(tone: Tone): string {
  if (tone === 'ok') return 'Good';
  if (tone === 'warn') return 'Warning';
  if (tone === 'error') return 'Critical';
  return '—';
}

export function formatTps(value: number | undefined): string {
  return value === undefined ? '—' : value.toFixed(2);
}

export function formatPercent(value: number | undefined): string {
  return value === undefined ? '—' : `${Math.round(value)}%`;
}

export function formatRamCompact(usedMb: number | undefined): string {
  return usedMb === undefined ? '—' : `${(usedMb / 1024).toFixed(1)} GB`;
}

export function formatRamSubtitle(maxMb: number | undefined): string {
  return maxMb === undefined ? 'Heap' : `of ${(maxMb / 1024).toFixed(0)} GB`;
}

/** MSC 1 AppViewModel+ServerControls.swift's updateUptimeDisplay, ported
 *  from a TimeInterval to milliseconds since Date.now(). */
export function formatUptime(elapsedMs: number | undefined): string {
  if (elapsedMs === undefined) return 'Offline';
  const totalMinutes = Math.floor(elapsedMs / 60000);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours >= 48) {
    const days = Math.floor(hours / 24);
    return `${days}d ${hours % 24}h`;
  }
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m`;
  return 'Just started';
}

/** Bedrock's Load 1m/5m/15m is a client-side rolling average over a raw CPU%
 *  history buffer (MSC 1 AppViewModel+BedrockPerformance.swift's
 *  `rollingAverage(from:sampleCount:)`), not a distinct backend field --
 *  the agent only ever reports the instantaneous cpuPercent, sampled here
 *  once per poll. `count` is the number of POLL_INTERVAL_MS-spaced samples
 *  the window covers (12/60/180 at the 5s cadence below == 1m/5m/15m). */
export function rollingAverage(history: readonly number[], count: number): number | undefined {
  if (history.length === 0) return undefined;
  const slice = history.slice(-count);
  return slice.reduce((sum, value) => sum + value, 0) / slice.length;
}

/** Matches the Monitoring rail's "Active (5s)" label and the 12/60/180
 *  sample windows above; also this screen's own /v1/performance poll rate. */
export const POLL_INTERVAL_MS = 5000;
/** 180 samples * 5s == 15 minutes, the longest rolling window shown. */
export const HISTORY_CAPACITY = 180;
/** MSC 1 caps its charted (as opposed to averaged) histories at 30 samples. */
export const CHART_CAPACITY = 30;

export function pushCapped(history: readonly number[], value: number, cap: number): number[] {
  const next = [...history, value];
  return next.length > cap ? next.slice(next.length - cap) : next;
}
