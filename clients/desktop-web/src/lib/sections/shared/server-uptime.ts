import { writable } from 'svelte/store';

type Run = { running: boolean; startedAt?: number };
export const serverRuns = writable<ReadonlyMap<string, Run>>(new Map());

export function serverRunKey(hostId: string, serverId: string): string {
  return JSON.stringify([hostId, serverId]);
}

// An already-running server has an unknown start time. Only a witnessed
// stopped-to-running transition can establish elapsed uptime in this client.
export function observeServerRun(hostId: string, serverId: string, running: boolean): void {
  if (!serverId) return;
  const key = serverRunKey(hostId, serverId);
  serverRuns.update((runs) => {
    const previous = runs.get(key);
    if (previous?.running === running) return runs;
    const next = new Map(runs);
    next.set(key, {
      running,
      startedAt: running && previous?.running === false ? Date.now() : undefined,
    });
    return next;
  });
}

export function forgetHostRuns(hostId: string): void {
  serverRuns.update((runs) => {
    const next = new Map(runs);
    for (const key of runs.keys()) {
      if ((JSON.parse(key) as [string, string])[0] === hostId) next.delete(key);
    }
    return next.size === runs.size ? runs : next;
  });
}
