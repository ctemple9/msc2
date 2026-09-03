import { describe, expect, it } from 'vitest';
import componentsSource from '../../src/lib/sections/components/ComponentsSection.svelte?raw';
import { addonPaths, addonStatusLabel, demoAddons } from '../../src/lib/sections/addons/model';
import {
  addOnKind,
  catalogDetailPaths,
  collapseVersions,
  componentPaths,
  componentStatusLabel,
  componentTone,
  conflictCount,
  demoComponentsStatus,
  expandedLoaders,
  filterVisibleVersions,
  flavorDisplayName,
  formatCount,
  isModdedFlavor,
  isStableVersion,
  isVersionCompatible,
  modrinthLoaderFacets,
  parseInlineMarkdown,
  sanitizeModrinthBody,
  supportsCrossplay,
} from '../../src/lib/sections/components/model';
import type { Schema } from '../../src/lib/sections/shared/types';

function makeVersion(
  overrides: Partial<Schema['CatalogVersionDTO']> = {},
): Schema['CatalogVersionDTO'] {
  return {
    id: 'v1',
    projectId: 'p1',
    name: 'Version 1',
    versionNumber: '1.0.0',
    versionType: 'release',
    gameVersions: ['1.21.1'],
    loaders: ['paper'],
    dependencies: [],
    files: [],
    ...overrides,
  };
}

describe('add-on and modpack screens', () => {
  it('offers the MSC 1 voice-tunnel choices after an SVC add-on change', () => {
    expect(componentsSource).toContain('Simple Voice Chat needs a tunnel');
    expect(componentsSource).toContain('Set up voice tunnel');
    expect(componentsSource).toContain('Disable Voice Chat');
    expect(componentsSource).toContain("Don't Ask Again");
    expect(componentsSource).toContain('msc2.svc-tunnel-prompt');
    expect(componentsSource).toContain('serverRunning');
  });
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
  it('shows pack contents while preserving the whole-pack replacement boundary', () => {
    expect(componentsSource).toContain('This server is managed by its modpack');
    expect(componentsSource).toContain("{packManaged ? 'Replace Modpack' : 'Import Modpack'}");
    expect(componentsSource).toContain('replace the whole pack to change it');
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

describe('project detail page -- ModrinthProjectDetailView.swift is the real oracle', () => {
  it('builds the two new P12.7c routes, percent-encoding the project id', () => {
    expect(catalogDetailPaths.project('sodium mod')).toBe('/v1/catalog/projects/sodium%20mod');
    expect(catalogDetailPaths.versions('sodium')).toBe('/v1/catalog/projects/sodium/versions');
  });

  it('formats large counts the same way for downloads and followers', () => {
    expect(formatCount(999)).toBe('999');
    expect(formatCount(1_200)).toBe('1.2K');
    expect(formatCount(1_200_000)).toBe('1.2M');
  });

  it('mirrors JavaServerFlavor.modrinth_loader_facets exactly', () => {
    expect(modrinthLoaderFacets('paper')).toEqual(['paper', 'spigot', 'bukkit']);
    expect(modrinthLoaderFacets('fabric')).toEqual(['fabric']);
    expect(modrinthLoaderFacets('quilt')).toEqual(['quilt', 'fabric']);
    expect(modrinthLoaderFacets('neoforge')).toEqual(['neoforge']);
    expect(modrinthLoaderFacets('vanilla')).toEqual([]);
    expect(modrinthLoaderFacets(undefined)).toEqual([]);
  });

  it('lets NeoForge run Forge mods and Quilt run Fabric mods for version filtering', () => {
    expect(expandedLoaders('neoforge', ['neoforge'])).toEqual(new Set(['neoforge', 'forge']));
    expect(expandedLoaders('quilt', ['quilt', 'fabric'])).toEqual(new Set(['quilt', 'fabric']));
    expect(expandedLoaders('paper', ['paper'])).toEqual(new Set(['paper']));
  });

  it('only calls a version compatible when its gameVersions include the server version', () => {
    const v = makeVersion({ gameVersions: ['1.21.1', '1.21.2'] });
    expect(isVersionCompatible(v, '1.21.1')).toBe(true);
    expect(isVersionCompatible(v, '1.20.4')).toBe(false);
    expect(isVersionCompatible(v, undefined)).toBe(false);
  });

  it('counts only "incompatible" dependencies as conflicts', () => {
    const v = makeVersion({
      dependencies: [
        { dependencyType: 'required' },
        { dependencyType: 'incompatible' },
        { dependencyType: 'incompatible' },
      ],
    });
    expect(conflictCount(v)).toBe(2);
  });

  it('treats only versionType release as stable', () => {
    expect(isStableVersion(makeVersion({ versionType: 'release' }))).toBe(true);
    expect(isStableVersion(makeVersion({ versionType: 'beta' }))).toBe(false);
  });

  it('collapses same-versionNumber platform variants, preferring the server-loader match', () => {
    const paperBuild = makeVersion({ id: 'a', versionNumber: '2.0.0', loaders: ['paper'] });
    const fabricBuild = makeVersion({ id: 'b', versionNumber: '2.0.0', loaders: ['fabric'] });
    const collapsed = collapseVersions([fabricBuild, paperBuild], new Set(['paper']));
    expect(collapsed).toHaveLength(1);
    expect(collapsed[0].id).toBe('a');
  });

  it('falls back to the unfiltered stable list rather than hiding every version', () => {
    const onlyFabric = makeVersion({ loaders: ['fabric'] });
    const visible = filterVisibleVersions([onlyFabric], {
      stableOnly: true,
      loaders: new Set(['paper']),
    });
    expect(visible).toEqual([onlyFabric]);
  });

  it('strips embeds/images and converts HTML anchors to safe inline links', () => {
    const raw =
      '# Title\n<iframe src="x"></iframe>\n<img src="a.png">\nSee <a href="https://example.test">docs</a>.\n\n\n\nDone';
    const clean = sanitizeModrinthBody(raw);
    expect(clean).not.toContain('<iframe');
    expect(clean).not.toContain('<img');
    expect(clean).toContain('**Title**');
    expect(clean).toContain('[docs](https://example.test)');
    expect(clean).not.toMatch(/\n{3,}/);
  });

  it('parses the sanitized subset into text/bold/link segments, never raw HTML', () => {
    const segments = parseInlineMarkdown('Hello **world**, see [docs](https://example.test) now');
    expect(segments).toEqual([
      { type: 'text', text: 'Hello ' },
      { type: 'bold', text: 'world' },
      { type: 'text', text: ', see ' },
      { type: 'link', text: 'docs', href: 'https://example.test' },
      { type: 'text', text: ' now' },
    ]);
  });
});
