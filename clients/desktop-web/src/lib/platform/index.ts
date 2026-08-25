import { isTauri } from '@tauri-apps/api/core';
import type { FetchLike } from '../api/client';
import { cookieCredentialAdapter, desktopCredentialAdapter } from '../api/auth';
import { DesktopSessionAuth, loadTauriDesktopCredentialBridge } from '../auth/desktop';
import { createBrowserPlatform } from './browser';
import { loadTauriPlatform } from './tauri';
import type {
  AgentReadiness,
  AgentServiceAction,
  AgentServiceStatus,
  PlatformAdapter,
} from './types';

export const LOCAL_AGENT_ORIGIN = 'http://127.0.0.1:48001';

export interface AgentTransport {
  readonly baseUrl: string;
  readonly fetchImpl?: FetchLike;
  readonly credentialAdapter: ReturnType<typeof cookieCredentialAdapter>;
}

export interface AgentPreparationPlatform {
  readonly kind: PlatformAdapter['kind'];
  agentServiceStatus(): Promise<AgentServiceStatus>;
  manageAgentService(action: AgentServiceAction): Promise<AgentServiceStatus>;
}

export interface AgentHealthCheckOptions {
  readonly attempts?: number;
  readonly delayMs?: number;
}

export class AgentHealthTimeoutError extends Error {
  constructor() {
    super('The local agent service is running but its health endpoint did not respond.');
    this.name = 'AgentHealthTimeoutError';
  }
}

export type {
  AgentAction,
  AgentReadiness,
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

/** Starts only an already-installed service, then waits for its HTTP listener. */
export async function prepareInstalledAgent(
  platform: AgentPreparationPlatform,
  healthCheck: () => Promise<boolean> = localAgentHealthCheck,
  options: AgentHealthCheckOptions = {},
): Promise<AgentServiceStatus> {
  let status = await platform.agentServiceStatus();
  if (status.state === 'stopped') {
    status = await platform.manageAgentService('start');
  }
  if (status.state !== 'running') return status;

  const attempts = options.attempts ?? 20;
  const delayMs = options.delayMs ?? 250;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    if (await healthCheck()) return status;
    if (attempt + 1 < attempts) await delay(delayMs);
  }
  throw new AgentHealthTimeoutError();
}

/** Browser users have no native service to prepare; Tauri users do. */
export async function prepareLocalAgent(): Promise<AgentServiceStatus | null> {
  const platform = await getPlatform();
  if (platform.kind !== 'tauri') return null;
  return prepareInstalledAgent(platform);
}

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

async function localAgentHealthCheck(): Promise<boolean> {
  try {
    const response = await fetch(`${LOCAL_AGENT_ORIGIN}/v1/health`, {
      credentials: 'omit',
    });
    return response.ok;
  } catch {
    return false;
  }
}

async function delay(milliseconds: number): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, milliseconds));
}
