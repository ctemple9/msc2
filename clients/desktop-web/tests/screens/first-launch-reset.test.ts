import { describe, expect, it } from 'vitest';
import appSource from '../../src/App.svelte?raw';
import gateSource from '../../src/lib/help/FirstLaunchGate.svelte?raw';
import introSource from '../../src/lib/help/SetupIntro.svelte?raw';
import setupSource from '../../src/lib/sections/setup/AgentSetupSection.svelte?raw';

describe('first-launch reset recovery', () => {
  it('offers a continuation action for each local service recovery state', () => {
    expect(setupSource).toContain('Install and Continue');
    expect(setupSource).toContain('Start and Continue');
    expect(setupSource).toContain("readiness === 'incompatible'");
    expect(setupSource).toContain('Repair service');
    expect(setupSource).toContain('Closing the app window never stops the service');
  });

  it('pairs a reset remote host with a new host identity', () => {
    expect(setupSource).toContain('Pair this host again');
    expect(setupSource).toContain('msc pairing create');
    expect(setupSource).toContain('Pair Again');
    expect(appSource).toContain('async function pairAgain');
    expect(appSource).toContain('redeemRemotePairing(previousHost.baseUrl, pairingCode)');
    expect(appSource).toContain('hostStore.removeHost(previousHost.id)');
    expect(appSource).toContain('hostStore.addHost({');
    expect(appSource).toContain('await initializeClient();');
  });

  it('keeps first launch agent-owned and never creates a server during recovery', () => {
    expect(gateSource).toContain("'/v1/config/host-setup'");
    expect(introSource).toContain("'/v1/config/host-setup/complete'");
    expect(gateSource).toContain("'/v1/guides/concept-guide'");
    expect(gateSource).toContain("'/v1/guides/onboarding'");
    expect(gateSource).not.toContain("'/v1/servers/create'");
    expect(setupSource).not.toContain("'/v1/servers/create'");
  });
});
