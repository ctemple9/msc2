import { describe, expect, it } from 'vitest';
import { playerContentChanges } from '../../src/lib/sections/components/player-content-view';
import type { ContentEntry } from '../../src/lib/sections/components/content-model';

const entry = (path: string, sha512: string): ContentEntry => ({
  path,
  sha512,
  byteSize: 4,
  audience: 'both',
  optional: false,
  classificationEvidence: [],
  unresolvedReasons: [],
});

// New multi-selection must not replace customized player bytes with server bytes.
// Backend safety tests do not cover which changes this UI chooses. Pure inputs;
// no API, filesystem, timers or rendering; expected runtime <5 ms total.
describe('player content bulk decisions', () => {
  it('keeps existing player customizations and each side’s exact review hash', () => {
    const path = 'mods/example.jar';
    const server = entry(path, 'a'.repeat(128));
    const player = { ...entry(path, 'b'.repeat(128)), enabled: false };
    const changes = playerContentChanges([{ path, server, player }], 'optional');
    expect(
      changes.some(
        (change) => change.action === 'copyServerToPlayers' || change.action === 'removePlayerFile',
      ),
    ).toBe(false);
    expect(changes.find((change) => change.action === 'setServerAudience')).toMatchObject({
      path,
      sha512: server.sha512,
      audience: 'both',
      optional: true,
    });
    expect(changes.find((change) => change.action === 'setPlayerOptions')).toMatchObject({
      path,
      sha512: player.sha512,
      optional: true,
    });
    expect(changes.some((change) => change.enabled !== undefined)).toBe(false);
    expect(playerContentChanges([{ path, server, player }], 'serverOnly')).toEqual([
      { action: 'setServerAudience', path, sha512: server.sha512, audience: 'server' },
    ]);
  });
  it('includes a newly selected optional shader without activating it', () => {
    const path = 'shaderpacks/example.zip';
    const server = entry(path, 'c'.repeat(128));
    const changes = playerContentChanges([{ path, server }], 'optional');
    expect(changes).toContainEqual({
      action: 'copyServerToPlayers',
      path,
      sha512: server.sha512,
      optional: true,
    });
    expect(changes).toContainEqual({
      action: 'setPlayerOptions',
      path,
      sha512: server.sha512,
      enabled: false,
    });
  });
});
