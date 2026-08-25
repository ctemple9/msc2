import { isTauri } from '@tauri-apps/api/core';
import { createBrowserPlatform } from './browser';
import { loadTauriPlatform } from './tauri';
import type { PlatformAdapter } from './types';

export type {
  AgentAction,
  AgentServiceAction,
  AgentServiceStatus,
  DesktopNotification,
  FilePickerRequest,
  MenuEntry,
  PickedFile,
  PlatformAdapter,
  TauriPlatformDependencies,
} from './types';
export { createBrowserPlatform } from './browser';
export { createTauriPlatform } from './tauri';

let platform: Promise<PlatformAdapter> | undefined;

/** The only runtime check for the desktop shell; screens remain platform-neutral. */
export function getPlatform(): Promise<PlatformAdapter> {
  platform ??= isTauri()
    ? loadTauriPlatform().catch(() => createBrowserPlatform())
    : Promise.resolve(createBrowserPlatform());
  return platform;
}

export async function openExternal(url: string): Promise<void> {
  await (await getPlatform()).openExternal(url);
}
