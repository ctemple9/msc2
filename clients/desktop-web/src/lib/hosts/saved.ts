import type { HostRecord } from './types';

/**
 * Remote host metadata is useful to remember, but it is not a credential.
 * Bearer credentials remain in the native desktop secret store; this file only
 * keeps the label and address needed to rediscover a saved host after restart.
 */
export const SAVED_REMOTE_HOSTS_KEY = 'msc2.saved-remote-hosts';

export function loadSavedRemoteHosts(): HostRecord[] {
  if (typeof localStorage === 'undefined') return [];
  const raw = localStorage.getItem(SAVED_REMOTE_HOSTS_KEY);
  if (!raw) return [];

  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];

    const seen = new Set<string>();
    return parsed.filter((value): value is HostRecord => {
      if (!isHostRecord(value) || seen.has(value.id) || value.id === 'local-agent') return false;
      seen.add(value.id);
      return true;
    });
  } catch {
    return [];
  }
}

export function saveRemoteHost(host: HostRecord): void {
  if (typeof localStorage === 'undefined') return;
  const hosts = loadSavedRemoteHosts().filter((saved) => saved.id !== host.id);
  hosts.push({ id: host.id, label: host.label, baseUrl: host.baseUrl });
  localStorage.setItem(SAVED_REMOTE_HOSTS_KEY, JSON.stringify(hosts));
}

export function forgetSavedRemoteHost(hostId: string): void {
  if (typeof localStorage === 'undefined') return;
  const hosts = loadSavedRemoteHosts().filter((host) => host.id !== hostId);
  localStorage.setItem(SAVED_REMOTE_HOSTS_KEY, JSON.stringify(hosts));
}

function isHostRecord(value: unknown): value is HostRecord {
  if (!value || typeof value !== 'object') return false;
  const record = value as Record<string, unknown>;
  if (
    typeof record.id !== 'string' ||
    typeof record.label !== 'string' ||
    typeof record.baseUrl !== 'string' ||
    !record.id.trim() ||
    !record.label.trim()
  ) {
    return false;
  }
  try {
    new URL(record.baseUrl);
    return true;
  } catch {
    return false;
  }
}
