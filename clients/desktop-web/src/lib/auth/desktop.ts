import type { FetchLike, HttpMethod } from '../api/client';
import type { SshAuthentication } from '../hosts/types';

export interface DesktopPairingResult {
  agentHostId: string;
}

export interface RemoteDesktopPairingRequest {
  baseUrl: string;
  ssh: {
    sshHost: string;
    sshPort: number;
    username: string;
    authentication: SshAuthentication;
    privateKeyPath?: string;
    /** Passed to native code for one connection attempt; never persisted. */
    password?: string;
    localPort: number;
    remotePort: number;
    expectedHostKeyFingerprint?: string;
    rememberHostKey: boolean;
  };
}

export type RemoteDesktopPairingState =
  | 'paired'
  | 'awaiting-host-key'
  | 'host-key-changed'
  | 'host-key-mismatch';

export interface RemoteDesktopPairingResult {
  state: RemoteDesktopPairingState;
  agentHostId: string | null;
  hostKeyFingerprint: string | null;
  storedHostKeyFingerprint: string | null;
  detail: string;
}

export interface DesktopResponse {
  status: number;
  headers: readonly [string, string][];
  body: readonly number[];
}

export type SshTunnelState =
  | 'awaiting-host-key'
  | 'host-key-changed'
  | 'host-key-mismatch'
  | 'connecting'
  | 'connected'
  | 'failed'
  | 'stopped';

export interface SshTunnelStatus {
  hostId: string;
  state: SshTunnelState;
  localPort: number;
  remotePort: number;
  hostKeyFingerprint: string | null;
  storedHostKeyFingerprint: string | null;
  stderr: string;
  exitReason: string | null;
  recoverable: boolean;
}

export interface SshTunnelRequest {
  hostId: string;
  sshHost: string;
  sshPort: number;
  username: string;
  authentication: SshAuthentication;
  privateKeyPath?: string;
  /** Passed to native code for one connection attempt; never persisted. */
  password?: string;
  localPort: number;
  remotePort: number;
  /** Required after the user reviews an unknown or changed fingerprint. */
  expectedHostKeyFingerprint?: string;
  rememberHostKey: boolean;
}

export interface DesktopSshTunnelBridge {
  start(request: SshTunnelRequest): Promise<SshTunnelStatus>;
  status(hostId: string): Promise<SshTunnelStatus>;
  retry(hostId: string): Promise<SshTunnelStatus>;
  stop(hostId: string): Promise<SshTunnelStatus>;
}

/**
 * The only desktop-auth operations exposed to Svelte. There is deliberately
 * no `readToken`: native Rust keeps the bearer credential in the platform
 * store and adds it only while forwarding a request to that host's origin.
 */
export interface DesktopCredentialBridge {
  bootstrapLocal(): Promise<DesktopPairingResult>;
  exchangePairing(request: { baseUrl: string; pairingCode: string }): Promise<DesktopPairingResult>;
  automateRemotePairing?: (
    request: RemoteDesktopPairingRequest,
  ) => Promise<RemoteDesktopPairingResult>;
  forgetCredentials(request: {
    hostIds: readonly string[];
    includeLocalHost: boolean;
  }): Promise<void>;
  authorizedRequest(request: {
    agentHostId: string;
    method: HttpMethod;
    path: string;
    headers: readonly [string, string][];
    body?: Uint8Array;
  }): Promise<DesktopResponse>;
}

export class DesktopSessionAuth {
  constructor(private readonly bridge: DesktopCredentialBridge) {}

  async redeemRemotePairing(baseUrl: string, pairingCode: string): Promise<DesktopPairingResult> {
    return this.bridge.exchangePairing({ baseUrl, pairingCode });
  }

  async automateRemotePairing(
    request: RemoteDesktopPairingRequest,
  ): Promise<RemoteDesktopPairingResult> {
    if (!this.bridge.automateRemotePairing) {
      throw new Error('This desktop shell cannot automate remote pairing.');
    }
    return this.bridge.automateRemotePairing(request);
  }

  async bootstrapLocal(): Promise<DesktopPairingResult> {
    return this.bridge.bootstrapLocal();
  }

  async forgetCredentials(hostIds: readonly string[], includeLocalHost = false): Promise<void> {
    await this.bridge.forgetCredentials({ hostIds, includeLocalHost });
  }

  /**
   * Builds an ApiClient-compatible fetch boundary for exactly one agent host.
   * Full URLs are reduced to their path before crossing into Rust; the shell
   * checks that path against the origin stored with this host's credential.
   */
  fetchForHost(agentHostId: string): FetchLike {
    return async (input, init = {}) => {
      const url = new URL(input);
      const headers = new Headers(init.headers);
      const body = await bodyBytes(init.body);
      const response = await this.bridge.authorizedRequest({
        agentHostId,
        method: (init.method ?? 'GET') as HttpMethod,
        path: `${url.pathname}${url.search}`,
        headers: [...headers.entries()],
        body,
      });
      return new Response(new Uint8Array(response.body), {
        status: response.status,
        headers: response.headers.map(([name, value]) => [name, value] as [string, string]),
      });
    };
  }
}

/** Loads the native bridge lazily, so the browser bundle has no shell token path. */
export async function loadTauriDesktopCredentialBridge(): Promise<DesktopCredentialBridge> {
  const { invoke } = await import('@tauri-apps/api/core');
  return {
    bootstrapLocal: () => invoke<DesktopPairingResult>('desktop_bootstrap_local'),
    exchangePairing: (request) =>
      invoke<DesktopPairingResult>('desktop_exchange_pairing', { request }),
    automateRemotePairing: (request) =>
      invoke<RemoteDesktopPairingResult>('desktop_automate_remote_pairing', { request }),
    forgetCredentials: (request) =>
      invoke<void>('desktop_forget_credentials', {
        request: { ...request, hostIds: [...request.hostIds] },
      }),
    authorizedRequest: (request) =>
      invoke<DesktopResponse>('desktop_authorized_request', {
        request: { ...request, body: request.body ? [...request.body] : null },
      }),
  };
}

/** Loads the native managed-forwarding seam without exposing Tauri to screens. */
export async function loadTauriSshTunnelBridge(): Promise<DesktopSshTunnelBridge> {
  const { invoke } = await import('@tauri-apps/api/core');
  return {
    start: (request) => invoke<SshTunnelStatus>('ssh_tunnel_start', { request }),
    status: (hostId) => invoke<SshTunnelStatus>('ssh_tunnel_status', { hostId }),
    retry: (hostId) => invoke<SshTunnelStatus>('ssh_tunnel_retry', { hostId }),
    stop: (hostId) => invoke<SshTunnelStatus>('ssh_tunnel_stop', { hostId }),
  };
}

async function bodyBytes(body: BodyInit | null | undefined): Promise<Uint8Array | undefined> {
  if (body === undefined || body === null) return undefined;
  if (body instanceof Uint8Array) return body;
  return new Uint8Array(await new Response(body).arrayBuffer());
}
