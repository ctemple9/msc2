import { expect, test } from '@playwright/test';

async function finishHostSetup(page: import('@playwright/test').Page): Promise<void> {
  const gate = page.locator('.gate');
  await expect(gate.getByRole('heading', { name: 'First-time Setup' })).toBeVisible();

  await gate.getByRole('button', { name: 'Next' }).click();
  await expect(gate.getByRole('heading', { name: 'Server Type' })).toBeVisible();
  await gate.getByRole('button', { name: 'Next' }).click();
  await expect(gate.getByRole('heading', { name: 'Server Setup' })).toBeVisible();
  await gate.getByRole('button', { name: 'Next' }).click();
  await expect(gate.getByRole('heading', { name: 'playit.gg' })).toBeVisible();
  await gate.getByRole('button', { name: 'Skip' }).click();
  await expect(gate.getByRole('heading', { name: 'Xbox Broadcast' })).toBeVisible();
  await gate.getByRole('button', { name: 'Skip' }).click();
  await expect(gate.getByRole('heading', { name: 'Tailscale' })).toBeVisible();
  await gate.getByRole('button', { name: 'Skip' }).click();
  await expect(gate.getByRole('heading', { name: 'You’re All Set', level: 2 })).toBeVisible();
  await gate.getByRole('button', { name: 'Get Started' }).click();
}

test('client reset reopens first launch without touching the host or creating a server', async ({
  page,
}) => {
  await page.request.post('/__test/host-setup');
  await page.goto('/hosts/local-agent/servers/survival/handbook');
  await finishHostSetup(page);

  await expect(page.getByText('Begin the guided tour.')).toBeVisible();
  await page.getByRole('button', { name: 'Skip tour' }).click();

  await page.getByRole('button', { name: 'Preferences' }).click();
  const settings = page.getByRole('dialog', { name: 'MSC Settings' });
  await expect(settings).toBeVisible();
  await settings.getByRole('button', { name: 'Reset…' }).click();
  const resetSheet = page.getByRole('dialog', { name: 'Reset' });
  await resetSheet.getByRole('button', { name: 'Reset this client…' }).click();
  const confirmation = page.getByRole('alertdialog', { name: 'Reset this client?' });
  await confirmation.getByRole('button', { name: 'Reset this client' }).click();

  await expect(page.getByText('Begin the guided tour.')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Help and guides' })).toBeVisible();
  const countResponse = await page.request.get('/__test/server-create-count');
  expect(countResponse.ok()).toBe(true);
  expect(await countResponse.json()).toEqual({ count: 0 });
});
