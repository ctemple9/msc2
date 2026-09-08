import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';

const screenshotPath = process.env.MSC_WEBKITGTK_SCREENSHOT;
const motionMode = process.env.MSC_EXPECT_MOTION ?? 'fallback';

async function waitForText(selector: string, expected: string): Promise<void> {
  const element = await $(selector);
  await element.waitForDisplayed();
  await browser.waitUntil(
    async () =>
      (
        await browser.execute(
          (target) => document.querySelector(target)?.textContent ?? '',
          selector,
        )
      ).includes(expected),
    {
      timeout: 15_000,
      timeoutMsg: `Timed out waiting for ${selector} to include ${expected}.`,
    },
  );
}

describe('Linux WebKitGTK native Tauri renderer', () => {
  it('renders and drives the production desktop bundle through the native driver', async () => {
    await browser.waitUntil(
      async () => await $('[role="tablist"][aria-label="Server sections"]').isDisplayed(),
      {
        timeout: 15_000,
        timeoutMsg: 'The native Tauri window did not render the shared navigation shell.',
      },
    );
    await browser.execute(() => localStorage.clear());
    await browser.execute(async () => {
      await fetch('http://127.0.0.1:4173/__test/host-setup', {
        method: 'POST',
        credentials: 'include',
      });
    });
    await browser.refresh();
    await browser.waitUntil(
      async () => await $('[role="tablist"][aria-label="Server sections"]').isDisplayed(),
      {
        timeout: 15_000,
        timeoutMsg: 'The native Tauri window did not render after clearing its profile.',
      },
    );
    await waitForText('.picker-label', 'Local agent');
    if (motionMode === 'fallback') {
      await browser.waitUntil(async () => !(await $('.splash').isExisting()), {
        timeout: 15_000,
        timeoutMsg: 'The splash playback or fallback did not finish in the native renderer.',
      });
    }

    const shellLayout = await browser.execute(() => {
      const shell = document.querySelector('.shell');
      const style = shell ? getComputedStyle(shell) : null;
      return { display: style?.display, width: shell?.getBoundingClientRect().width ?? 0 };
    });
    assert.equal(shellLayout.display, 'flex');
    assert.ok(shellLayout.width >= 320, 'the native shell respects the configured minimum width');

    const visibleSectionLabels = await browser.execute(() =>
      Array.from(
        document.querySelectorAll('[role="tablist"][aria-label="Server sections"] button'),
      ).map((button) => button.textContent?.trim() ?? ''),
    );
    if (screenshotPath) await browser.saveScreenshot(`${screenshotPath}.bootstrap.png`);
    assert.ok(
      visibleSectionLabels.includes('Overview'),
      `the native shell did not load capability-filtered sections: ${visibleSectionLabels.join(', ')}`,
    );

    await waitForText('.gate', 'Next');
    await (await $('//*[contains(@class, "gate")]//button[normalize-space() = "Next"]')).click();
    await waitForText('.gate', 'Server Type');
    await (await $('//*[contains(@class, "gate")]//button[normalize-space() = "Next"]')).click();
    await waitForText('.gate', 'Server Setup');
    await (await $('//*[contains(@class, "gate")]//button[normalize-space() = "Next"]')).click();
    await waitForText('.gate', 'playit.gg');
    await (await $('//*[contains(@class, "gate")]//button[normalize-space() = "Skip"]')).click();
    await waitForText('.gate', 'Xbox Broadcast');
    await (await $('//*[contains(@class, "gate")]//button[normalize-space() = "Skip"]')).click();
    await waitForText('.gate', 'Tailscale');
    await (await $('//*[contains(@class, "gate")]//button[normalize-space() = "Skip"]')).click();
    await waitForText('.gate', 'You’re All Set');
    await (
      await $('//*[contains(@class, "gate")]//button[normalize-space() = "Get Started"]')
    ).click();
    await browser.execute(() => {
      localStorage.setItem('msc_onboarding_tour_complete', 'true');
    });
    await browser.refresh();
    if (motionMode === 'fallback') {
      await browser.waitUntil(async () => !(await $('.splash').isExisting()), { timeout: 15_000 });
    }

    await (await $('[aria-label="Help & guides"]')).click();
    await waitForText('.help-screen', 'Guides');

    await (await $('.picker')).click();
    await (await $('//*[@role="menuitem" and normalize-space() = "Manage…"]')).click();
    const manage = await $('[role="dialog"][aria-label="Manage Servers"]');
    await manage.waitForDisplayed();
    await (await manage.$('[aria-label="More actions"]')).click();
    await (await $('//*[@role="menuitem" and normalize-space() = "Remove…"]')).click();
    await waitForText('.confirm-row', 'Remove "Survival" from this controller?');
    await (
      await $('//*[@role="dialog"]//button[normalize-space() = "Remove from Controller"]')
    ).click();
    await waitForText('.notice', 'Server record removed.');
    await (await manage.$('[aria-label="Close"]')).click();

    await browser.execute(() => {
      history.pushState({}, '', '/hosts/local-agent/servers/survival/handbook');
      dispatchEvent(new PopStateEvent('popstate'));
    });
    await waitForText('.help-screen', 'Guides');

    await browser.execute(() => {
      history.pushState({}, '', '/hosts/local-agent/servers/survival/console');
      dispatchEvent(new PopStateEvent('popstate'));
    });
    await waitForText('.screen-header', 'Console');

    const reducedMotion = await browser.execute(
      () => window.matchMedia('(prefers-reduced-motion: reduce)').matches,
    );
    if (motionMode === 'reduced') {
      assert.equal(reducedMotion, true, 'the native WebKitGTK renderer reports reduced motion');
      assert.equal(
        await $('.splash').isExisting(),
        false,
        'reduced motion omits the splash animation',
      );
    } else {
      assert.equal(reducedMotion, false, 'the fallback run keeps native motion enabled');
    }

    if (screenshotPath) await browser.saveScreenshot(screenshotPath);
  });
});
