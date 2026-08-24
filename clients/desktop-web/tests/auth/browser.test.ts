import { describe, expect, it } from 'vitest';
import { BrowserSessionAuth } from '../../src/lib/auth/browser';
import { ApiClient } from '../../src/lib/api/client';

describe('browser session authentication', () => {
  it('keeps pairing in the cookie jar and adds the CSRF value only to mutations', async () => {
    const calls: Array<{ url: string; init?: RequestInit }> = [];
    const fetchImpl = async (url: string, init?: RequestInit): Promise<Response> => {
      calls.push({ url, init });
      if (url.endsWith('/v1/auth/csrf')) {
        return new Response(JSON.stringify({ csrfToken: 'csrf-value', expiresAt: 'later' }), {
          headers: { 'content-type': 'application/json' },
        });
      }
      return new Response(JSON.stringify({ result: 'ok' }), {
        headers: { 'content-type': 'application/json' },
      });
    };
    const session = new BrowserSessionAuth('http://agent.test', fetchImpl);
    const client = new ApiClient({
      baseUrl: 'http://agent.test',
      hostId: 'agent',
      fetchImpl,
      credentialAdapter: session.credentialAdapter(),
    });

    await session.exchangePairingCode('pairing-code');
    await client.requestJson('GET', '/v1/capabilities');
    await client.requestJson('POST', '/v1/start');

    expect(calls[0]).toMatchObject({
      url: 'http://agent.test/v1/auth/browser-sessions',
      init: { credentials: 'include' },
    });
    expect(calls[1].init?.headers).not.toMatchObject({ 'X-MSC-CSRF': expect.anything() });
    expect(calls[2]).toMatchObject({ url: 'http://agent.test/v1/auth/csrf' });
    expect(calls[3].init?.headers).toMatchObject({ 'X-MSC-CSRF': 'csrf-value' });
  });

  it('does not reuse a cached CSRF token after logout', async () => {
    const calls: Array<{ url: string; init?: RequestInit }> = [];
    const fetchImpl = async (url: string, init?: RequestInit): Promise<Response> => {
      calls.push({ url, init });
      if (url.endsWith('/v1/auth/csrf')) {
        return new Response(JSON.stringify({ csrfToken: `csrf-${calls.length}`, expiresAt: 'later' }), {
          headers: { 'content-type': 'application/json' },
        });
      }
      return new Response(null, { status: 204 });
    };
    const session = new BrowserSessionAuth('http://agent.test', fetchImpl);

    await session.logout();
    const adapter = session.credentialAdapter();
    await adapter.headersForRequest?.('agent', 'POST');

    expect(calls[0]).toMatchObject({ url: 'http://agent.test/v1/auth/csrf' });
    expect(calls[1].init?.headers).toMatchObject({ 'X-MSC-CSRF': 'csrf-1' });
    expect(calls[1]).toMatchObject({ url: 'http://agent.test/v1/auth/browser-sessions/current' });
    expect(calls[2]).toMatchObject({ url: 'http://agent.test/v1/auth/csrf' });
    expect(await adapter.headersForRequest?.('agent', 'POST')).toEqual({ 'X-MSC-CSRF': 'csrf-3' });
  });
});
