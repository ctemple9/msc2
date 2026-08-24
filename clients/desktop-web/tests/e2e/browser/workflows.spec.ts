import { expect, test, type Page } from '@playwright/test';

async function skipFirstLaunch(page: Page): Promise<void> {
  await page.addInitScript(() => {
    localStorage.setItem('msc.setup-complete', 'true');
    localStorage.setItem('msc.concept-guide-seen', 'true');
    localStorage.setItem('msc_onboarding_tour_complete', 'true');
  });
}

test('renders the production bundle at wide and narrow widths with keyboard navigation', async ({
  page,
}) => {
  await skipFirstLaunch(page);
  await page.goto('/');
  await expect(page.getByRole('navigation', { name: 'Sections' })).toBeVisible();
  await page.getByRole('button', { name: 'Handbook' }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Help and guides' })).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole('navigation', { name: 'Mobile navigation' })).toBeVisible();
});

test('walks a fresh profile through setup, Concept Guide, tour pauses, handoff, and reopen', async ({
  page,
}) => {
  await page.goto('/hosts/local-agent/servers/survival/handbook');
  const gate = page.locator('.gate');
  await gate.getByRole('button', { name: 'Next' }).click();
  await expect(gate.getByRole('heading', { name: 'One server. Your worlds.' })).toBeVisible();
  await gate.getByRole('button', { name: 'Continue to tour' }).click();
  await expect(page.getByText('Begin the guided tour.')).toBeVisible();
  await page.getByRole('button', { name: "Let's go →" }).click();
  await expect(page.locator('[data-onboarding-anchor="ob_manage_servers"]')).toBeVisible();
  await page.getByRole('button', { name: 'I did that' }).click();
  await expect(page.locator('[data-onboarding-anchor="ob_world_creation"]')).toBeVisible();
  await page.getByRole('button', { name: 'Continue' }).click();
  await gate.getByRole('button', { name: 'Finish', exact: true }).click();
  await expect(
    page.locator('.topic-reader').getByRole('heading', { name: 'Overview' }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Restart this tour' }).click();
  await expect(page.getByText('Begin the guided tour.')).toBeVisible();
});

test('uses the bounded splash fallback and removes it for reduced motion', async ({ page }) => {
  await skipFirstLaunch(page);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.locator('.splash')).toHaveCount(0);
});

test('keeps host state distinct and presents reconnect fallback', async ({ page }) => {
  await skipFirstLaunch(page);
  await page.goto('/');
  await expect(page.getByText('Local agent', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Switch host' }).click();
  await expect(page.getByText('Demo agent', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Refresh host' }).click();
  await expect(page.getByText('Reconnecting', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Refresh host' }).click();
  await expect(page.getByRole('main').getByText('Connected', { exact: true })).toBeVisible();
});

test('names destructive targets and completes bounded upload and download workflows', async ({
  page,
}) => {
  await skipFirstLaunch(page);
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible();
  const sections = page.getByRole('navigation', { name: 'Sections' });
  await sections.getByRole('button', { name: 'Fleet fleet', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Servers', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Delete server' }).first().click();
  const dialog = page.getByRole('alertdialog');
  await expect(dialog.getByText('Host: local-agent · Server: Survival')).toBeVisible();
  await dialog.getByRole('button', { name: 'Delete server' }).click();
  await expect(page.getByText('Server record removed.')).toBeVisible();

  await sections.getByRole('button', { name: 'Worlds worlds', exact: true }).click();
  await page.locator('input[type=file]').setInputFiles('tests/e2e/browser/fixtures/world.zip');
  await page.getByRole('button', { name: 'Stage file' }).click();
  await expect(page.getByText('world.zip staged (4 B).')).toBeVisible();
  await page.getByRole('button', { name: 'Import staged world' }).click();
  await page.getByRole('button', { name: 'Export world' }).click();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Download world export' }).first().click();
  expect((await download).suggestedFilename()).toBe('world-export.zip');
});
