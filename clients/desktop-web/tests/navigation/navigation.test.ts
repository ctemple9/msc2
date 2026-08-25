import { describe, expect, it, vi } from 'vitest';
import { sectionsForLayout, layoutForWidth } from '../../src/lib/navigation/layout';
import { hasCapability } from '../../src/lib/navigation/predicates';
import { NavigationRegistry } from '../../src/lib/navigation/registry';
import { buildSectionPath, parseRoute } from '../../src/lib/navigation/route';
import appSource from '../../src/App.svelte?raw';
import type {
  Capabilities,
  NavigationContext,
  SectionDescriptor,
} from '../../src/lib/navigation/types';

const capabilities = (overrides: Partial<Capabilities> = {}): Capabilities => ({
  agentVersion: '0.1.0',
  apiMajor: 1,
  apiMinor: 0,
  helpers: { duckdns: false, geyser: false, playit: false },
  hostOs: 'linux',
  permissions: ['settings'],
  serverTypes: {
    bedrock: { backend: null, supported: false },
    fabric: false,
    forge: false,
    neoforge: false,
    paper: true,
    vanilla: true,
  },
  ...overrides,
});

const context = (overrides: Partial<NavigationContext> = {}): NavigationContext => ({
  hostId: 'host one',
  serverId: 'server/one',
  permissions: ['settings'],
  capabilities: capabilities(),
  ...overrides,
});

function descriptor(overrides: Partial<SectionDescriptor> = {}): SectionDescriptor {
  return {
    id: 'settings',
    label: 'Settings',
    segment: 'settings',
    scope: 'host',
    requiredPermissions: ['settings'],
    load: async () => ({ default: 'settings-component' }),
    ...overrides,
  };
}

describe('client navigation', () => {
  it('round-trips stable host and server deep links without losing parameters', () => {
    const future = descriptor({
      id: 'future',
      label: 'Future',
      segment: 'future',
      scope: 'server',
    });
    const path = buildSectionPath(future, 'host one', 'server/one', ['detail', 'part two']);

    expect(path).toBe('/hosts/host%20one/servers/server%2Fone/future/detail/part%20two');
    expect(parseRoute(path)).toMatchObject({
      kind: 'unknown',
      hostId: 'host one',
      serverId: 'server/one',
      sectionSegment: 'future',
      tail: ['detail', 'part two'],
    });
  });

  it('keeps reserved Bedrock and profile families out of the Phase 11 registry', () => {
    const registry = new NavigationRegistry();
    registry.register(descriptor());

    expect(registry.resolve('/hosts/h/servers/s/bedrock/runtime', context())).toMatchObject({
      kind: 'reserved',
      reason: 'reserved-family',
    });
    expect(registry.resolve('/hosts/h/servers/s/profiles/one', context())).toMatchObject({
      kind: 'reserved',
      reason: 'reserved-family',
    });
  });

  it('filters by permission and advertised capability, never by host operating system', () => {
    const registry = new NavigationRegistry();
    registry.register(
      descriptor({
        id: 'future-bedrock',
        label: 'Future Bedrock',
        segment: 'future-bedrock',
        scope: 'server',
        requiredPermissions: ['settings'],
        isAvailable: hasCapability(['serverTypes', 'bedrock', 'supported']),
      }),
    );

    expect(registry.visibleSections(context())).toHaveLength(0);
    const enabled = context({
      capabilities: capabilities({
        hostOs: 'windows',
        serverTypes: {
          ...capabilities().serverTypes,
          bedrock: { backend: 'native', supported: true },
        },
      }),
    });
    expect(registry.resolve('/hosts/h/servers/s/future-bedrock', enabled).kind).toBe('section');
    expect(
      registry.resolve('/hosts/h/servers/s/future-bedrock', context({ permissions: [] })).kind,
    ).toBe('forbidden');
  });

  it('adds a future descriptor without changing the router or shell', async () => {
    const load = vi.fn(async () => ({ default: 'future-component' }));
    const registry = new NavigationRegistry();
    registry.register(descriptor({ id: 'future', label: 'Future', segment: 'future', load }));

    expect(registry.resolve('/hosts/h/future', context()).kind).toBe('section');
    expect(load).not.toHaveBeenCalled();
    await expect(registry.load('/hosts/h/future', context())).resolves.toEqual({
      default: 'future-component',
    });
    expect(load).toHaveBeenCalledOnce();
  });

  it('uses layout mode as a presentation choice without imposing a tab count', () => {
    const registry = new NavigationRegistry();
    registry.register(descriptor());
    registry.register(
      descriptor({ id: 'another', label: 'Another', segment: 'another', requiredPermissions: [] }),
    );

    expect(layoutForWidth(759)).toBe('narrow');
    expect(layoutForWidth(760)).toBe('wide');
    expect(sectionsForLayout(registry, context(), 400).sections).toHaveLength(2);
    expect(sectionsForLayout(registry, context(), 1200).sections).toHaveLength(2);
  });

  it('boots the shared shell from host capabilities and token permissions', () => {
    expect(appSource).toContain('client.getCapabilities()');
    expect(appSource).toContain("'/v1/me'");
    expect(appSource).toContain('router.visibleSections(navigationContext)');
    expect(appSource).toContain('buildSectionPath(section, hostId, selectedServerId)');
    expect(appSource).toContain("const localAgentHostId = 'local-agent'");
    expect(appSource).not.toContain('demo-agent');
    expect(appSource).not.toContain('switchHost');
  });
});
