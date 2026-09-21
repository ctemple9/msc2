// Legacy migration only. New notes are owned by the agent and travel through
// the server API, so localStorage must not remain a second source of truth.
function legacyKey(hostId: string, serverId: string): string {
  return `msc2.notes.${hostId}.${serverId}`;
}

export function readLegacyNotes(hostId: string, serverId: string): string {
  if (typeof localStorage === 'undefined') return '';
  return localStorage.getItem(legacyKey(hostId, serverId)) ?? '';
}

export function clearLegacyNotes(hostId: string, serverId: string): void {
  if (typeof localStorage === 'undefined') return;
  localStorage.removeItem(legacyKey(hostId, serverId));
}

/** Preserve both records when another client has already written host notes. */
export function mergeLegacyNotes(serverNotes: string, legacyNotes: string): string {
  const current = serverNotes.trimEnd();
  const legacy = legacyNotes.trim();
  if (!legacy || current === legacy || current.endsWith(`\n${legacy}`)) return serverNotes;
  return current ? `${current}\n\n${legacy}` : legacy;
}
