import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';
import { describe, it } from 'mocha';

const screenshotPath = process.env.MSC_WEBKITGTK_SCREENSHOT;
const motionMode = process.env.MSC_EXPECT_MOTION ?? 'fallback';

async function waitForText(selector: string, expected: string): Promise<void> {
  const element = await $(selector);
  await element.waitForDisplayed();
  await browser.waitUntil(async () => (await element.getText()).includes(expected), {
    timeout: 15_000,
    timeoutMsg: `Timed out waiting for ${selector} to include ${expected}.`,
  });
}

describe('Linux WebKitGTK native Tauri renderer', () => {
  it('renders and drives the production desktop bundle through the native driver', async () => {
    await browser.waitUntil(async () => await $('nav[aria-label="Sections"]').isDisplayed(), {
      timeout: 15_000,
      timeoutMsg: 'The native Tauri window did not render the shared navigation shell.',
    });
    await browser.execute(() => localStorage.clear());
    await browser.refresh();
    await browser.waitUntil(async () => await $('nav[aria-label="Sections"]').isDisplayed(), {
      timeout: 15_000,
      timeoutMsg: 'The native Tauri window did not render after clearing its profile.',
    });
    await waitForText('header', 'Local agent');
    if (motionMode === 'fallback') {
      await browser.waitUntil(async () => !(await $('.splash').isExisting()), {
        timeout: 15_000,
        timeoutMsg: 'The splash playback or fallback did not finish in the native renderer.',
      });
    }

    const shellLayout = await browser.execute(() => {
      const shell = document.querySelector('.application-shell');
      const style = shell ? getComputedStyle(shell) : null;
      return { display: style?.display, width: shell?.getBoundingClientRect().width ?? 0 };
    });
    assert.equal(shellLayout.display, 'grid');
    assert.ok(shellLayout.width >= 320, 'the native shell respects the configured minimum width');

    const visibleSectionLabels = await browser.execute(() =>
      Array.from(document.querySelectorAll('nav[aria-label="Sections"] button')).map(
        (button) => button.textContent?.trim() ?? '',
      ),
    );
    if (screenshotPath) await browser.saveScreenshot(`${screenshotPath}.bootstrap.png`);
    assert.ok(
      visibleSectionLabels.includes('Handbook handbook'),
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
    await waitForText('.gate', 'One server. Your worlds.');
    await browser.execute(() => {
      localStorage.setItem('msc.setup-complete', 'true');
      localStorage.setItem('msc.concept-guide-seen', 'true');
      localStorage.setItem('msc_onboarding_tour_complete', 'true');
    });
    await browser.refresh();
    if (motionMode === 'fallback') {
      await browser.waitUntil(async () => !(await $('.splash').isExisting()), { timeout: 15_000 });
    }

    await (await $('//nav[@aria-label="Sections"]//button[contains(., "Handbook")]')).click();
    await waitForText('main', 'Help and guides');

    await (await $('//nav[@aria-label="Sections"]//button[contains(., "Fleet")]')).click();
    await waitForText('main', 'Servers');
    await (await $('[aria-label="Delete server"]')).click();
    await waitForText('[role="alertdialog"]', 'HOST: LOCAL-AGENT · SERVER: SURVIVAL');
    await (
      await $('//dialog[@role="alertdialog"]//button[normalize-space() = "Delete server"]')
    ).click();
    await waitForText('main', 'Server record removed.');

    await (await $('//nav[@aria-label="Sections"]//button[contains(., "Console")]')).click();
    await waitForText('main', 'Console');
    await browser.execute(() => {
      history.pushState({}, '', '/hosts/local-agent/servers/survival/handbook');
      dispatchEvent(new PopStateEvent('popstate'));
    });
    await waitForText('main', 'Help and guides');

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
