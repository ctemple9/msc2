import { describe, expect, it } from 'vitest';
import {
  demoServers,
  fleetMutationPaths,
  selectedServer,
} from '../../src/lib/sections/fleet/model';

describe('fleet screen workflows', () => {
  it('keeps server selection host-scoped and names every lifecycle boundary', () => {
    expect(selectedServer(demoServers, 'creative')?.name).toBe('Creative');
    expect(fleetMutationPaths).toMatchObject({
      create: '/v1/servers/create',
      import: '/v1/servers/import',
      start: '/v1/start',
      stop: '/v1/stop',
    });
  });
  it('has a safe fallback for a missing active server', () => {
    expect(selectedServer(demoServers, 'missing')?.id).toBe('survival');
  });
});
