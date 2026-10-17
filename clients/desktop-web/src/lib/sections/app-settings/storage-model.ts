import type { StorageSnapshot, StorageItem, StorageResult } from '../../platform/storage';
export const categories = [
  ['servers', 'Servers'],
  ['desktopApp', 'Desktop app'],
  ['agent', 'Agent'],
  ['sharedTools', 'Shared tools & runtimes'],
  ['cache', 'Cache & temporary files'],
] as const;
export function total(items: StorageItem[]): number {
  return items.reduce((sum, item) => sum + item.logicalBytes, 0);
}
export function bySize(items: StorageItem[]): StorageItem[] {
  return [...items].sort(
    (a, b) => b.logicalBytes - a.logicalBytes || a.label.localeCompare(b.label),
  );
}
export function grouped(
  snapshot: StorageSnapshot,
  key: 'category' | 'serverId' | 'group',
  items = snapshot.items,
): { id: string; label: string; bytes: number }[] {
  const groups = new Map<string, { id: string; label: string; bytes: number }>();
  for (const item of items) {
    const id = item[key] ?? 'unassigned';
    const label = key === 'serverId' ? item.label.split(' / ')[0] : id;
    const group = groups.get(id) ?? { id, label, bytes: 0 };
    group.bytes += item.logicalBytes;
    groups.set(id, group);
  }
  return [...groups.values()].sort((a, b) => b.bytes - a.bytes || a.label.localeCompare(b.label));
}
export interface PendingCleanup {
  hostId: string;
  name: string;
  operationId: string;
}
const pendingKey = 'msc2.storage-pending-cleanups';
export function pendingCleanups(): PendingCleanup[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(pendingKey) ?? '[]');
    if (!Array.isArray(parsed)) return [];
    return parsed
      .slice(0, 64)
      .filter(
        (p): p is PendingCleanup =>
          !!p &&
          typeof p.hostId === 'string' &&
          typeof p.name === 'string' &&
          typeof p.operationId === 'string',
      );
  } catch {
    return [];
  }
}
export function rememberCleanup(pending: PendingCleanup): void {
  try {
    localStorage.setItem(
      pendingKey,
      JSON.stringify(
        [...pendingCleanups().filter((p) => p.hostId !== pending.hostId), pending].slice(-64),
      ),
    );
  } catch {
    /* The operation remains addressable by its displayed ID. */
  }
}
export const recentResults = new Map<string, StorageResult>();

export function forgetCleanup(hostId: string): void {
  try {
    localStorage.setItem(
      pendingKey,
      JSON.stringify(pendingCleanups().filter((p) => p.hostId !== hostId)),
    );
  } catch {
    /* Stale metadata is reconciled from its journal result. */
  }
}
