import type { components } from '../api/generated';

export type HostId = string;
export type ConnectionStatus =
  'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error';

export const LOCAL_HOST_ID = 'local-agent';
export const DEFAULT_MANAGEMENT_PORT = 48001;
export const DEFAULT_LOCAL_FORWARDED_PORT = 48002;

/** A route family, ordered by the user's preferred connection path. */
export type HostRoute = 'lan' | 'tailscale';

/** The only SSH authentication choices a profile is allowed to remember. */
export type SshAuthentication = 'password' | 'private-key' | 'agent';

/** SSH connection metadata. Secret values are deliberately not representable here. */
export interface SshProfile {
  readonly hostname: string;
  readonly port: number;
  readonly username: string;
  readonly authentication: SshAuthentication;
  /** A path or OS key reference, never the private-key contents. */
  readonly privateKeyPath?: string;
}

export interface HostRecord {
  id: HostId;
  displayName: string;
  /** Hostnames, IP addresses, or complete HTTP(S) origins for direct access. */
  lanAddresses: readonly string[];
  tailscaleAddresses: readonly string[];
  preferredRouteOrder: readonly HostRoute[];
  ssh: SshProfile;
  /** The agent's remote management port; this is not the Minecraft server port. */
  managementPort: number;
  /** A client-side port for a future managed SSH forward. */
  localForwardedPort?: number;

  /** Accepted while reading old in-memory callers; never written by saved.ts. */
  readonly label?: string;
  readonly baseUrl?: string;
}

/** Input compatibility for callers written before the profile migration. */
export interface LegacyHostRecord {
  readonly id: HostId;
  readonly label: string;
  readonly baseUrl: string;
}

export type HostRecordInput = HostRecord | LegacyHostRecord;

export interface RemoteHostProfileInput {
  readonly id: HostId;
  readonly displayName: string;
  readonly baseUrl: string;
  readonly tailscaleAddresses?: readonly string[];
  readonly preferredRouteOrder?: readonly HostRoute[];
  readonly ssh?: Partial<SshProfile>;
  readonly managementPort?: number;
  readonly localForwardedPort?: number;
}

export function createLocalHostRecord(baseUrl: string): HostRecord {
  return {
    id: LOCAL_HOST_ID,
    displayName: 'Local agent',
    lanAddresses: [baseUrl],
    tailscaleAddresses: [],
    preferredRouteOrder: ['lan'],
    ssh: {
      hostname: '',
      port: 22,
      username: '',
      authentication: 'agent',
    },
    managementPort: portFromUrl(baseUrl) ?? DEFAULT_MANAGEMENT_PORT,
  };
}

export function createRemoteHostRecord(input: RemoteHostProfileInput): HostRecord {
  const parsedAddress = parseAddress(input.baseUrl);
  return {
    id: input.id,
    displayName: input.displayName.trim(),
    lanAddresses: [input.baseUrl.trim()],
    tailscaleAddresses: [...(input.tailscaleAddresses ?? [])],
    preferredRouteOrder: [...(input.preferredRouteOrder ?? ['lan', 'tailscale'])],
    ssh: {
      hostname: input.ssh?.hostname?.trim() || parsedAddress?.hostname || '',
      port: input.ssh?.port ?? 22,
      username: input.ssh?.username?.trim() ?? '',
      authentication: input.ssh?.authentication ?? 'agent',
      ...(input.ssh?.privateKeyPath?.trim()
        ? { privateKeyPath: input.ssh.privateKeyPath.trim() }
        : {}),
    },
    managementPort: input.managementPort ?? portFromUrl(input.baseUrl) ?? DEFAULT_MANAGEMENT_PORT,
    ...(input.localForwardedPort === undefined
      ? { localForwardedPort: DEFAULT_LOCAL_FORWARDED_PORT }
      : { localForwardedPort: input.localForwardedPort }),
  };
}

/**
 * Converts both the current profile and the pre-P14.11 `{ id, label, baseUrl }`
 * shape into one canonical record. This is the only migration entry point, so
 * localStorage and in-memory callers cannot quietly grow different schemas.
 */
