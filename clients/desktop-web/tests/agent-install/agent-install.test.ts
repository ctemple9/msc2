import { describe, expect, it } from 'vitest';
import { createBrowserPlatform } from '../../src/lib/platform';
import appSource from '../../src/App.svelte?raw';
import setupSource from '../../src/lib/sections/setup/AgentSetupSection.svelte?raw';
import tauriSource from '../../src/lib/platform/tauri.ts?raw';

describe('local agent installation boundary', () => {
  it('gives browser users a truthful shared-screen fallback', async () => {
    const status = await createBrowserPlatform().agentServiceStatus();

    expect(status).toMatchObject({ available: false, state: 'unavailable' });
    expect(status.detail).toContain('headless package');
  });

  it('keeps setup shared while only the shell invokes native service commands', () => {
    expect(setupSource).toContain('getPlatform()');
    expect(setupSource).not.toContain('isTauri');
    expect(tauriSource).toContain("invoke<AgentServiceStatus>('agent_service_status')");
    expect(tauriSource).toContain("invoke<AgentServiceStatus>('manage_agent_service'");
    expect(appSource).toContain("scope: 'host'");
    expect(appSource).toContain("await selectSection('agent-setup')");
  });

  it('states that closing the window does not stop a service or server', () => {
    expect(setupSource).toContain('Closing the app window never stops the service');
  });
});
