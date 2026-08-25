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
