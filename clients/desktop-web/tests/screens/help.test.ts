import { describe, expect, it } from 'vitest';
import gateSource from '../../src/lib/help/FirstLaunchGate.svelte?raw';
import helpSource from '../../src/lib/sections/handbook/HelpSection.svelte?raw';
import setupSource from '../../src/lib/help/SetupIntro.svelte?raw';
import splashSource from '../../src/lib/help/SplashGate.svelte?raw';
import { renderMarkdown } from '../../src/lib/help/markdown';
import { firstLaunchStage, nextTourStep } from '../../src/lib/help/onboarding';

describe('shared help and onboarding screens', () => {
  it('renders agent Markdown through a safe allow-list rather than forwarding HTML', () => {
    const rendered = renderMarkdown(
      '# Hello\n<script>alert(1)</script>\n[guide](https://example.test)',
    );
    expect(rendered).toContain('<h1>Hello</h1>');
    expect(rendered).toContain('&lt;script&gt;alert(1)&lt;/script&gt;');
    expect(rendered).toContain('rel="noopener noreferrer"');
    expect(rendered).not.toContain('<script>');
  });

  it('preserves setup, Concept Guide, tour, skip, and reopen ordering', () => {
    expect(
      firstLaunchStage({ setupComplete: false, conceptGuideSeen: false, tourComplete: false }),
    ).toBe('setup');
    expect(
      firstLaunchStage({ setupComplete: true, conceptGuideSeen: false, tourComplete: false }),
    ).toBe('concept-guide');
    expect(
      firstLaunchStage({ setupComplete: true, conceptGuideSeen: true, tourComplete: false }),
    ).toBe('tour');
    expect(
      firstLaunchStage({ setupComplete: true, conceptGuideSeen: true, tourComplete: true }),
    ).toBe('complete');
    expect(
      nextTourStep(
        [{ order: 0, id: 'pause', title: '', body: '', anchor: 'x', requiresUserAction: true }],
        0,
      ),
    ).toBe(0);
    expect(
      nextTourStep(
        [{ order: 0, id: 'pause', title: '', body: '', anchor: 'x', requiresUserAction: true }],
        0,
        true,
      ),
    ).toBe(0);
  });

  it('uses contract fixtures for every explanation and retains an additive unknown-topic boundary', () => {
    const setupText = setupSource.replace(/\s+/g, ' ');
    expect(helpSource).toContain("'/v1/help/catalog'");
    expect(setupText).toContain('What is Minecraft Server Controller?');
    expect(setupText).toContain('Start and stop Java and Bedrock servers with one click');
    expect(setupText).toContain('Server Type');
    expect(setupText).toContain('Paper');
    expect(setupText).toContain('NeoForge');
    expect(setupText).toContain('Geyser crossplay');
    expect(setupText).toContain('setup-page-in');
    expect(setupText).toContain('Pick an Accent Color');
    expect(setupText).toContain('This setup takes about 2 minutes.');
    expect(setupText).toContain('aria-pressed');
    expect(gateSource).toContain('max-height: calc(100vh - 2rem)');
    expect(gateSource).toContain('scrollbar-width: none');
    expect(helpSource).toContain('resetSetupPreferences');
    expect(helpSource).toContain("'/v1/guides/onboarding'");
    expect(helpSource).toContain('That topic is not available on this agent');
    expect(helpSource).toContain('data-onboarding-anchor');
    expect(helpSource).toContain('hideCard');
    expect(splashSource).toContain('prefers-reduced-motion');
    expect(splashSource).toContain('fallbackMs');
    expect(splashSource).toContain('/splash_intro.mp4');
    expect(splashSource).toContain('autoplay');
    expect(splashSource).toContain('onended={finish}');
    expect(splashSource).toContain('onerror={handleVideoError}');
  });
});
