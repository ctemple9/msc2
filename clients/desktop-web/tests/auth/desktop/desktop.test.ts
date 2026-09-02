import { describe, expect, it, vi } from 'vitest';
import { ApiClient } from '../../../src/lib/api';
import { DesktopSessionAuth, type DesktopCredentialBridge } from '../../../src/lib/auth/desktop';

function bridge(): DesktopCredentialBridge {
  return {
    bootstrapLocal: vi.fn(async () => ({ agentHostId: 'agent-local' })),
    exchangePairing: vi.fn(async () => ({ agentHostId: 'agent-beta' })),
    forgetCredentials: vi.fn(async () => undefined),
    authorizedRequest: vi.fn(async () => ({
      status: 200,
      headers: [['X-MSC-Api-Version', '1.0']],
      body: [...new TextEncoder().encode('{"value":"beta"}')],
    })),
  };
}

describe('desktop credentials', () => {
  it('redeems a remote code without receiving its bearer token in Svelte', async () => {
    const native = bridge();
    const session = new DesktopSessionAuth(native);

    await expect(
      session.redeemRemotePairing('https://beta.example', 'pairing-code'),
    ).resolves.toEqual({
      agentHostId: 'agent-beta',
    });
    expect(native.exchangePairing).toHaveBeenCalledWith({
      baseUrl: 'https://beta.example',
      pairingCode: 'pairing-code',
    });
  });

  it('keeps requests with each returned host ID so a switch cannot reuse another host token', async () => {
    const native = bridge();
    const session = new DesktopSessionAuth(native);
    const beta = new ApiClient({
      baseUrl: 'https://beta.example',
      hostId: 'agent-beta',
      fetchImpl: session.fetchForHost('agent-beta'),
    });
    const alpha = new ApiClient({
      baseUrl: 'https://alpha.example',
      hostId: 'agent-alpha',
      fetchImpl: session.fetchForHost('agent-alpha'),
    });

    await beta.requestJson('GET', '/v1/me');
    await alpha.requestJson('GET', '/v1/me');

    expect(native.authorizedRequest).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({ agentHostId: 'agent-beta', path: '/v1/me' }),
    );
    expect(native.authorizedRequest).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({ agentHostId: 'agent-alpha', path: '/v1/me' }),
    );
    expect(JSON.stringify(native.authorizedRequest.mock.calls)).not.toContain('msc2_');
  });

  it('forwards JSON request bodies through the native desktop transport', async () => {
    const native = bridge();
    const session = new DesktopSessionAuth(native);
    const client = new ApiClient({
      baseUrl: 'https://beta.example',
      hostId: 'agent-beta',
      fetchImpl: session.fetchForHost('agent-beta'),
    });

    await client.requestJson('POST', '/v1/config/ram', {
      body: { minRamGB: 2, maxRamGB: 4.5 },
    });

    const request = native.authorizedRequest.mock.calls[0]?.[0];
    expect(request?.path).toBe('/v1/config/ram');
    expect(JSON.parse(new TextDecoder().decode(request?.body))).toEqual({
      minRamGB: 2,
      maxRamGB: 4.5,
    });
  });

  it('forgets only the requested host credentials and can include the local bootstrap record', async () => {
    const native = bridge();
    const session = new DesktopSessionAuth(native);

    await session.forgetCredentials(['agent-beta'], true);

    expect(native.forgetCredentials).toHaveBeenCalledWith({
      hostIds: ['agent-beta'],
      includeLocalHost: true,
    });
  });
});
