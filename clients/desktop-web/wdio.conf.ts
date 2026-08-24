import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync } from 'node:fs';

const appBinary = process.env.MSC_TAURI_BINARY;
const tauriDriver = process.env.MSC_TAURI_DRIVER ?? 'tauri-driver';
let driver: ChildProcess | undefined;
let shuttingDown = false;

function stopDriver(): void {
  shuttingDown = true;
  driver?.kill('SIGTERM');
}

export const config: WebdriverIO.Config = {
  hostname: '127.0.0.1',
  port: 4444,
  specs: ['./tests/e2e/tauri-linux/**/*.test.ts'],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: appBinary,
      },
    },
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60_000,
  },
  onPrepare: () => {
    if (!appBinary || !existsSync(appBinary)) {
      throw new Error('MSC_TAURI_BINARY must name the built Linux Tauri binary.');
    }
  },
  beforeSession: () => {
    driver = spawn(tauriDriver, [], { stdio: 'inherit' });
    driver.once('error', (error) => {
      throw new Error(`Could not start ${tauriDriver}: ${error.message}`);
    });
    driver.once('exit', (code) => {
      if (!shuttingDown && code !== 0) {
        throw new Error(`${tauriDriver} exited unexpectedly with status ${code}.`);
      }
    });
  },
  afterSession: stopDriver,
  onComplete: stopDriver,
};
