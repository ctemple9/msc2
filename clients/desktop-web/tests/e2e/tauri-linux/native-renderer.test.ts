import assert from 'node:assert/strict';
import { browser, $ } from '@wdio/globals';

const screenshotPath = process.env.MSC_WEBKITGTK_SCREENSHOT;
const motionMode = process.env.MSC_EXPECT_MOTION ?? 'fallback';

async function waitForText(selector: string, expected: string): Promise<void> {
  await browser.waitUntil(
    async () => {
      return await browser.execute(
        (target, expectedText) => {
          return Array.from(document.querySelectorAll(target)).some((element) => {
            const style = getComputedStyle(element);
            const bounds = element.getBoundingClientRect();
            return (
              style.display !== 'none' &&
              style.visibility !== 'hidden' &&
              bounds.width > 0 &&
              bounds.height > 0 &&
              (element.textContent ?? '').includes(expectedText)
            );
          });
        },
        selector,
        expected,
      );
    },
    {
      timeout: 15_000,
      timeoutMsg: `Timed out waiting for ${selector} to include ${expected}.`,
    },
  );
}

describe('Linux WebKitGTK native Tauri renderer', () => {
  it('renders the production bundle in the native renderer', async () => {
    await browser.waitUntil(
      async () => await $('[role="tablist"][aria-label="Server sections"]').isDisplayed(),
      {
        timeout: 15_000,
        timeoutMsg: 'The native Tauri window did not render the shared navigation shell.',
      },
    );
    await browser.execute(() => localStorage.clear());
    await browser.execute(async () => {
      await fetch('http://127.0.0.1:4173/__test/host-setup?native=1', {
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
