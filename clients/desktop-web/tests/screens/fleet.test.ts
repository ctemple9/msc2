import { describe, expect, it } from 'vitest';
import manageSheetSource from '../../src/lib/sections/fleet/ManageSheet.svelte?raw';
import transferSheetSource from '../../src/lib/sections/fleet/TransferSheet.svelte?raw';
import {
  demoServers,
  fleetMutationPaths,
  selectedServer,
} from '../../src/lib/sections/fleet/model';

describe('fleet screen workflows', () => {
  it('puts export and import transfer actions behind Manage Servers’ Transfer button', () => {
    expect(manageSheetSource).toContain('showTransfer = true');
    expect(manageSheetSource).not.toContain('showImport');
    expect(manageSheetSource).toContain('<TransferSheet');
    expect(transferSheetSource).toContain('Export transfer file');
    expect(transferSheetSource).toContain('Import transfer file');
    expect(transferSheetSource).toContain('transferMode: importMode');
  });

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
