import { describe, expect, it } from 'vitest';
import { backupPaths, demoBackups } from '../../src/lib/sections/backups/model';
import { demoWorlds, worldPaths } from '../../src/lib/sections/worlds/model';

describe('worlds, backups, and transfers', () => {
  it('keeps active-world mutation and saved-slot paths distinct', () => {
    expect(worldPaths.replaceActive).toBe('/v1/worlds/replace-active-world');
    expect(worldPaths.activate).toBe('/v1/worlds/activate');
  });
  it('exposes transactional backup actions and bounded fixture data', () => {
    expect(backupPaths).toMatchObject({
      now: '/v1/backups/now',
      restore: '/v1/backups/restore',
      delete: '/v1/backups/delete',
    });
    expect(demoBackups[0].slotId).toBe(demoWorlds[0].id);
  });
});
