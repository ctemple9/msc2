import {
  anonymousCredentialAdapter,
  emptyHostCache,
  type CredentialAdapter,
  type DestructiveConfirmation,
  type HostCache,
  type HostId,
  type HostRecord,
  type HostState,
} from './types';
import type { components } from '../api/generated';

const MAX_CONSOLE_LINES = 200;
const MAX_OPERATIONS = 100;
const MAX_NOTIFICATIONS = 200;

export interface HostStoreOptions {
  credentialAdapter?: CredentialAdapter;
  now?: () => number;
}

/**
 * Keeps every piece of connection data below its host ID. The selected host is
 * only navigation state; it is never used as a substitute for a cache key.
 */
export class HostStore {
  private readonly hosts = new Map<HostId, HostRecord>();
  private readonly caches = new Map<HostId, HostCache>();
  private readonly credentialAdapter: CredentialAdapter;
  private readonly now: () => number;
  private selectedHostId: HostId | null = null;

  constructor(options: HostStoreOptions = {}) {
    this.credentialAdapter = options.credentialAdapter ?? anonymousCredentialAdapter;
    this.now = options.now ?? Date.now;
  }

  addHost(record: HostRecord): void {
    validateHost(record);
    if (this.hosts.has(record.id)) {
      throw new Error(`Host '${record.id}' is already registered`);
    }
    this.hosts.set(record.id, { ...record });
    this.caches.set(record.id, emptyHostCache());
    this.selectedHostId ??= record.id;
  }

  updateHost(record: HostRecord): void {
    validateHost(record);
    if (!this.hosts.has(record.id)) {
      throw new Error(`Unknown host '${record.id}'`);
    }
    this.hosts.set(record.id, { ...record });
  }

  removeHost(hostId: HostId): void {
    this.requireHost(hostId);
    this.hosts.delete(hostId);
    this.caches.delete(hostId);
    if (this.selectedHostId === hostId) {
      this.selectedHostId = this.hosts.keys().next().value ?? null;
    }
  }

  listHosts(): readonly HostRecord[] {
    return [...this.hosts.values()].map((host) => ({ ...host }));
  }

  selectHost(hostId: HostId): void {
    this.requireHost(hostId);
    this.selectedHostId = hostId;
  }

  get selectedHost(): HostId | null {
    return this.selectedHostId;
  }

  getState(hostId: HostId): HostState {
    const host = this.requireHost(hostId);
    const cache = this.requireCache(hostId);
    return { host: { ...host }, cache: cloneCache(cache) };
  }

  getSelectedState(): HostState | null {
    return this.selectedHostId ? this.getState(this.selectedHostId) : null;
  }

  updateConnection(hostId: HostId, connection: HostCache['connection'], error = null): void {
    this.updateCache(hostId, { connection, error });
  }

  setCapabilities(
    hostId: HostId,
    capabilities: components['schemas']['CapabilitiesDTO'] | null,
  ): void {
    this.updateCache(hostId, {
      capabilities: capabilities ? structuredClone(capabilities) : null,
      permissions: capabilities?.permissions ?? this.requireCache(hostId).permissions,
    });
  }

  setPermissions(hostId: HostId, permissions: readonly string[]): void {
    this.updateCache(hostId, { permissions: [...permissions] });
  }

  setServers(hostId: HostId, servers: readonly components['schemas']['ServerDTO'][]): void {
    const cache = this.requireCache(hostId);
    const copied = structuredClone(servers);
    const activeStillExists = copied.some((server) => server.id === cache.activeServerId);
    this.updateCache(hostId, {
      servers: copied,
      activeServerId: activeStillExists ? cache.activeServerId : (copied[0]?.id ?? null),
    });
  }

  selectServer(hostId: HostId, serverId: string): void {
    const cache = this.requireCache(hostId);
    if (!cache.servers.some((server) => server.id === serverId)) {
      throw new Error(`Server '${serverId}' is not known for host '${hostId}'`);
    }
    this.updateCache(hostId, { activeServerId: serverId });
  }

  appendConsole(hostId: HostId, line: components['schemas']['ConsoleLineDTO']): void {
    const lines = [...this.requireCache(hostId).consoleLines, structuredClone(line)].slice(
      -MAX_CONSOLE_LINES,
    );
    this.updateCache(hostId, { consoleLines: lines });
  }

  replaceConsole(hostId: HostId, lines: readonly components['schemas']['ConsoleLineDTO'][]): void {
    this.updateCache(hostId, { consoleLines: structuredClone(lines).slice(-MAX_CONSOLE_LINES) });
  }

  upsertOperation(hostId: HostId, operation: components['schemas']['OperationDTO']): void {
    const operations = new Map(
      this.requireCache(hostId).operations.map((current) => [current.id, current]),
    );
    operations.set(operation.id, structuredClone(operation));
    this.updateCache(hostId, { operations: [...operations.values()].slice(-MAX_OPERATIONS) });
  }

  appendNotification(
    hostId: HostId,
    notification: components['schemas']['NotificationEventDTO'],
  ): void {
    const notifications = [
      ...this.requireCache(hostId).notifications,
      structuredClone(notification),
    ].slice(-MAX_NOTIFICATIONS);
    this.updateCache(hostId, { notifications });
  }

  makeDestructiveConfirmation(
    hostId: HostId,
    action: string,
    serverId: string | null = null,
  ): DestructiveConfirmation {
    this.requireHost(hostId);
    if (!action.trim()) {
      throw new Error('A destructive confirmation needs an action');
    }
    return Object.freeze({ action, hostId, serverId, issuedAt: this.now() });
  }

  assertDestructiveConfirmation(
    confirmation: DestructiveConfirmation,
    hostId: HostId,
    serverId: string | null = null,
  ): void {
    this.requireHost(hostId);
    if (confirmation.hostId !== hostId || confirmation.serverId !== serverId) {
      throw new Error('Destructive confirmation belongs to a different host or server');
    }
  }

  async credentialHeaders(hostId: HostId): Promise<Readonly<Record<string, string>>> {
    return this.credentialAdapter.headersFor(this.requireHost(hostId));
  }

  requestCredentials(): RequestCredentials | undefined {
    return this.credentialAdapter.requestCredentials;
  }

  private updateCache(hostId: HostId, patch: Partial<HostCache>): void {
    const current = this.requireCache(hostId);
    this.caches.set(hostId, { ...current, ...patch });
  }

  private requireHost(hostId: HostId): HostRecord {
    const host = this.hosts.get(hostId);
    if (!host) {
      throw new Error(`Unknown host '${hostId}'`);
    }
    return host;
  }

  private requireCache(hostId: HostId): HostCache {
    const cache = this.caches.get(hostId);
    if (!cache) {
      throw new Error(`Host '${hostId}' has no cache`);
    }
    return cache;
  }
}

function validateHost(host: HostRecord): void {
  if (!host.id.trim() || !host.label.trim()) {
    throw new Error('A host needs a non-empty id and label');
  }
  try {
    new URL(host.baseUrl);
  } catch {
    throw new Error(`Host '${host.id}' has an invalid base URL`);
  }
}

function cloneCache(cache: HostCache): HostCache {
  return structuredClone(cache);
}
