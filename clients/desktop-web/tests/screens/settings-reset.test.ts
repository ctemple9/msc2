import { describe, expect, it } from 'vitest';
import appSettingsSource from '../../src/lib/sections/app-settings/AppSettingsSheet.svelte?raw';
import resetSource from '../../src/lib/sections/app-settings/ResetSheet.svelte?raw';
import confirmDialogSource from '../../src/lib/components/ConfirmDialog.svelte?raw';
import appSource from '../../src/App.svelte?raw';

describe('settings reset flows', () => {
  it('keeps the MSC Settings sheet language and adds both reset entry points', () => {
    expect(appSettingsSource).toContain('<Sheet title="MSC Settings" size="md"');
    expect(appSettingsSource).toContain('Reset this client');
    expect(appSettingsSource).toContain('Reset this host');
    expect(appSettingsSource).toContain('canResetHost');
    expect(appSettingsSource).not.toContain('gradient');
    expect(appSettingsSource).not.toContain('backdrop-filter');
  });

  it('requires the exact reset phrase and keeps native controls out of the reset sheet', () => {
    expect(resetSource).toContain('/v1/host/reset');
    expect(resetSource).toContain("'RESET AGENT'");
    expect(resetSource).not.toContain('RESET ${agentHostId}');
    expect(resetSource).toContain('serversRootPath');
    expect(resetSource).toContain('configuration');
    expect(resetSource).toContain('everything');
    expect(resetSource).toContain('onHostResetComplete');
    expect(resetSource).not.toContain('manageAgentService');
  });

  it('keeps the destructive confirmation in the shared MSC visual language', () => {
    expect(confirmDialogSource).toContain('<Button variant="secondary"');
    expect(confirmDialogSource).toContain('<Button variant="destructive"');
    expect(confirmDialogSource).toContain('var(--msc2-tier-chrome)');
    expect(confirmDialogSource).not.toContain('font-weight: 750');
    expect(confirmDialogSource).not.toContain('var(--msc-surface-raised)');
  });

  it('keeps local service and credential cleanup in the desktop app boundary', () => {
    expect(appSource).toContain('forgetCredentials');
    expect(appSource).toContain("manageAgentService('uninstall')");
    expect(appSource).toContain('hostStore.reset()');
    expect(appSource).toContain('clearClientPreferences()');
    expect(appSource).toContain('hostId === localAgentHostId');
  });
});
