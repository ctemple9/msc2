import { isTauri } from '@tauri-apps/api/core';
import type { FetchLike } from '../api/client';
import { cookieCredentialAdapter, desktopCredentialAdapter } from '../api/auth';
import { DesktopSessionAuth, loadTauriDesktopCredentialBridge } from '../auth/desktop';
import { createBrowserPlatform } from './browser';
import { loadTauriPlatform } from './tauri';
import type { PlatformAdapter } from './types';

export const LOCAL_AGENT_ORIGIN = 'http://127.0.0.1:48001';

export interface AgentTransport {
  readonly baseUrl: string;
  readonly fetchImpl?: FetchLike;
  readonly credentialAdapter: ReturnType<typeof cookieCredentialAdapter>;
}

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

/** Selects authentication at the shell boundary, before ApiClient is built. */
export async function createAgentTransport(hostId: string): Promise<AgentTransport> {
  const configuredBaseUrl = import.meta.env.VITE_MSC_API_BASE_URL;
  if (isTauri()) {
    const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
    return {
      baseUrl: configuredBaseUrl ?? LOCAL_AGENT_ORIGIN,
      fetchImpl: auth.fetchForHost(hostId),
      credentialAdapter: desktopCredentialAdapter(),
    };
  }

  return {
    baseUrl:
      configuredBaseUrl ??
      (typeof window === 'undefined' ? 'http://127.0.0.1' : window.location.origin),
    credentialAdapter: cookieCredentialAdapter(),
  };
}

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
