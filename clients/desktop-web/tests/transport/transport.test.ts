import { describe, expect, it } from 'vitest';
import { ApiClient, ApiError, bearerCredentialAdapter } from '../../src/lib/api';
import { ReconnectingStream } from '../../src/lib/streams';
import { OperationTracker } from '../../src/lib/operations';

function response(body: unknown, status = 200, version = '1.0'): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json', 'X-MSC-Api-Version': version },
  });
}

describe('shared host-aware transport', () => {
  it('builds resource URLs against the selected host', () => {
    const client = new ApiClient({ baseUrl: 'http://alpha.test/', hostId: 'alpha' });

    expect(client.resourceUrl('/v1/worlds/slot-1/thumbnail')).toBe(
      'http://alpha.test/v1/worlds/slot-1/thumbnail',
    );
  });

  it('fetches binary resources through the selected host transport', async () => {
    let requestedUrl = '';
    const client = new ApiClient({
      baseUrl: 'http://alpha.test',
      hostId: 'alpha',
      fetchImpl: async (url) => {
        requestedUrl = url;
        return new Response(new Uint8Array([1, 2, 3]), {
          headers: { 'X-MSC-Api-Version': '1.0' },
        });
      },
    });

    await expect(client.requestBytes('GET', '/v1/worlds/slot-1/thumbnail')).resolves.toEqual(
      new Uint8Array([1, 2, 3]),
    );
    expect(requestedUrl).toBe('http://alpha.test/v1/worlds/slot-1/thumbnail');
  });

  it('adds version and bearer headers and decodes ErrorDTO failures', async () => {
    let request: RequestInit | undefined;
    const client = new ApiClient({
      baseUrl: 'http://alpha.test',
      hostId: 'alpha',
      credentialAdapter: bearerCredentialAdapter(async (hostId) => `secret-${hostId}`),
      fetchImpl: async (_url, init) => {
        request = init;
        return response({ code: 'forbidden', message: 'No access', helpId: null }, 403);
      },
    });

    await expect(client.getCapabilities()).rejects.toBeInstanceOf(ApiError);
    expect(request?.headers).toMatchObject({
      Authorization: 'Bearer secret-alpha',
      'X-MSC-Client-Api-Version': '1.0',
    });
  });

  it('rejects a staged upload before sending bytes over the configured ceiling', async () => {
    let calls = 0;
    const client = new ApiClient({
      baseUrl: 'http://alpha.test',
      hostId: 'alpha',
      fetchImpl: async () => {
        calls += 1;
        return response({
          stagedUploadId: 'u',
          uploadPath: '/v1/staged-uploads/u',
          maxBytes: 2,
          expiresAt: '',
        });
      },
    });

    await expect(
      client.uploadBytes('/v1/staged-uploads/u', new Uint8Array([1, 2, 3]), 2),
    ).rejects.toThrow('exceeds 2 bytes');
    expect(calls).toBe(0);
  });

  it('stops a staged download at the configured client memory ceiling', async () => {
    const client = new ApiClient({
      baseUrl: 'http://alpha.test',
      hostId: 'alpha',
      fetchImpl: async () =>
        new Response(new Uint8Array([1, 2, 3]), {
          headers: { 'X-MSC-Api-Version': '1.0' },
        }),
    });

    await expect(client.downloadBytes('download', 2)).rejects.toThrow('exceeds 2 bytes');
  });

  it('deduplicates bounded stream history and retains the latest entries after reconnect', () => {
    let close: (() => void) | undefined;
    const retries: (() => void)[] = [];
    const stream = new ReconnectingStream<{ id: string; text: string }>({
      maxHistory: 2,
      connector: {
        connect: (handlers) => {
          close = handlers.onClose;
          handlers.onOpen();
          return { close: () => undefined };
        },
      },
      dedupeKey: (value) => value.id,
      retryDelayMs: 0,
      schedule: (retry) => {
        retries.push(retry);
        return retries.length as unknown as ReturnType<typeof setTimeout>;
      },
    });
    stream.connect();
    stream.receive({ id: 'one', text: 'one' });
    stream.receive({ id: 'one', text: 'duplicate' });
    stream.receive({ id: 'two', text: 'two' });
    stream.receive({ id: 'three', text: 'three' });
    close?.();
    retries.shift()?.();

    expect(stream.historySnapshot.map((item) => item.id)).toEqual(['two', 'three']);
    expect(stream.state).toBe('live');
  });

  it('keeps terminal operations available for recovery and removes them only explicitly', () => {
    const tracker = new OperationTracker();
    tracker.upsert({
      id: 'op',
      type: 'install',
      state: 'succeeded',
      target: 'server',
      result: null,
    });
    expect(tracker.get('op')?.state).toBe('succeeded');
    tracker.beginRecovery();
    expect(tracker.state).toBe('recovering');
    tracker.completeRecovery();
    tracker.removeTerminal();
    expect(tracker.get('op')).toBeUndefined();
  });
});
