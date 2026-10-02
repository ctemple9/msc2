import { describe, expect, it, vi } from 'vitest';
import {
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

describe('local agent installation boundary', () => {
  it('keeps an explicitly stopped agent stopped until the user starts it', async () => {
    const platform: AgentPreparationPlatform = {
      kind: 'tauri',
      agentHealthCheck: vi.fn(async () => true),
      agentServiceStatus: vi.fn(async () => status('stopped')),
      manageAgentService: vi.fn(async () => status('running')),
    };
    const healthCheck = vi.fn().mockResolvedValue(true);

    await expect(
      prepareInstalledAgent(platform, healthCheck, { attempts: 2, delayMs: 0 }),
    ).resolves.toMatchObject({ state: 'stopped' });
    expect(platform.manageAgentService).not.toHaveBeenCalled();
    expect(healthCheck).not.toHaveBeenCalled();
  });

  it('does not install a missing service during automatic launch', async () => {
    const platform: AgentPreparationPlatform = {
      kind: 'tauri',
      agentHealthCheck: vi.fn(async () => true),
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
