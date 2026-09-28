import { isTauri } from '@tauri-apps/api/core';
import type { FetchLike } from '../api/client';
import { desktopCredentialAdapter, type TransportCredentialAdapter } from '../api/auth';
import { DesktopSessionAuth, loadTauriDesktopCredentialBridge } from '../auth/desktop';
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
  readonly hostId: string;
  readonly fetchImpl?: FetchLike;
  readonly credentialAdapter: TransportCredentialAdapter;
}

export interface AgentPreparationPlatform {
  readonly kind: PlatformAdapter['kind'];
  agentHealthCheck(): Promise<boolean>;
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
  AgentReadiness,
  AgentServiceAction,
  AgentServiceStatus,
  DesktopNotification,
  FilePickerRequest,
  MenuEntry,
  PickedFile,
  PlatformAdapter,
  UpdateCheckResult,
  UpdateInstallResult,
  TauriPlatformDependencies,
} from './types';
export { createTauriPlatform } from './tauri';

/** Reports an installed service without changing an explicit stopped state. */
export async function prepareInstalledAgent(
  platform: AgentPreparationPlatform,
  healthCheck: () => Promise<boolean> = localAgentHealthCheck,
  options: AgentHealthCheckOptions = {},
): Promise<AgentServiceStatus> {
  const status = await platform.agentServiceStatus();
  if (status.state !== 'running') return status;

  const attempts = options.attempts ?? 20;
  const delayMs = options.delayMs ?? 250;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    if (await healthCheck()) return status;
    if (attempt + 1 < attempts) await delay(delayMs);
  }
  throw new AgentHealthTimeoutError();
}

/** Reports the locally installed service without changing its stopped state. */
export async function prepareLocalAgent(): Promise<AgentServiceStatus> {
  const platform = await getPlatform();
  return prepareInstalledAgent(platform, () => platform.agentHealthCheck());
}

/** Selects authentication at the shell boundary, before ApiClient is built. */
export async function createAgentTransport(hostId: string): Promise<AgentTransport> {
  const configuredBaseUrl = import.meta.env.VITE_MSC_API_BASE_URL;
  const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
  const authenticatedHostId =
    hostId === 'local-agent' ? (await auth.bootstrapLocal()).agentHostId : hostId;
  return {
    baseUrl: configuredBaseUrl ?? LOCAL_AGENT_ORIGIN,
    hostId: authenticatedHostId,
    fetchImpl: auth.fetchForHost(authenticatedHostId),
    credentialAdapter: desktopCredentialAdapter(),
  };
}

let platform: Promise<PlatformAdapter> | undefined;

/** Loads the native adapter; the shared frontend is shipped inside Tauri only. */
export function getPlatform(): Promise<PlatformAdapter> {
  if (!isTauri()) {
    throw new Error('The MSC desktop client must run inside its Tauri shell.');
  }
  platform ??= loadTauriPlatform();
  return platform;
}

export async function openExternal(url: string): Promise<void> {
  await (await getPlatform()).openExternal(url);
}

async function localAgentHealthCheck(): Promise<boolean> {
  try {
    const response = await fetch(`${LOCAL_AGENT_ORIGIN}/v1/healthz`, {
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