export function normalizeHostRecord(value: unknown): HostRecord | null {
  if (!value || typeof value !== 'object') return null;
  const record = value as Record<string, unknown>;
  const id = stringValue(record.id);
  const displayName = stringValue(record.displayName) ?? stringValue(record.label);
  if (!id || !displayName) return null;

  const oldBaseUrl = stringValue(record.baseUrl);
  const lanAddresses = stringArray(record.lanAddresses).filter(isAddress);
  const tailscaleAddresses = stringArray(record.tailscaleAddresses).filter(isAddress);
  if (!lanAddresses.length && oldBaseUrl && isAddress(oldBaseUrl)) lanAddresses.push(oldBaseUrl);
  if (!lanAddresses.length && !tailscaleAddresses.length) return null;

  const preferredRouteOrder = routeOrder(
    record.preferredRouteOrder,
    lanAddresses,
    tailscaleAddresses,
  );
  const sshRecord = objectValue(record.ssh);
  const firstAddress = [...lanAddresses, ...tailscaleAddresses][0] ?? '';
  const parsedAddress = parseAddress(firstAddress);
  const authentication = sshAuthentication(sshRecord?.authentication) ?? 'agent';
  const managementPort =
    validPort(record.managementPort) ?? portFromUrl(firstAddress) ?? DEFAULT_MANAGEMENT_PORT;
  const localForwardedPort =
    validPort(record.localForwardedPort) ??
    (id === LOCAL_HOST_ID ? undefined : DEFAULT_LOCAL_FORWARDED_PORT);

  return {
    id,
    displayName,
    lanAddresses,
    tailscaleAddresses,
    preferredRouteOrder,
    ssh: {
      hostname: stringValue(sshRecord?.hostname) ?? parsedAddress?.hostname ?? '',
      port: validPort(sshRecord?.port) ?? 22,
      username: stringValue(sshRecord?.username) ?? '',
      authentication,
      ...(stringValue(sshRecord?.privateKeyPath)
        ? { privateKeyPath: stringValue(sshRecord?.privateKeyPath) }
        : {}),
    },
    managementPort,
    ...(localForwardedPort === undefined ? {} : { localForwardedPort }),
  };
}

/** Returns the first address in the stored preference order. */
export function preferredHostAddress(host: HostRecord): string | null {
  for (const route of host.preferredRouteOrder) {
    const addresses = route === 'lan' ? host.lanAddresses : host.tailscaleAddresses;
    const address = addresses.find((candidate) => candidate.trim());
    if (address) return address.trim();
  }
  return (
    [...host.lanAddresses, ...host.tailscaleAddresses].find((address) => address.trim())?.trim() ??
    null
  );
}

/** Builds the direct agent origin without changing a complete URL's scheme or port. */
export function hostManagementUrl(host: HostRecord): string {
  const address = preferredHostAddress(host);
  if (!address) return `http://127.0.0.1:${host.managementPort}`;
  const parsed = parseAddress(address);
  if (parsed?.origin) return parsed.origin;
  const formattedHost =
    address.includes(':') && !address.startsWith('[') ? `[${address}]` : address;
  return `http://${formattedHost}:${host.managementPort}`;
}

export function hostAddressSummary(host: HostRecord): string {
  return preferredHostAddress(host) ?? 'No direct address saved';
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

function stringValue(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

function stringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return [
    ...new Set(
      value
        .filter((item): item is string => typeof item === 'string' && item.trim().length > 0)
        .map((item) => item.trim()),
    ),
  ];
}

function objectValue(value: unknown): Record<string, unknown> | undefined {
  return value && typeof value === 'object' ? (value as Record<string, unknown>) : undefined;
}

function routeOrder(
  value: unknown,
  lanAddresses: readonly string[],
  tailscaleAddresses: readonly string[],
): HostRoute[] {
  const requested = Array.isArray(value)
    ? value.filter((route): route is HostRoute => route === 'lan' || route === 'tailscale')
    : [];
  const order = [...new Set(requested)];
  if (lanAddresses.length && !order.includes('lan')) order.push('lan');
  if (tailscaleAddresses.length && !order.includes('tailscale')) order.push('tailscale');
  return order.length ? order : ['lan', 'tailscale'];
}

function sshAuthentication(value: unknown): SshAuthentication | undefined {
  return value === 'password' || value === 'private-key' || value === 'agent' ? value : undefined;
}

function validPort(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isInteger(value) && value >= 1 && value <= 65535
    ? value
    : undefined;
}

function parseAddress(value: string): { hostname: string; origin?: string } | null {
  try {
    const hasScheme = value.includes('://');
    const bareAddress =
      value.includes(':') && !value.startsWith('[') && value.split(':').length > 2
        ? `[${value}]`
        : value;
    const url = new URL(hasScheme ? value : `http://${bareAddress}`);
    if (!url.hostname) return null;
    if (hasScheme && !['http:', 'https:'].includes(url.protocol)) return null;
    url.pathname = '';
    url.search = '';
    url.hash = '';
    return { hostname: url.hostname, origin: hasScheme || !!url.port ? url.origin : undefined };
  } catch {
    return null;
  }
}

function isAddress(value: string): boolean {
  return parseAddress(value) !== null;
}

function portFromUrl(value: string): number | undefined {
  try {
    const url = new URL(value);
    return url.port ? validPort(Number(url.port)) : undefined;
  } catch {
    return undefined;
  }
}
