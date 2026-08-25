import { describe, expect, it, vi } from 'vitest';
import {
  createBrowserPlatform,
  createTauriPlatform,
  type TauriPlatformDependencies,
} from '../../src/lib/platform';
import appSource from '../../src/App.svelte?raw';
import routerSource from '../../src/routes/router.ts?raw';
import transferPanelSource from '../../src/lib/sections/transfers/TransferPanel.svelte?raw';
import workspaceSource from '../../../../Cargo.toml?raw';
import tauriConfigSource from '../../src-tauri/tauri.conf.json?raw';

const picked = { name: 'world.zip', bytes: new Uint8Array([1, 2, 3]) };
const screenSources = import.meta.glob('../../src/lib/sections/**/*.svelte', {
  eager: true,
  import: 'default',
  query: '?raw',
}) as Record<string, string>;

function nativeDependencies(): TauriPlatformDependencies {
  return {
    pickFolder: vi.fn(async () => '/Users/example/MinecraftServers'),
    pickFilePath: vi.fn(async () => '/usr/bin/java'),
    pickFile: vi.fn(async () => picked),
    notify: vi.fn(async () => undefined),
    showMenu: vi.fn(async () => undefined),
    closeWindow: vi.fn(async () => undefined),
    openExternal: vi.fn(async () => undefined),
    onCloseRequested: vi.fn(async () => () => undefined),
    agentServiceStatus: vi.fn(async () => ({
      available: true,
      platform: 'macos',
      serviceName: 'com.ctemple.msc2.agent',
      state: 'running' as const,
      pid: 42,
      detail: 'Running as the installing user.',
    })),
    manageAgentService: vi.fn(async () => ({
      available: true,
      platform: 'macos',
      serviceName: 'com.ctemple.msc2.agent',
      state: 'running' as const,
      pid: 42,
      detail: 'Running as the installing user.',
    })),
  };
}

describe('Tauri boundary', () => {
  it('uses one browser fallback vocabulary when no desktop shell is present', async () => {
    const browser = createBrowserPlatform();
    const fallback = vi.fn(async () => picked);
    const notifyFallback = vi.fn(async () => undefined);

    expect(await browser.pickFile({ label: 'World archive' }, fallback)).toEqual(picked);
    await browser.notify({ title: 'Complete' }, notifyFallback);
    await browser.showMenu([], notifyFallback);
    await browser.closeWindow(notifyFallback);
    await browser.requestAgentAction('install', notifyFallback);

    expect(fallback).toHaveBeenCalledOnce();
    expect(notifyFallback).toHaveBeenCalledTimes(4);
    expect(await browser.credentialFor('remote-host')).toBeNull();
    expect((await browser.agentServiceStatus()).state).toBe('unavailable');
    expect((await browser.manageAgentService('install')).available).toBe(false);
  });

  it('uses native adapters only as a substitute for the same shared workflows', async () => {
    const dependencies = nativeDependencies();
    const desktop = createTauriPlatform(dependencies);
    const fileFallback = vi.fn(async () => picked);
    const workflowFallback = vi.fn(async () => undefined);

    await expect(desktop.pickFolder('Servers root')).resolves.toBe(
      '/Users/example/MinecraftServers',
    );
    await expect(desktop.pickFilePath({ label: 'Java executable' })).resolves.toBe('/usr/bin/java');
    expect(
      await desktop.pickFile({ label: 'World archive', extensions: ['zip'] }, fileFallback),
    ).toEqual(picked);
    await desktop.notify({ title: 'Complete' }, workflowFallback);
    await desktop.showMenu(
      [{ id: 'open-console', label: 'Open console', onSelect: vi.fn() }],
      workflowFallback,
    );
    await desktop.closeWindow(workflowFallback);
    await desktop.openExternal('https://example.test');

    expect(dependencies.pickFile).toHaveBeenCalledWith({
      label: 'World archive',
      extensions: ['zip'],
    });
    expect(dependencies.pickFolder).toHaveBeenCalledWith('Servers root');
    expect(dependencies.pickFilePath).toHaveBeenCalledWith({ label: 'Java executable' });
    expect(dependencies.notify).toHaveBeenCalledOnce();
    expect(dependencies.showMenu).toHaveBeenCalledOnce();
    expect(dependencies.closeWindow).toHaveBeenCalledOnce();
    expect(dependencies.openExternal).toHaveBeenCalledWith('https://example.test');
    expect(fileFallback).not.toHaveBeenCalled();
    expect(workflowFallback).not.toHaveBeenCalled();
    expect(await desktop.credentialFor('remote-host')).toBeNull();
    expect((await desktop.agentServiceStatus()).state).toBe('running');
    await desktop.manageAgentService('repair');
    expect(dependencies.manageAgentService).toHaveBeenCalledWith('repair');
  });

  it('treats cancelling a native file picker as a user choice', async () => {
    const dependencies = nativeDependencies();
    vi.mocked(dependencies.pickFile).mockResolvedValue(null);
    const fallback = vi.fn(async () => picked);

    await expect(
      createTauriPlatform(dependencies).pickFile({ label: 'World archive' }, fallback),
    ).resolves.toBeNull();
    expect(fallback).not.toHaveBeenCalled();
  });

  it('keeps platform detection out of routes and screens and the Tauri crate out of the workspace', () => {
    const routeAndScreenSources = [
      appSource,
      routerSource,
      transferPanelSource,
      ...Object.values(screenSources),
    ].join('\n');
    const config = JSON.parse(tauriConfigSource);

    expect(routeAndScreenSources).not.toContain('isTauri');
    expect(workspaceSource).not.toContain('clients/desktop-web/src-tauri');
    expect(config.build.frontendDist).toBe('../dist');
    expect(config.app.windows[0].backgroundColor).toBe('#1a1816');
    expect(routeAndScreenSources).toContain("segment: 'local-agent'");
    expect(routeAndScreenSources).not.toContain('invoke<AgentServiceStatus>');
  });
});
