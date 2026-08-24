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
    pickFile: vi.fn(async () => picked),
    notify: vi.fn(async () => undefined),
    showMenu: vi.fn(async () => undefined),
    closeWindow: vi.fn(async () => undefined),
    onCloseRequested: vi.fn(async () => () => undefined),
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
  });

  it('uses native adapters only as a substitute for the same shared workflows', async () => {
    const dependencies = nativeDependencies();
    const desktop = createTauriPlatform(dependencies);
    const fileFallback = vi.fn(async () => picked);
    const workflowFallback = vi.fn(async () => undefined);

    expect(
      await desktop.pickFile({ label: 'World archive', extensions: ['zip'] }, fileFallback),
    ).toEqual(picked);
    await desktop.notify({ title: 'Complete' }, workflowFallback);
    await desktop.showMenu(
      [{ id: 'open-console', label: 'Open console', onSelect: vi.fn() }],
      workflowFallback,
    );
    await desktop.closeWindow(workflowFallback);

    expect(dependencies.pickFile).toHaveBeenCalledWith({
      label: 'World archive',
      extensions: ['zip'],
    });
    expect(dependencies.notify).toHaveBeenCalledOnce();
    expect(dependencies.showMenu).toHaveBeenCalledOnce();
    expect(dependencies.closeWindow).toHaveBeenCalledOnce();
    expect(fileFallback).not.toHaveBeenCalled();
    expect(workflowFallback).not.toHaveBeenCalled();
    expect(await desktop.credentialFor('remote-host')).toBeNull();
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
  });
});
