import { describe, expect, it } from 'vitest';
import { addonPaths, demoAddons } from '../../src/lib/sections/addons/model';

describe('add-on and modpack screens', () => {
  it('keeps catalog install and local-file staging as separate paths', () => {
    expect(addonPaths.install).toBe('/v1/components/install');
    expect(addonPaths.inspectPack).toBe('/v1/modpacks/inspect');
  });
  it('keeps provider state on the item instead of inventing a client list', () => {
    expect(demoAddons.every((addon) => addon.jarStem && addon.bucket)).toBe(true);
  });
});
