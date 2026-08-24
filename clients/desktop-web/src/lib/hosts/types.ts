import type { components } from '../api/generated';

export type HostId = string;
export type ConnectionStatus =
  'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error';

export interface HostRecord {
  id: HostId;
  label: string;
  baseUrl: string;
}

export interface CredentialAdapter {
  headersFor(host: HostRecord): Promise<Readonly<Record<string, string>>>;
  requestCredentials?: RequestCredentials;
}

export const anonymousCredentialAdapter: CredentialAdapter = {
  headersFor: async () => ({}),
};

export interface HostCache {
  readonly connection: ConnectionStatus;
  readonly capabilities: components['schemas']['CapabilitiesDTO'] | null;
  readonly permissions: readonly string[];
  readonly servers: readonly components['schemas']['ServerDTO'][];
  readonly activeServerId: string | null;
  readonly consoleLines: readonly components['schemas']['ConsoleLineDTO'][];
  readonly operations: readonly components['schemas']['OperationDTO'][];
  readonly notifications: readonly components['schemas']['NotificationEventDTO'][];
  readonly error: components['schemas']['ErrorDTO'] | null;
}

export interface HostState {
  readonly host: HostRecord;
  readonly cache: HostCache;
}

export interface DestructiveConfirmation {
  readonly action: string;
  readonly hostId: HostId;
  readonly serverId: string | null;
  readonly issuedAt: number;
}

export function emptyHostCache(): HostCache {
  return {
    connection: 'disconnected',
    capabilities: null,
    permissions: [],
    servers: [],
    activeServerId: null,
    consoleLines: [],
    operations: [],
    notifications: [],
    error: null,
  };
}
