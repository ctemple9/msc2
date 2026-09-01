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

  it('keeps service controls scoped to the selected local host', () => {
    expect(appSource).toContain(
      'baseUrl: isDesktopShell ? LOCAL_AGENT_ORIGIN : window.location.origin',
    );
    expect(appSource).toContain('isLocalHost={hostId === localAgentHostId}');
    expect(setupSource).toContain("export let hostId = '';");
    expect(setupSource).toContain('isDesktopShell && isLocalHost');
    expect(setupSource).toContain('{:else if !isDesktopShell && isLoopbackHost}');
    expect(setupSource).toContain('Manage the agent on {hostLabel}');
    expect(setupSource).toContain('label={`Managed on ${hostLabel}`}');
  });

  it('states that closing the window does not stop a service or server', () => {
    expect(setupSource).toContain('Closing this window does not stop it or any');
  });

  it('labels a healthy connection as connected', () => {
    expect(setupSource).toContain("ready: 'Agent connected'");
    expect(setupSource).toContain('connected and ready for server management');
  });

  it('keeps connection, service, and service controls in equal columns', () => {
    expect(setupSource).toContain('<div class="screen-grid two">');
    expect(setupSource).toContain('Install, start, stop, or repair the agent.');
    expect(setupSource).toContain(
      'onclick={() => void (onAgentRetry ? onAgentRetry() : refresh())}',
    );
    expect(setupSource).toContain('grid-template-columns: repeat(2, minmax(0, 1fr));');
  });

  it('explains the control panel and agent relationship', () => {
    expect(setupSource).toContain('MSC has two parts');
    expect(setupSource).toContain('The control panel');
    expect(setupSource).toContain('The agent');
    expect(setupSource).toContain('The control panel is like a remote control');
    expect(setupSource).toContain('One control panel can connect to multiple agents');
  });

  it('starts an installed stopped agent and waits for its health endpoint', async () => {
    const platform: AgentPreparationPlatform = {
      kind: 'tauri',
      agentHealthCheck: vi.fn(async () => true),
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

  it('shows native service errors instead of leaving install feedback blank', () => {
    expect(setupSource).toContain('errorMessage = String(error)');
    expect(setupSource).toContain('Could not change the agent service');
  });
});
