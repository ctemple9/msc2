import type { Schema } from '../shared/types';
import { parseChatFeed, type ChatFeedMessage } from '../home/chatFeed';

export const demoPlayers: Schema['PlayerDTO'][] = [
  { id: 'player-1', name: 'cameron', displayName: 'Cameron', level: 42 },
];

export function playerSearch(
  players: readonly Schema['PlayerDTO'][],
  query: string,
): Schema['PlayerDTO'][] {
  const needle = query.trim().toLowerCase();
  return players.filter(
    (player) => !needle || `${player.name} ${player.displayName}`.toLowerCase().includes(needle),
  );
}

export const playerPaths = {
  players: '/v1/players',
  profiles: '/v1/players/profiles',
  hidden: '/v1/players/hidden',
  skinOverride: '/v1/players/skin-override',
  skin: (profileId: string) => `/v1/players/${encodeURIComponent(profileId)}/skin`,
  delete: '/v1/players/delete',
  migrateOffline: '/v1/players/migrate-offline',
  migrate: '/v1/players/migrate',
  duplicate: '/v1/players/duplicate',
  identify: '/v1/players/identify',
  allowlist: '/v1/allowlist',
  consoleTail: '/v1/console/tail?n=200',
  sessionLog: '/v1/session-log',
  sessionLogClear: '/v1/session-log/clear',
} as const;

// ── Player Data (profiles) ──────────────────────────────────────────────

export type ProfileSortOrder = 'lastSeen' | 'nameAZ';

export function profileDisplayName(profile: Schema['PlayerProfileDTO']): string {
  if (profile.username && profile.username.length > 0) return profile.username;
  return `${profile.id.slice(0, 8)}…`;
}

export function profileSearch(
  profiles: readonly Schema['PlayerProfileDTO'][],
  query: string,
): Schema['PlayerProfileDTO'][] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [...profiles];
  return profiles.filter((profile) =>
    [profile.username, profile.id].some((value) => value?.toLowerCase().includes(needle)),
  );
}

export function profileSort(
  profiles: readonly Schema['PlayerProfileDTO'][],
  order: ProfileSortOrder,
): Schema['PlayerProfileDTO'][] {
  const sorted = [...profiles];
  if (order === 'nameAZ') {
    sorted.sort((a, b) => profileDisplayName(a).localeCompare(profileDisplayName(b)));
  } else {
    sorted.sort((a, b) => (b.lastSeen ?? '').localeCompare(a.lastSeen ?? ''));
  }
  return sorted;
}

/** mc-heads.net face-crop identifier. `PlayerProfileDTO.imageIdentifier` is
 *  already resolved server-side for both editions -- a bare uuid-hex for
 *  Java, a dotted gamertag (".Gamertag") for Bedrock, matching mc-heads.net's
 *  own documented Bedrock convention (ported server-side from
 *  PlayerProfile.imageIdentifier) -- so this always has something real to
 *  try, even for a not-yet-identified Bedrock profile (its raw XUID, same
 *  fallback the backend itself uses when no name is cached yet). */
export function avatarUrl(profile: Schema['PlayerProfileDTO'], size = 40): string {
  return `https://mc-heads.net/avatar/${encodeURIComponent(profile.imageIdentifier)}/${size}`;
}

/** Full-body render for the same identifier scheme `avatarUrl` resolves. */
export function bodyUrl(profile: Schema['PlayerProfileDTO'], size = 96): string {
  return `https://mc-heads.net/body/${encodeURIComponent(profile.imageIdentifier)}/${size}`;
}

// ── Session log — real backend (P12.3b/c) for Java, ported from MSC 1's
// SessionLogManager.swift; Java's join/leave events are written to disk by
// lifecycle.rs's live hook. Bedrock has no equivalent live-drain wiring yet
// (BedrockServiceEvent is defined but nothing drains it), so Bedrock keeps
// deriving events from /v1/console/tail via chatFeed.ts's join/leave parsing
// — the same substitute Overview's ChatCard already uses for its own feed.
// Both sources normalize to the same SessionEvent shape so SessionLogCard
// doesn't need to know which one it's looking at. ─────────────────────────

