import { describe, expect, it } from 'vitest';
import { addonPaths, addonStatusLabel, demoAddons } from '../../src/lib/sections/addons/model';
import {
  addOnKind,
  componentPaths,
  componentStatusLabel,
  componentTone,
  demoComponentsStatus,
  flavorDisplayName,
  isModdedFlavor,
  supportsCrossplay,
} from '../../src/lib/sections/components/model';
import type { Schema } from '../../src/lib/sections/shared/types';

describe('add-on and modpack screens', () => {
  it('keeps catalog install and local-file staging as separate paths', () => {
    expect(addonPaths.install).toBe('/v1/components/install');
    expect(addonPaths.inspectPack).toBe('/v1/modpacks/inspect');
  });
  it('exposes the real modpack manual-file completion route (D-027)', () => {
    expect(addonPaths.manualFile('op-1')).toBe('/v1/modpacks/op-1/manual-file');
  });
  it('keeps provider state on the item instead of inventing a client list', () => {
    expect(demoAddons.every((addon) => addon.jarStem && addon.bucket)).toBe(true);
  });
  it('uses the real resolver bucket enum, not an invented mod/plugin/component category', () => {
    // crates/msc-agent/src/routes/components.rs's addon_bucket_name -- the
    // only four values GET /v1/addons ever puts on AddonItemDTO.bucket.
    const realBuckets = new Set(['updateAvailable', 'noCompatibleVersion', 'upToDate', 'unlinked']);
    expect(demoAddons.every((addon) => realBuckets.has(addon.bucket))).toBe(true);
  });
  it('only labels a real update as available, not every bucket', () => {
    const updatable: Schema['AddonItemDTO'] = { ...demoAddons[0], bucket: 'updateAvailable' };
    const current: Schema['AddonItemDTO'] = { ...demoAddons[0], bucket: 'upToDate' };
    expect(addonStatusLabel(updatable)).toBe('Update available');
    expect(addonStatusLabel(current)).toBeUndefined();
  });
});

describe('components tab -- DetailsComponentsTabView.swift is the real oracle', () => {
  it('shares the version-management route pair with the Bedrock Runtime card', () => {
    expect(componentPaths).toMatchObject({
      status: '/v1/components',
      version: '/v1/components/version',
      versions: '/v1/versions',
    });
  });
  it('classifies Java flavors the same way JavaServerFlavor.swift does', () => {
    expect(addOnKind('paper')).toBe('plugin');
    expect(addOnKind('fabric')).toBe('mod');
    expect(addOnKind('vanilla')).toBeUndefined();
    expect(isModdedFlavor('neoforge')).toBe(true);
    expect(isModdedFlavor('purpur')).toBe(false);
    expect(flavorDisplayName('neoforge')).toBe('NeoForge');
  });
  it('only offers Crossplay for non-modded, non-vanilla Java servers', () => {
    const paper: Schema['ServerDTO'] = {
      id: 's',
      name: 'Test',
      directory: '/tmp',
      serverType: 'java',
      javaFlavor: 'paper',
    };
    expect(supportsCrossplay(paper)).toBe(true);
    expect(supportsCrossplay({ ...paper, javaFlavor: 'vanilla' })).toBe(false);
    expect(supportsCrossplay({ ...paper, javaFlavor: 'fabric' })).toBe(false);
    expect(supportsCrossplay({ ...paper, serverType: 'bedrock' })).toBe(false);
  });
  it('derives the Server JAR row status straight from the agent, not a client-side version diff', () => {
    const [paper] = demoComponentsStatus.components;
    expect(componentTone(paper)).toBe('ok');
    expect(componentStatusLabel(paper)).toBe('Up to date');
    const missing: Schema['ComponentStatusDTO'] = {
      name: 'paper',
      isUpToDate: false,
      updatable: false,
    };
    expect(componentTone(missing)).toBe('error');
    expect(componentStatusLabel(missing)).toBe('Missing');
  });
});
