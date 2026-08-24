import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e/browser',
  timeout: 30_000,
  use: { baseURL: 'http://127.0.0.1:4173', trace: 'retain-on-failure' },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
  webServer: {
    command: 'node ./tests/e2e/browser/contract-harness.mjs',
    port: 4173,
    reuseExistingServer: false,
  },
});
