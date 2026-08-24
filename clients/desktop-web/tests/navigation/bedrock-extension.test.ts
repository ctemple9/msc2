import { describe, expect, it, vi } from 'vitest';
import { sectionsForLayout } from '../../src/lib/navigation/layout';
import { hasUsableBedrockRuntime } from '../../src/lib/navigation/predicates';
import { NavigationRegistry } from '../../src/lib/navigation/registry';
import { buildSectionPath } from '../../src/lib/navigation/route';
import type {
  Capabilities,
  NavigationContext,
  SectionDescriptor,
} from '../../src/lib/navigation/types';

function capabilities(bedrock: unknown): Capabilities {
  return {
    agentVersion: '0.1.0',
    apiMajor: 1,
    apiMinor: 0,
    helpers: { duckdns: false, geyser: false, playit: false },
    hostOs: 'linux',
    permissions: ['settings'],
    serverTypes: {
      bedrock: bedrock as Capabilities['serverTypes']['bedrock'],
      fabric: false,
      forge: false,
      neoforge: false,
      paper: true,
      vanilla: true,
    },
  };
}

function context(bedrock: unknown): NavigationContext {
  return {
    hostId: 'host-a',
    serverId: 'server-a',
    permissions: ['settings'],
    capabilities: capabilities(bedrock),
  };
}

function futureBedrockDescriptor(load = vi.fn(async () => ({ default: 'future-bedrock' }))) {
  return {
    id: 'future-bedrock-runtime',
    label: 'Future Bedrock runtime',
    segment: 'runtime',
    routeFamily: 'bedrock',
    scope: 'server',
    isAvailable: hasUsableBedrockRuntime,
    load,
  } satisfies SectionDescriptor;
}

describe('Bedrock navigation extension seam', () => {
  it('keeps the reserved family absent until a later group registers a capability-gated descriptor', () => {
    const registry = new NavigationRegistry();
    const descriptor = futureBedrockDescriptor();
    const path = buildSectionPath(descriptor, 'host-a', 'server-a');

    expect(path).toBe('/hosts/host-a/servers/server-a/bedrock/runtime');
    expect(registry.resolve(path, context({ backend: null, supported: false }))).toMatchObject({
      kind: 'reserved',
      reason: 'reserved-family',
    });

    registry.register(descriptor);
    expect(registry.visibleSections(context({ backend: null, supported: false }))).toEqual([]);
    expect(registry.resolve(path, context({ backend: null, supported: false }))).toMatchObject({
      kind: 'forbidden',
      reason: 'capability',
    });
  });

  it('uses generated capability/runtime state, not host operating system, to activate the future route', async () => {
    const load = vi.fn(async () => ({ default: 'future-bedrock' }));
    const descriptor = futureBedrockDescriptor(load);
    const registry = new NavigationRegistry();
    registry.register(descriptor);
    const path = buildSectionPath(descriptor, 'host-a', 'server-a');

    const advertised = {
      backend: 'future-backend',
      supported: true,
      runtime: {
        backend: 'future-backend',
        reasonCode: 'future-reason',
        state: 'available',
      },
    };
    const enabled = context(advertised);
    enabled.capabilities = { ...enabled.capabilities!, hostOs: 'windows' };

    expect(registry.resolve(path, enabled).kind).toBe('section');
    expect(sectionsForLayout(registry, enabled, 400).sections).toEqual([descriptor]);
    await expect(registry.load(path, enabled)).resolves.toEqual({ default: 'future-bedrock' });
    expect(load).toHaveBeenCalledOnce();
  });

  it('does not activate a future route when the advertised runtime is unavailable', () => {
    const registry = new NavigationRegistry();
    const descriptor = futureBedrockDescriptor();
    registry.register(descriptor);

    expect(
      registry.resolve(
        buildSectionPath(descriptor, 'host-a', 'server-a'),
        context({
          backend: 'native',
          supported: true,
          runtime: { reasonCode: 'missing_bds', state: 'unavailable' },
        }),
      ),
    ).toMatchObject({ kind: 'forbidden', reason: 'capability' });
  });
});
