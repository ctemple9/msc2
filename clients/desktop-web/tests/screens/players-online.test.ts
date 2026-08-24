import { describe, expect, it } from 'vitest';
import {
  demoPlayers,
  playerPaths,
  playerSearch,
} from '../../src/lib/sections/players-online/model';

describe('online roster', () => {
  it('searches generic players and leaves future profile fields additive', () => {
    expect(playerSearch(demoPlayers, 'cam')).toHaveLength(1);
    expect(playerPaths.players).toBe('/v1/players');
  });
  it('does not claim a profile route', () => {
    expect(Object.values(playerPaths)).not.toContain('/v1/players/profiles');
  });
});
