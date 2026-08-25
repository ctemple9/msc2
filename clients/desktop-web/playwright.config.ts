import { defineConfig, devices } from '@playwright/test';

const e2ePort = Number(process.env.MSC_E2E_PORT ?? '4173');

export default defineConfig({
  testDir: './tests/e2e/browser',
  timeout: 30_000,
  use: { baseURL: `http://127.0.0.1:${e2ePort}`, trace: 'retain-on-failure' },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
  ],
  webServer: {
    command: 'node ./tests/e2e/browser/contract-harness.mjs',
    env: { PORT: String(e2ePort) },
    port: e2ePort,
    reuseExistingServer: false,
  },
});
