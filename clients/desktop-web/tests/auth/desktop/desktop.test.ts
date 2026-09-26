import { describe, expect, it, vi } from 'vitest';
import { ApiClient } from '../../../src/lib/api';
import {
  DesktopSessionAuth,
  type DesktopCredentialBridge,
  type DesktopResponse,
} from '../../../src/lib/auth/desktop';

type AuthorizedRequest = Parameters<DesktopCredentialBridge['authorizedRequest']>[0];

function bridge() {
  return {
    bootstrapLocal: vi.fn(async () => ({ agentHostId: 'agent-local' })),
    exchangePairing: vi.fn(async () => ({ agentHostId: 'agent-beta' })),
    forgetCredentials: vi.fn(async () => undefined),
    authorizedRequest: vi.fn(async (_request: AuthorizedRequest): Promise<DesktopResponse> => ({
      status: 200,
      headers: [['X-MSC-Api-Version', '1.0'] as [string, string]],
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
    expect(JSON.stringify(vi.mocked(native.authorizedRequest).mock.calls)).not.toContain('msc2_');
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

    const calls = vi.mocked(native.authorizedRequest).mock.calls as unknown as Array<
      [AuthorizedRequest]
    >;
    const request = calls[0]?.[0];
    expect(request?.path).toBe('/v1/config/ram');
    expect(JSON.parse(new TextDecoder().decode(request?.body))).toEqual({
      minRamGB: 2,
      maxRamGB: 4.5,
    });
  });

  it('streams a remote modpack through bodyless intermediate chunk responses', async () => {
    const chunkSize = 2 * 1024 * 1024;
    const totalBytes = chunkSize + 3;
    const native = bridge();
    vi.mocked(native.authorizedRequest).mockImplementation(async (request: AuthorizedRequest) => {
      if (request.path === '/v1/staged-uploads') {
        return {
          status: 200,
          headers: [['X-MSC-Api-Version', '1.0']],
          body: [...new TextEncoder().encode(
            JSON.stringify({
              stagedUploadId: 'upload-1',
              uploadPath: '/v1/staged-uploads/upload-1',
              maxBytes: totalBytes,
              expiresAt: '',
            }),
          )],
        };
      }
      if (request.path.endsWith('complete=false')) {
        return { status: 204, headers: [], body: [] };
      }
      return {
        status: 200,
        headers: [['X-MSC-Api-Version', '1.0']],
        body: [...new TextEncoder().encode(
          JSON.stringify({
            stagedUploadId: 'upload-1',
            receivedBytes: totalBytes,
            sha256: 'digest',
          }),
        )],
      };
    });
    const session = new DesktopSessionAuth(native);
    const client = new ApiClient({
      baseUrl: 'https://xubuntu.example',
      hostId: 'agent-xubuntu',
      fetchImpl: session.fetchForHost('agent-xubuntu'),
    });
    const source = {
      name: 'All the Mods 10.zip',
      size: totalBytes,
      readChunk: async (_offset: number, length: number) => new Uint8Array(length),
      close: async () => undefined,
    };

    await expect(
      client.stagedUploadFromFile({ purpose: 'modpack-archive' }, source),
    ).resolves.toMatchObject({ stagedUploadId: 'upload-1', receivedBytes: totalBytes });
    expect(native.authorizedRequest).toHaveBeenCalledTimes(3);
    expect(vi.mocked(native.authorizedRequest).mock.calls[1]?.[0].path).toContain(
      'complete=false',
    );
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
