// Per-server free-text notes. MSC 1 keeps these local to the app (never sent
// to the agent), and the agent contract has no notes field, so this is the
// same "client-local until a real field exists" treatment P12.1a gave the
// sidebar avatar identity — except notes genuinely are per-server, so the
// key is host+server scoped rather than global.
function key(hostId: string, serverId: string): string {
  return `msc2.notes.${hostId}.${serverId}`;
}

export function readNotes(hostId: string, serverId: string): string {
  if (typeof localStorage === 'undefined') return '';
  return localStorage.getItem(key(hostId, serverId)) ?? '';
}

export function writeNotes(hostId: string, serverId: string, text: string): void {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(key(hostId, serverId), text);
}

export type UnresolvedModpackNote = {
  fileName: string;
  provider?: string;
  reason?: string;
  skipped?: boolean;
};

const unresolvedMarker = '[MSC unresolved modpack files]';

/** Keeps the Overview copy readable while the agent's operation state remains
 * the structured source of truth. */
export function writeUnresolvedModpackNotes(
  hostId: string,
  serverId: string,
  pending: UnresolvedModpackNote[],
  skipped: UnresolvedModpackNote[] = [],
): void {
  const current = readNotes(hostId, serverId);
  const base = current.split(unresolvedMarker)[0].trimEnd();
  const entries = [...pending, ...skipped];
  if (entries.length === 0) {
    writeNotes(hostId, serverId, base);
    return;
  }
  const lines = entries.map((entry) => {
    const status = entry.skipped ? 'skipped' : 'unresolved';
    const provider = entry.provider ? `; ${entry.provider}` : '';
    const reason = entry.reason ? ` — ${entry.reason}` : '';
    return `- ${entry.fileName} [${status}${provider}]${reason}`;
  });
  writeNotes(hostId, serverId, `${base}\n\n${unresolvedMarker}\n${lines.join('\n')}`);
}
