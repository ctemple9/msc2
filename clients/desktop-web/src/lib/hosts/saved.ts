import { LOCAL_HOST_ID, normalizeHostRecord, type HostRecord } from './types';

/**
 * Remote host metadata is useful to remember, but it is not a credential.
 * Bearer credentials remain in the native desktop secret store; this file only
 * keeps the editable profile needed to rediscover a saved host after restart.
 */
export const SAVED_REMOTE_HOSTS_KEY = 'msc2.saved-remote-hosts';
export const SAVED_REMOTE_HOSTS_SCHEMA_VERSION = 2;

interface SavedHostsEnvelope {
  readonly version: typeof SAVED_REMOTE_HOSTS_SCHEMA_VERSION;
  readonly hosts: readonly HostRecord[];
}

export function loadSavedRemoteHosts(): HostRecord[] {
  if (typeof localStorage === 'undefined') return [];
  const raw = localStorage.getItem(SAVED_REMOTE_HOSTS_KEY);
  if (!raw) return [];

  try {
    const parsed: unknown = JSON.parse(raw);
    const entries = Array.isArray(parsed)
      ? parsed
      : isSavedHostsEnvelope(parsed)
        ? parsed.hosts
        : [];

    const seen = new Set<string>();
    return entries.reduce<HostRecord[]>((hosts, value) => {
      const host = normalizeHostRecord(value);
      if (!host || host.id === LOCAL_HOST_ID || seen.has(host.id)) return hosts;
      seen.add(host.id);
      hosts.push(host);
      return hosts;
    }, []);
  } catch {
    return [];
  }
}

export function saveRemoteHost(host: HostRecord): void {
  if (typeof localStorage === 'undefined') return;
  const normalized = normalizeHostRecord(host);
  if (!normalized || normalized.id === LOCAL_HOST_ID) {
    throw new Error('Only a valid remote host profile can be saved.');
  }
  const hosts = loadSavedRemoteHosts().filter((saved) => saved.id !== host.id);
  hosts.push(normalized);
  const saved: SavedHostsEnvelope = {
    version: SAVED_REMOTE_HOSTS_SCHEMA_VERSION,
    hosts,
  };
  localStorage.setItem(SAVED_REMOTE_HOSTS_KEY, JSON.stringify(saved));
}

export function forgetSavedRemoteHost(hostId: string): void {
  if (typeof localStorage === 'undefined') return;
  const hosts = loadSavedRemoteHosts().filter((host) => host.id !== hostId);
  const saved: SavedHostsEnvelope = {
    version: SAVED_REMOTE_HOSTS_SCHEMA_VERSION,
    hosts,
  };
  localStorage.setItem(SAVED_REMOTE_HOSTS_KEY, JSON.stringify(saved));
}

function isSavedHostsEnvelope(value: unknown): value is SavedHostsEnvelope {
  return (
    !!value &&
    typeof value === 'object' &&
    (value as Record<string, unknown>).version === SAVED_REMOTE_HOSTS_SCHEMA_VERSION &&
    Array.isArray((value as Record<string, unknown>).hosts)
  );
}
