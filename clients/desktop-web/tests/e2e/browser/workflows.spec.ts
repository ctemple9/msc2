import { expect, test, type Page } from '@playwright/test';

async function skipFirstLaunch(page: Page): Promise<void> {
  await page.addInitScript(() => {
    localStorage.setItem('msc_onboarding_tour_complete', 'true');
  });
}

test('renders the production bundle at wide and narrow widths with keyboard navigation', async ({
  page,
}) => {
  await skipFirstLaunch(page);
  await page.goto('/');
  const sections = page.getByRole('tablist', { name: 'Server sections' });
  await expect(sections).toBeVisible();
  await page.getByRole('button', { name: 'Help & guides' }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByText('Guides', { exact: true })).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(sections).toBeVisible();
});

test('walks a fresh profile through setup, tour pauses, handoff, and reopen', async ({ page }) => {
  await page.request.post('/__test/host-setup');
  await page.goto('/hosts/local-agent/servers/survival/handbook');
  const gate = page.locator('.gate');
  await gate.getByRole('button', { name: 'Next' }).click();
  await expect(gate.getByRole('heading', { name: 'Server Type', level: 2 })).toBeVisible();
  await gate.getByRole('button', { name: 'Next' }).click();
  await expect(gate.getByRole('heading', { name: 'Server Setup', level: 2 })).toBeVisible();
  await gate.getByRole('button', { name: 'Next' }).click();
  await expect(gate.getByRole('heading', { name: 'playit.gg', level: 2 })).toBeVisible();
  await gate.getByRole('button', { name: 'Skip' }).click();
  await expect(gate.getByRole('heading', { name: 'Xbox Broadcast', level: 2 })).toBeVisible();
  await gate.getByRole('button', { name: 'Download Now' }).click();
  await expect(gate.getByText('Verified downloaded: MCXboxBroadcastStandalone.jar')).toBeVisible();
  await gate.getByRole('button', { name: 'Skip' }).click();
  await expect(gate.getByRole('heading', { name: 'Tailscale', level: 2 })).toBeVisible();
  await gate.getByRole('button', { name: 'Skip' }).click();
  await expect(gate.getByRole('heading', { name: 'You’re All Set', level: 2 })).toBeVisible();
  await gate.getByRole('button', { name: 'Get Started' }).click();
  await expect(page.getByText('Begin the guided tour.')).toBeVisible();
  await page.getByRole('button', { name: "Let's go →" }).click();
  await page.getByRole('button', { name: /Local agent/ }).click();
  await page.getByRole('menuitem', { name: 'Manage…', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Add Server…', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Add Server…', exact: true }).click();
  await expect(page.locator('button.path-card.selected')).toContainText('Start Fresh');
  await expect(
    page.getByRole('button', { name: /Import or Create from Modpack/ }),
  ).toBeDisabled();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await expect(page.getByRole('heading', { name: "You're All Set", level: 2 })).toBeVisible();
  await page.getByRole('dialog').getByRole('button', { name: 'Finish', exact: true }).click();
  await page
    .getByRole('dialog', { name: 'Add Server' })
    .getByRole('button', { name: 'Close' })
    .click();
  await page
    .getByRole('dialog', { name: 'Manage Servers' })
    .getByRole('button', { name: 'Close' })
    .click();
  await expect(
    page.locator('.reader').getByRole('heading', { name: 'Overview' }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Restart the guide' }).click();
  await expect(page.getByText('Begin the guided tour.')).toBeVisible();
});

test('uses the bounded splash fallback and removes it for reduced motion', async ({ page }) => {
  await skipFirstLaunch(page);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.locator('.splash')).toHaveCount(0);
});

test('keeps the local host identity and presents reconnect fallback', async ({ page }) => {
  await skipFirstLaunch(page);
  await page.setExtraHTTPHeaders({ 'x-msc-test-reconnect': 'true' });
  await page.goto('/');
  await expect(page.getByRole('button', { name: /Local agent/ })).toBeVisible();
  await page.getByRole('button', { name: 'Refresh' }).click();
  await expect(page.getByRole('heading', { name: 'Connect MSC 2 to an agent' })).toBeVisible();
  await page.getByRole('button', { name: 'Refresh' }).click();
  await expect(page.getByText('Connected', { exact: true }).first()).toBeVisible();
});

test('names destructive targets and completes bounded upload and download workflows', async ({
  page,
}) => {
  await skipFirstLaunch(page);
  await page.goto('/');
  const sections = page.getByRole('tablist', { name: 'Server sections' });
  await page.getByRole('button', { name: /Local agent/ }).click();
  await page.getByRole('menuitem', { name: 'Manage…', exact: true }).click();
  const manage = page.getByRole('dialog', { name: 'Manage Servers' });
  await expect(manage).toBeVisible();
  await manage.getByRole('button', { name: 'More actions' }).first().click();
  await page.getByRole('menuitem', { name: 'Remove…', exact: true }).click();
  await expect(manage.getByText(/Remove "Survival" from this controller/)).toBeVisible();
  await manage.getByRole('button', { name: 'Remove from Controller' }).click();
  await expect(page.getByText('Server record removed.')).toBeVisible();

  await manage.getByRole('button', { name: 'Close' }).click();
  await sections.getByRole('tab', { name: 'Worlds', exact: true }).click();
  await page.getByRole('button', { name: '…', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Import ZIP…', exact: true }).click();
  const importWorld = page.getByRole('dialog', { name: 'Import ZIP as New World' });
  const fileChooser = page.waitForEvent('filechooser');
  await importWorld.getByRole('button', { name: 'Choose ZIP…', exact: true }).click();
  await (await fileChooser).setFiles('tests/e2e/browser/fixtures/world.zip');
  await expect(importWorld.getByText('Selected: world.zip')).toBeVisible();
  await importWorld.getByRole('button', { name: 'Import', exact: true }).click();
  await expect(page.getByText('Imported ZIP as a new world slot.')).toBeVisible();
});
