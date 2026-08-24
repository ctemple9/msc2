import { describe, expect, it } from 'vitest';
import { administrationPaths, isSchemaDriven } from '../../src/lib/sections/admin/model';

describe('administration screens', () => {
  it('covers settings, diagnostics, helpers, networking, packs, and access routes', () => {
    expect(administrationPaths.settings).toContain('/v1/settings');
    expect(administrationPaths.health).toContain('/v1/health/repair');
    expect(administrationPaths.connectivity).toContain('/v1/resourcepacks');
    expect(administrationPaths.access).toContain('/v1/users/revoke');
  });
  it('accepts new setting fields without a client-side enum', () => {
    expect(isSchemaDriven([{ key: 'future.setting' }])).toBe(true);
    expect(isSchemaDriven([{ key: '' }])).toBe(false);
  });
});
