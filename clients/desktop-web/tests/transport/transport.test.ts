import { describe, expect, it } from 'vitest';
import {
  ApiClient,
  ApiError,
  UploadCancelledError,
  bearerCredentialAdapter,
} from '../../src/lib/api';
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

  it('streams a selected file using its chosen chunk size and verifies completion', async () => {
    const chunkSize = 4 * 1024 * 1024;
    const totalBytes = chunkSize + 3;
    const requests: { url: string; init?: RequestInit }[] = [];
    const progress: { phase: string; bytesUploaded: number; totalBytes: number }[] = [];
    const source = {
      name: 'mods.zip',
      size: totalBytes,
      readChunk: async (offset: number, maxBytes: number) =>
        new Uint8Array(Math.min(maxBytes, totalBytes - offset)).fill(offset === 0 ? 1 : 2),
      close: async () => undefined,
    };
    const client = new ApiClient({
      baseUrl: 'http://alpha.test',
      hostId: 'alpha',
      fetchImpl: async (url, init) => {
        requests.push({ url: String(url), init });
        if (requests.length === 1) {
          return response({
            stagedUploadId: 'upload-1',
            uploadPath: '/v1/staged-uploads/upload-1',
            maxBytes: totalBytes,
            maxChunkBytes: 8 * 1024 * 1024,
            expiresAt: '',
          });
        }
        if (requests.length < 3) return new Response(null, { status: 204 });
        return response({
          stagedUploadId: 'upload-1',
          receivedBytes: totalBytes,
          sha256: 'digest',
        });
      },
    });

    await expect(
      client.stagedUploadFromFile({ purpose: 'modpack-archive' }, source, {
        chunkSizeBytes: chunkSize,
        onProgress: (update) => progress.push(update),
      }),
    ).resolves.toMatchObject({ stagedUploadId: 'upload-1', receivedBytes: totalBytes });

    expect(requests.map(({ url }) => url)).toEqual([
      'http://alpha.test/v1/staged-uploads',
      'http://alpha.test/v1/staged-uploads/upload-1/chunks?offset=0&complete=false',
      `http://alpha.test/v1/staged-uploads/upload-1/chunks?offset=${chunkSize}&complete=true`,
    ]);
    const uploadedBodies = requests.slice(1).map(({ init }) => init?.body as Uint8Array);
    expect(uploadedBodies.map((body) => body.byteLength)).toEqual([chunkSize, 3]);
    expect(uploadedBodies.every((body) => body.byteLength <= chunkSize)).toBe(true);
    expect(progress).toEqual([
      { phase: 'preparing', bytesUploaded: 0, totalBytes },
      { phase: 'preparing', bytesUploaded: 0, totalBytes, chunkSizeBytes: chunkSize },
      { phase: 'reading', bytesUploaded: 0, totalBytes, chunkSizeBytes: chunkSize },
      { phase: 'uploading', bytesUploaded: 0, totalBytes, chunkSizeBytes: chunkSize },
      { phase: 'reading', bytesUploaded: chunkSize, totalBytes, chunkSizeBytes: chunkSize },
      { phase: 'uploading', bytesUploaded: chunkSize, totalBytes, chunkSizeBytes: chunkSize },
      { phase: 'complete', bytesUploaded: totalBytes, totalBytes, chunkSizeBytes: chunkSize },
    ]);
    expect(JSON.parse(String(requests[0].init?.body))).toMatchObject({
      purpose: 'modpack-archive',
      expectedBytes: totalBytes,
    });
  });

  it('rejects a short file chunk before sending it to the agent', async () => {
    const requests: { url: string; method: string }[] = [];
    const client = new ApiClient({
      baseUrl: 'http://alpha.test',
      hostId: 'alpha',
      fetchImpl: async (url, init) => {
        requests.push({ url: String(url), method: String(init?.method) });
        if (init?.method === 'DELETE') return new Response(null, { status: 204 });
        return response({
          stagedUploadId: 'upload-1',
          uploadPath: '/v1/staged-uploads/upload-1',
          maxBytes: 100,
          expiresAt: '',
        });
      },
    });

    await expect(
      client.stagedUploadFromFile(
        { purpose: 'modpack-archive' },
        {
          name: 'mods.zip',
          size: 4,
          readChunk: async () => new Uint8Array([1, 2]),
          close: async () => undefined,
        },
      ),
    ).rejects.toThrow('Could not read the complete file at byte 0.');
    expect(requests).toEqual([
      { url: 'http://alpha.test/v1/staged-uploads', method: 'POST' },
      { url: 'http://alpha.test/v1/staged-uploads/upload-1', method: 'DELETE' },
    ]);
  });

  it('cancels an active chunk request and removes the staged upload', async () => {
    const controller = new AbortController();
    let chunkStarted = () => {};
    const started = new Promise<void>((resolve) => (chunkStarted = resolve));
    const requests: string[] = [];
    const client = new ApiClient({
      baseUrl: 'http://alpha.test',
      hostId: 'alpha',
      fetchImpl: async (url, init) => {
        requests.push(`${String(init?.method)} ${String(url)}`);
        if (init?.method === 'POST') {
          return response({
            stagedUploadId: 'upload-1',
            uploadPath: '/v1/staged-uploads/upload-1',
            maxBytes: 2 * 1024 * 1024,
            maxChunkBytes: 8 * 1024 * 1024,
            expiresAt: '',
          });
        }
        if (init?.method === 'DELETE') return new Response(null, { status: 204 });
        chunkStarted();
        return new Promise<Response>((_resolve, reject) => {
          init?.signal?.addEventListener('abort', () => reject(new Error('aborted')), {
            once: true,
          });
        });
      },
    });
    const upload = client.stagedUploadFromFile(
      { purpose: 'modpack-archive' },
      {
        name: 'large-pack.zip',
        size: 2 * 1024 * 1024,
        readChunk: async (_offset, bytes) => new Uint8Array(bytes),
        close: async () => undefined,
      },
      { chunkSizeBytes: 1024 * 1024, signal: controller.signal },
    );

    await started;
    controller.abort();
    await expect(upload).rejects.toBeInstanceOf(UploadCancelledError);
    expect(requests).toContain('DELETE http://alpha.test/v1/staged-uploads/upload-1');
    expect(requests.filter((request) => request.includes('/chunks?'))).toHaveLength(1);
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
