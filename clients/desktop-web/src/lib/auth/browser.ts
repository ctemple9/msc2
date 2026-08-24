import type { TransportCredentialAdapter } from '../api/auth';

export type BrowserFetch = (input: string, init?: RequestInit) => Promise<Response>;
type Method = 'DELETE' | 'GET' | 'POST' | 'PUT';

/**
 * Keeps the browser's session secret in the httpOnly cookie jar. JavaScript
 * receives only the separate CSRF value needed to prove a mutation began in
 * this same agent-served page.
 */
export class BrowserSessionAuth {
  private csrfToken: string | null = null;

  constructor(
    private readonly baseUrl: string,
    private readonly fetchImpl: BrowserFetch = (input, init) => fetch(input, init),
  ) {}

  async exchangePairingCode(pairingCode: string): Promise<void> {
    const response = await this.fetchImpl(this.url('/v1/auth/browser-sessions'), {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pairingCode }),
    });
    if (!response.ok) throw new Error(`Browser pairing failed with HTTP ${response.status}`);
    this.csrfToken = null;
  }

  async logout(): Promise<void> {
    const response = await this.fetchImpl(this.url('/v1/auth/browser-sessions/current'), {
      method: 'DELETE',
      credentials: 'include',
      headers: await this.headersForMutation(),
    });
    if (!response.ok) throw new Error(`Browser logout failed with HTTP ${response.status}`);
    this.csrfToken = null;
  }

  credentialAdapter(): TransportCredentialAdapter {
    return {
      headersFor: async () => ({}),
      headersForRequest: async (_hostId, method) =>
        isSafe(method) ? {} : this.headersForMutation(),
      requestCredentials: 'include',
    };
  }

  private async headersForMutation(): Promise<Record<string, string>> {
    if (!this.csrfToken) {
      const response = await this.fetchImpl(this.url('/v1/auth/csrf'), {
        credentials: 'include',
        headers: { Accept: 'application/json' },
      });
      if (!response.ok) throw new Error(`Unable to load CSRF token (HTTP ${response.status})`);
      const body = (await response.json()) as { csrfToken?: unknown };
      if (typeof body.csrfToken !== 'string' || !body.csrfToken) {
        throw new Error('The agent returned an invalid CSRF token');
      }
      this.csrfToken = body.csrfToken;
    }
    return { 'X-MSC-CSRF': this.csrfToken };
  }

  private url(path: string): string {
    return `${this.baseUrl.replace(/\/$/, '')}${path}`;
  }
}

function isSafe(method: Method): boolean {
  return method === 'GET';
}
