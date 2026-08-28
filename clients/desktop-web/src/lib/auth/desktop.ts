import type { FetchLike, HttpMethod } from '../api/client';

export interface DesktopPairingResult {
  agentHostId: string;
}

export interface DesktopResponse {
  status: number;
  headers: readonly [string, string][];
  body: readonly number[];
}

/**
 * The only desktop-auth operations exposed to Svelte. There is deliberately
 * no `readToken`: native Rust keeps the bearer credential in the platform
 * store and adds it only while forwarding a request to that host's origin.
 */
export interface DesktopCredentialBridge {
  bootstrapLocal(): Promise<DesktopPairingResult>;
  exchangePairing(request: { baseUrl: string; pairingCode: string }): Promise<DesktopPairingResult>;
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

  async bootstrapLocal(): Promise<DesktopPairingResult> {
    return this.bridge.bootstrapLocal();
  }

  async forgetCredentials(
    hostIds: readonly string[],
    includeLocalHost = false,
  ): Promise<void> {
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
        headers: response.headers,
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

async function bodyBytes(body: BodyInit | null | undefined): Promise<Uint8Array | undefined> {
  if (body === undefined || body === null) return undefined;
  if (body instanceof Uint8Array) return body;
  return new Uint8Array(await new Response(body).arrayBuffer());
}
