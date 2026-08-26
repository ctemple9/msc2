import { describe, expect, it } from 'vitest';
import {
  avatarUrl,
  bodyUrl,
  demoPlayers,
  filterSessionEvents,
  groupSessionEventsByDay,
  playerPaths,
  playerSearch,
  profileDisplayName,
  profileSearch,
  profileSort,
  seenThisSession,
  sessionDurationLabel,
  sessionEventsFromConsole,
  type SessionEvent,
} from '../../src/lib/sections/players-online/model';
import type { Schema } from '../../src/lib/sections/shared/types';

describe('online roster', () => {
  it('searches the generic online roster', () => {
    expect(playerSearch(demoPlayers, 'cam')).toHaveLength(1);
    expect(playerPaths.players).toBe('/v1/players');
  });
});

describe('player data (profiles)', () => {
  const profiles: Schema['PlayerProfileDTO'][] = [
    {
      id: '11111111-1111-4111-8111-111111111111',
      username: 'Alice',
      imageIdentifier: '11111111111141118111111111111111',
      isOnline: true,
      isOp: false,
      isBedrockPlayer: false,
      inventory: [],
      lastSeen: '2026-08-20T00:00:00.000Z',
    },
    {
      id: '22222222-2222-4222-8222-222222222222',
      username: 'Bob',
      imageIdentifier: '22222222222242228222222222222222',
      isOnline: false,
      isOp: false,
      isBedrockPlayer: false,
      inventory: [],
      lastSeen: '2026-08-25T00:00:00.000Z',
    },
    {
      id: '33333333-3333-4333-8333-333333333333',
      imageIdentifier: '33333333333343338333333333333333',
      isOnline: false,
      isOp: false,
      isBedrockPlayer: false,
      inventory: [],
    },
  ];

  it('uses /v1/players/profiles now that the backend serves it (P12.2b-j)', () => {
    expect(playerPaths.profiles).toBe('/v1/players/profiles');
  });

  it('has the Bedrock identify route now that the backend serves it (P12.3d)', () => {
    expect(playerPaths.identify).toBe('/v1/players/identify');
  });

  it('falls back to a truncated id when username is unresolved', () => {
    expect(profileDisplayName(profiles[2])).toBe('33333333…');
  });

  it('searches by username or id', () => {
    expect(profileSearch(profiles, 'alice')).toHaveLength(1);
    expect(profileSearch(profiles, '22222222')).toHaveLength(1);
    expect(profileSearch(profiles, '')).toHaveLength(3);
  });

  it('sorts by last-seen (most recent first) and by name A-Z', () => {
    expect(profileSort(profiles, 'lastSeen').map((p) => p.id)).toEqual([
      profiles[1].id,
      profiles[0].id,
      profiles[2].id,
    ]);
    expect(profileSort(profiles, 'nameAZ').map((p) => profileDisplayName(p))).toEqual([
      '33333333…',
      'Alice',
      'Bob',
    ]);
  });

  it('resolves an avatar/body URL from imageIdentifier for both editions -- Bedrock is not a special case', () => {
    const bedrockProfile: Schema['PlayerProfileDTO'] = {
      id: 'xuid_2535416409816137',
      username: 'camkage',
      imageIdentifier: '.camkage',
      isOnline: false,
      isOp: false,
      isBedrockPlayer: true,
      inventory: [],
    };
    expect(avatarUrl(profiles[0])).toBe(
      'https://mc-heads.net/avatar/11111111111141118111111111111111/40',
    );
    expect(bodyUrl(profiles[0], 96)).toBe(
      'https://mc-heads.net/body/11111111111141118111111111111111/96',
    );
    // mc-heads.net's documented Bedrock convention: a dotted gamertag, already
    // resolved server-side into imageIdentifier (PlayerProfile.imageIdentifier).
    expect(avatarUrl(bedrockProfile)).toBe('https://mc-heads.net/avatar/.camkage/40');
    expect(bodyUrl(bedrockProfile)).toBe('https://mc-heads.net/body/.camkage/96');
  });
});

describe('session log (derived from console tail, /v1/session-log has no agent handler yet)', () => {
  const lines: Schema['ConsoleLineDTO'][] = [
    {
      text: '[12:00:00] [Server thread/INFO]: Alice joined the game',
      ts: '2026-08-25T12:00:00.000Z',
      source: 'stdout',
    },
    {
      text: '[12:05:00] [Server thread/INFO]: Alice left the game',
      ts: '2026-08-25T12:05:00.000Z',
      source: 'stdout',
    },
    {
      text: '[12:10:00] [Server thread/INFO]: Bob joined the game',
      ts: '2026-08-26T12:10:00.000Z',
      source: 'stdout',
    },
  ];

  it('extracts only join/leave events from the console feed', () => {
    const events = sessionEventsFromConsole(lines);
    expect(events).toHaveLength(3);
    expect(events.every((event) => event.kind === 'join' || event.kind === 'leave')).toBe(true);
  });

  it('lists distinct players seen this session, in first-seen order', () => {
    const events = sessionEventsFromConsole(lines);
    expect(seenThisSession(events)).toEqual(['Alice', 'Bob']);
  });

  it('filters by player name', () => {
    const events = sessionEventsFromConsole(lines);
    expect(filterSessionEvents(events, 'bob')).toHaveLength(1);
  });

  it('groups events by calendar day, most recent day first', () => {
    const events = sessionEventsFromConsole(lines);
    const days = groupSessionEventsByDay(events);
    expect(days).toHaveLength(2);
    expect(days[0].events[0].player).toBe('Bob');
    expect(days[1].events).toHaveLength(2);
  });

  it('computes a join-to-leave duration label', () => {
    const events = sessionEventsFromConsole(lines);
    const join = events.find(
      (event) => event.player === 'Alice' && event.kind === 'join',
    ) as SessionEvent;
    expect(sessionDurationLabel(join, events, false)).toBe('5m');
  });

  it('does not claim a dedicated session-log route (none exists on the agent yet)', () => {
    expect(Object.values(playerPaths)).not.toContain('/v1/session-log');
  });
});
