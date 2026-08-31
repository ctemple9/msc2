import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import applicationShellSource from '../../src/lib/components/ApplicationShell.svelte?raw';
import topBarSource from '../../src/lib/components/shell/TopBar.svelte?raw';
import controlSidebarSource from '../../src/lib/components/shell/ControlSidebar.svelte?raw';
import howToConnectSource from '../../src/lib/components/shell/sidebar/HowToConnectSection.svelte?raw';
import detailsHeaderSource from '../../src/lib/components/shell/DetailsHeader.svelte?raw';
import tabStripSource from '../../src/lib/components/shell/TabStrip.svelte?raw';
import consoleDockSource from '../../src/lib/components/shell/ConsoleDock.svelte?raw';
import playerAvatarSource from '../../src/lib/components/shell/PlayerAvatar.svelte?raw';
import primaryTabsSource from '../../src/lib/navigation/primaryTabs.ts?raw';
import confirmDialogSource from '../../src/lib/components/ConfirmDialog.svelte?raw';
import appSource from '../../src/App.svelte?raw';

const tokensSource = readFileSync(
  fileURLToPath(new URL('../../src/lib/styles/tokens.css', import.meta.url)),
  'utf-8',
);

describe('S1 shell skeleton (docs/msc2/renderings/shell.html)', () => {
  it('assembles the shell from the host-aware sidebar, header, tab strip, and docked console', () => {
    expect(applicationShellSource).toContain('<TopBar');
    expect(applicationShellSource).toContain('<ControlSidebar');
    expect(applicationShellSource).toContain('<DetailsHeader');
    expect(applicationShellSource).toContain('<TabStrip');
    expect(applicationShellSource).toContain('<ConsoleDock');
    expect(controlSidebarSource).toContain('hostLabel');
    expect(controlSidebarSource).toContain('onSelectServer');
    expect(controlSidebarSource).toContain('onLifecycle');
    expect(controlSidebarSource).toContain('onInitiate');
    expect(controlSidebarSource).toContain('firstStartRequired');
    expect(controlSidebarSource).toContain('onOpenAgentSetup');
    expect(applicationShellSource).toContain('{onOpenBrowser}');
    expect(controlSidebarSource).toContain('onManage');
  });

  it('keeps the host-scoped agent screen reachable without adding an eighth server tab', () => {
    expect(controlSidebarSource).toContain("label: 'Agent…'");
    expect(applicationShellSource).toContain('{onOpenAgentSetup}');
    expect(appSource).toContain("selectSection('agent-setup')");
    expect(primaryTabsSource.match(/\{ id: '/g)?.length).toBe(7);
  });

  it('offers the desktop local agent in a browser without exposing remote-host controls', () => {
    expect(topBarSource).toContain('aria-label="Open local agent in browser"');
    expect(topBarSource).toContain('name="external-link"');
    expect(appSource).toContain(
      'onOpenBrowser={isDesktopShell ? () => void openLocalAgentInBrowser() : undefined}',
    );
    expect(appSource).toContain('await openLocalAgentBrowser()');
    expect(appSource).toContain('redeemBrowserHandoff(window.location, window.history)');
    expect(appSource).toContain("await selectSection('agent-setup')");
  });

  it('drives the primary tab strip from the registry-backed tab list, not a hardcoded switch', () => {
    expect(tabStripSource).toContain('{#each tabs as tab');
    // MSC 2 ships 7 of MSC 1's DetailsView tabs (docs/msc2/renderings/shell.html);
    // Packs is deliberately dropped (rolling-plan.md P12.5, owner decision 2026-08-26).
    expect(primaryTabsSource.match(/\{ id: '/g)?.length).toBe(7);
  });

  it('spends bannerColor only on its four sanctioned spots', () => {
    expect(topBarSource).toContain('bannerColorAccent');
    expect(detailsHeaderSource).toContain('bannerColorAccent');
    expect(tabStripSource).toContain('bannerColorAccent');
    expect(controlSidebarSource).toContain('bannerColorAccent');
  });

  it('keeps the docked console collapsible and console-tier surfaced', () => {
    expect(consoleDockSource).toContain('onToggle');
    expect(consoleDockSource).toContain('var(--msc2-tier-terminal)');
  });

  it('keeps the avatar switcher centered and submits identity with Enter', () => {
    expect(controlSidebarSource).not.toContain('<p class="overline">Actions</p>');
    expect(playerAvatarSource).toContain('class="edition-switcher"');
    expect(playerAvatarSource).toContain('justify-content: center');
    expect(playerAvatarSource).not.toContain('edit-link');
    expect(playerAvatarSource).toContain('class="avatar-trigger"');
    expect(playerAvatarSource).toContain('showEdit = true');
    expect(playerAvatarSource).toContain('class="link edit-avatar"');
    expect(playerAvatarSource).toContain('onclick={startEdit}>Edit</button>');
    expect(playerAvatarSource).not.toContain('meta.addLabel');
    expect(playerAvatarSource).not.toContain('type="submit"');
    expect(playerAvatarSource).toContain('event.preventDefault();');
    expect(playerAvatarSource).toContain('fetchBedrockBodyUrl');
    expect(playerAvatarSource).not.toContain("lookup isn't available yet");
  });

  it('keeps the sidebar scrollable without displaying scrollbar chrome', () => {
    expect(controlSidebarSource).toContain('scrollbar-width: none');
    expect(controlSidebarSource).toContain('.scroll::-webkit-scrollbar');
    expect(howToConnectSource).not.toContain('toggleAddressVisibility');
    expect(howToConnectSource).not.toContain('showAddresses');
    expect(howToConnectSource).toContain('class="pill-value mono"');
    expect(howToConnectSource).toContain('class:scrollable={playitSelected');
    expect(howToConnectSource).toContain('overflow-x: auto');
    expect(howToConnectSource).toContain('.pill-value.scrollable::-webkit-scrollbar');
  });

  it('shows the real crossplay address and authenticated Xbox identity', () => {
    expect(controlSidebarSource).toContain('bedrockPort={activeServer?.bedrockPort}');
    expect(controlSidebarSource).toContain(
      'xboxBroadcastEnabled={activeServer?.xboxBroadcastEnabled === true}',
    );
    expect(howToConnectSource).toContain("'/v1/broadcast/status'");
    expect(howToConnectSource).toContain("label: 'Console · add friend'");
    expect(howToConnectSource).not.toContain('status-warn-tint');
    expect(howToConnectSource).not.toContain('status-ok-tint');
    expect(howToConnectSource).not.toContain('status-bedrock-tint');
    expect(howToConnectSource).toContain('font-size: 10px');
  });

  it('defines accessible states, focus treatment, and reduced motion tokens', () => {
    expect(appSource).toContain('role="status"');
    expect(confirmDialogSource).toContain('role="alertdialog"');
    expect(topBarSource).toContain(':focus-visible');
    expect(tabStripSource).toContain(':focus-visible');
    expect(confirmDialogSource).toContain('export let context');
    expect(confirmDialogSource).toContain('{context}');
    expect(tokensSource).toContain('prefers-reduced-motion: reduce');
  });

  it('holds the anti-slop line: no glass, no gradient fills, no per-card accent rails', () => {
    for (const source of [
      applicationShellSource,
      topBarSource,
      controlSidebarSource,
      detailsHeaderSource,
      tabStripSource,
      consoleDockSource,
    ]) {
      expect(source).not.toContain('backdrop-filter');
      expect(source).not.toContain('linear-gradient');
      expect(source).not.toContain('radial-gradient');
    }
  });
});
