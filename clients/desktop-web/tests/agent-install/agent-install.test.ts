import { describe, expect, it, vi } from 'vitest';
import {
  createBrowserPlatform,
  prepareInstalledAgent,
  type AgentPreparationPlatform,
  type AgentServiceStatus,
} from '../../src/lib/platform';

const status = (state: AgentServiceStatus['state']): AgentServiceStatus => ({
  available: state !== 'unavailable',
  platform: 'macos',
  serviceName: 'com.ctemple.msc2.agent',
  state,
  detail: state,
});
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

  it('starts an installed stopped agent and waits for its health endpoint', async () => {
    const platform: AgentPreparationPlatform = {
      kind: 'tauri',
      agentServiceStatus: vi.fn(async () => status('stopped')),
      manageAgentService: vi.fn(async () => status('running')),
    };
    const healthCheck = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);

    await expect(
      prepareInstalledAgent(platform, healthCheck, { attempts: 2, delayMs: 0 }),
    ).resolves.toMatchObject({ state: 'running' });
    expect(platform.manageAgentService).toHaveBeenCalledWith('start');
    expect(healthCheck).toHaveBeenCalledTimes(2);
  });

  it('does not install a missing service during automatic launch', async () => {
    const platform: AgentPreparationPlatform = {
      kind: 'tauri',
      agentServiceStatus: vi.fn(async () => status('not-installed')),
      manageAgentService: vi.fn(async () => status('running')),
    };
    const healthCheck = vi.fn().mockResolvedValue(true);

    await expect(prepareInstalledAgent(platform, healthCheck)).resolves.toMatchObject({
      state: 'not-installed',
    });
    expect(platform.manageAgentService).not.toHaveBeenCalled();
    expect(healthCheck).not.toHaveBeenCalled();
  });
});