export type SessionEvent = { id: string; player: string; kind: 'join' | 'leave'; ts: string };

/** Maps the real backend's SessionEventDTO (Java only, P12.3c) to the same
 *  shape sessionEventsFromConsole produces for Bedrock's console-tail path. */
export function sessionEventsFromLog(events: readonly Schema['SessionEventDTO'][]): SessionEvent[] {
  return events.map((event) => ({
    id: event.id,
    player: event.playerName,
    kind: event.eventType === 'joined' ? 'join' : 'leave',
    ts: event.timestamp,
  }));
}

export function sessionEventsFromConsole(
  lines: readonly Schema['ConsoleLineDTO'][],
): SessionEvent[] {
  return parseChatFeed(lines)
    .filter(
      (message): message is ChatFeedMessage & { kind: 'join' | 'leave'; player: string } =>
        (message.kind === 'join' || message.kind === 'leave') && message.player !== null,
    )
    .map((message) => ({
      id: message.id,
      player: message.player,
      kind: message.kind,
      ts: message.ts,
    }));
}

export function seenThisSession(events: readonly SessionEvent[]): string[] {
  const seen: string[] = [];
  for (const event of events) {
    if (!seen.includes(event.player)) seen.push(event.player);
  }
  return seen;
}

export function filterSessionEvents(
  events: readonly SessionEvent[],
  query: string,
): SessionEvent[] {
  const needle = query.trim().toLowerCase();
  return needle
    ? events.filter((event) => event.player.toLowerCase().includes(needle))
    : [...events];
}

export type SessionDay = { day: string; events: SessionEvent[] };

/** Groups events by calendar day (local time), most recent day first, events
 *  within a day in their original (chronological) order. */
export function groupSessionEventsByDay(events: readonly SessionEvent[]): SessionDay[] {
  const byDay = new Map<string, SessionEvent[]>();
  for (const event of events) {
    const day = new Date(event.ts).toDateString();
    const bucket = byDay.get(day);
    if (bucket) bucket.push(event);
    else byDay.set(day, [event]);
  }
  return [...byDay.entries()]
    .sort((a, b) => new Date(b[0]).getTime() - new Date(a[0]).getTime())
    .map(([day, dayEvents]) => ({ day, events: dayEvents }));
}

/** For a join event, the elapsed time until that player's next leave event
 *  (or "now" if still online) — matches MSC 1's inline session-duration label. */
export function sessionDurationLabel(
  event: SessionEvent,
  allEvents: readonly SessionEvent[],
  isOnline: boolean,
): string | null {
  if (event.kind !== 'join') return null;
  const startedAt = new Date(event.ts).getTime();
  const next = allEvents.find(
    (candidate) =>
      candidate.player === event.player &&
      candidate.kind === 'leave' &&
      new Date(candidate.ts).getTime() >= startedAt,
  );
  const endedAt = next ? new Date(next.ts).getTime() : isOnline ? Date.now() : null;
  if (endedAt === null) return null;
  const minutes = Math.max(0, Math.round((endedAt - startedAt) / 60000));
  if (minutes < 1) return '<1m';
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

// ── Client-local "clear log" — Bedrock-only fallback. Java's Clear Log now
// calls the real POST /v1/session-log/clear (P12.3a/c), which actually
// deletes the events. Bedrock still derives from the console tail, a bounded
// recent buffer with nothing durable on the agent to send a clear mutation
// to, so it keeps the client-side cutoff-timestamp treatment notes.ts
// already gives other per-server state with no backend field. ────────────

function clearedKey(hostId: string, serverId: string): string {
  return `msc2.sessionLogClearedAt.${hostId}.${serverId}`;
}

export function readSessionLogClearedAt(hostId: string, serverId: string): number {
  if (typeof localStorage === 'undefined') return 0;
  return Number(localStorage.getItem(clearedKey(hostId, serverId)) ?? 0);
}

export function clearSessionLog(hostId: string, serverId: string): void {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(clearedKey(hostId, serverId), String(Date.now()));
}
