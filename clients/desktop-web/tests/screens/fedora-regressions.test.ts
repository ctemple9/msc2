import { describe, expect, it } from 'vitest';
import appSource from '../../src/App.svelte?raw';
import settingsSource from '../../src/lib/sections/app-settings/AppSettingsSheet.svelte?raw';
import sidebarSource from '../../src/lib/components/shell/sidebar/HowToConnectSection.svelte?raw';

describe('Fedora remote Xbox Broadcast sign-in regression', () => {
  it('starts sign-in from Settings without requiring a running Minecraft server', () => {
    const startSignIn = settingsSource.match(
      /async function startBroadcastSignIn\(\): Promise<void> \{([\s\S]*?)\n  \}/,
    )?.[1];

    expect(startSignIn).toBeDefined();
    expect(startSignIn).toContain('await refreshBroadcastAuthPrompt()');
    expect(startSignIn).toContain("'/v1/broadcast/status'");
    expect(startSignIn).toContain("'/v1/broadcast/start'");
    expect(startSignIn).not.toMatch(/server.*running|running.*server/i);
    expect(settingsSource).toContain("'/v1/broadcast/auth-prompt'");
    expect(settingsSource).toContain('<BroadcastAuthSheet {api} prompt={broadcastAuth}');
  });

  it('polls the selected agent for a device-code prompt and shows it in a shell sheet', () => {
    expect(appSource).toContain("'/v1/broadcast/auth-prompt'");
    expect(appSource).toContain('broadcastAuthShouldPoll && broadcastAuthTimer === undefined');
    expect(appSource).toContain('broadcastAuthTimer = setInterval');
    expect(appSource).toContain('{#if broadcastAuth?.isPresent}');
    expect(appSource).toContain('<BroadcastAuthSheet api={screenApi}');
  });

  it('shows the authenticated identity returned by the remote agent in the sidebar', () => {
    expect(sidebarSource).toContain("'/v1/broadcast/status'");
    expect(sidebarSource).toContain('value: broadcast.gamertag');
    expect(sidebarSource).toContain("broadcast.authenticated ? 'Signed in' : 'Not signed in yet'");
  });
});
