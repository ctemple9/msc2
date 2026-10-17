import { expect, it } from 'vitest';
import { ApiClient } from '../../src/lib/api/client';
import { createNativeFileUploader } from '../../src/lib/auth/native-transfer';
import type {
  NativeFileSource,
  NativeUploadProgress,
  NativeUploadResult,
} from '../../src/lib/platform/types';

function source(host = 'alpha'): NativeFileSource {
  return {
    kind: 'native',
    sourceId: 'selection-1',
    binding: { agentHostId: host, purpose: 'world-import', targetServerId: 'server-a' },
    name: 'world.zip',
    size: 8 * 1024 ** 3,
    expiresInSeconds: 300,
    close: async () => {},
  };
}
function result(): NativeUploadResult {
  return {
    operationId: 'selection-1',
    agentHostId: 'alpha',
    stagedUploadId: 'stage-1',
    receivedBytes: 8 * 1024 ** 3,
    sha256: 'a'.repeat(64),
    transport: 'stream',
  };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

// Large input is metadata only: any accidental use of the generic byte bridge fails.
it('sends a native selection through its original host without a JavaScript byte request', async () => {
  let calls = 0;
  const client = new ApiClient({
    baseUrl: 'http://alpha.test',
    hostId: 'alpha',
    fetchImpl: async () => {
      throw new Error('Unexpected JavaScript byte request');
    },
    nativeFileUploader: {
      upload: async (file) => {
        calls++;
        expect(file.binding.agentHostId).toBe('alpha');
        return result();
      },
    },
  });
  await expect(client.stagedUploadNative(source('beta'))).rejects.toThrow();
  expect(calls).toBe(0);
  await expect(client.stagedUploadNative(source())).resolves.toMatchObject({
    receivedBytes: 8 * 1024 ** 3,
  });
  expect(calls).toBe(1);
});

it('filters unrelated progress and awaits the actual cancellation cleanup failure', async () => {
  const began = deferred<void>();
  const cleanup = deferred<NativeUploadResult>();
  const controller = new AbortController();
  const events: NativeUploadProgress[] = [];
  const hosts: string[] = [];
  const uploader = createNativeFileUploader({
    start: async (host, id, progress) => {
      hosts.push(host);
      progress({
        operationId: id,
        agentHostId: 'beta',
        phase: 'transferring',
        bytesTransferred: 1,
        totalBytes: 2,
        bytesPerSecond: 1,
        transport: 'stream',
      });
      progress({
        operationId: 'another-operation',
        agentHostId: host,
        phase: 'complete',
        bytesTransferred: 2,
        totalBytes: 2,
        bytesPerSecond: 1,
        transport: 'stream',
      });
      return { operationId: id, agentHostId: host };
    },
    wait: (host) => {
      hosts.push(host);
      began.resolve();
      return cleanup.promise;
    },
    cancel: (host) => {
      hosts.push(host);
      return cleanup.promise;
    },
  });
  let settled = false;
  const pending = uploader.upload(source(), {
    signal: controller.signal,
    onProgress: (progress) => events.push(progress),
  });
  void pending.then(
    () => {
      settled = true;
    },
    () => {
      settled = true;
    },
  );
  await began.promise;
  controller.abort();
  await Promise.resolve();
  expect(settled).toBe(false);
  expect(events).toEqual([]);
  // A deterministic rejected cleanup, with no timer or live network involved.
  const failure = {
    code: 'cancelled',
    message: 'Partial cleanup could not be confirmed',
    cleanup: 'unconfirmed' as const,
  };
  cleanup.reject(failure);
  await expect(pending).rejects.toMatchObject({ cleanup: 'unconfirmed' });
  expect(hosts.every((host) => host === 'alpha')).toBe(true);
});

it('recovers a lost start reply by cancelling the known source identity and preserves late completion', async () => {
  let cancelled = '';
  const uploader = createNativeFileUploader({
    start: async () => {
      throw new Error('IPC reply lost');
    },
    wait: async () => {
      throw new Error('Unexpected wait');
    },
    cancel: async (host, id) => {
      expect(host).toBe('alpha');
      cancelled = id;
      return result();
    },
  });
  await expect(uploader.upload(source())).resolves.toMatchObject({ stagedUploadId: 'stage-1' });
  expect(cancelled).toBe('selection-1');
});

// Essential: a completed upload must not make a later import or late response
// act on the newly selected host/server. Existing bridge tests stop at receipt.
it('keeps consumer requests on their captured context and rejects late results', async () => {
  const { guardUploadApi } = await import('../../src/lib/sections/shared/upload-source');
  const initial = 'alpha/server-a';
  let current = initial;
  const response = deferred<{ operationId: string }>();
  const started = deferred<void>();
  const requests: string[] = [];
  const scoped = guardUploadApi(
    {
      hostId: () => 'alpha',
      get: async <T>() => ({}) as T,
      post: async <T>(path: string) => {
        requests.push(path);
        started.resolve();
        return (await response.promise) as T;
      },
      delete: async (path: string) => {
        requests.push(path);
      },
    },
    () => current === initial,
  );
  let applied = false;
  const pending = scoped.post('/v1/servers/import').then(() => {
    applied = true;
  });
  const rejected = expect(pending).rejects.toThrow();
  await started.promise;
  current = 'beta/server-b';
  response.resolve({ operationId: 'operation-alpha' });
  await rejected;
  expect(applied).toBe(false);
  await expect(scoped.post('/v1/worlds/replace-active')).rejects.toThrow();
  await scoped.delete!('/v1/staged-uploads/input-alpha');
  expect(requests).toEqual(['/v1/servers/import', '/v1/staged-uploads/input-alpha']);
});

// Essential: cleanup failure must remain a failure, while an accepted importer
// keeps ownership. Controlled promises avoid sleeps and live agents.
it('awaits actual staging cleanup and distinguishes importer ownership from failure', async () => {
  const { discardUpload } = await import('../../src/lib/sections/shared/upload-source');
  const deletion = deferred<void>();
  const started = deferred<void>();
  let settled = false;
  const api = {
    get: async <T>() => {
      throw new Error('Unknown input');
    },
    post: async <T>() => ({}) as T,
    delete: () => {
      started.resolve();
      return deletion.promise;
    },
  };
  const pending = discardUpload(api, 'input-alpha').finally(() => {
    settled = true;
  });
  const failed = expect(pending).rejects.toThrow('disk cleanup failed');
  await started.promise;
  expect(settled).toBe(false);
  deletion.reject(new Error('disk cleanup failed'));
  await failed;
  await expect(
    discardUpload(
      {
        ...api,
        delete: async () => {
          throw new Error('Import owns the file');
        },
        cleanupStatus: async <T>() => ({ state: 'redeemed' }) as T,
      },
      'accepted-input',
    ),
  ).resolves.toBeUndefined();
});

// Essential: closing a wizard while the OS picker is open must release its
// eventual selection instead of retaining an unused native handle until expiry.
it('releases a late picker selection after its owning screen closes', async () => {
  const { pickUploadSource } = await import('../../src/lib/sections/shared/upload-source');
  const selection = deferred<NativeFileSource>();
  const opened = deferred<void>();
  let active = true;
  let closed = 0;
  const pending = pickUploadSource(
    {
      get: async <T>() => ({}) as T,
      post: async <T>() => ({}) as T,
      selectUploadSource: () => {
        opened.resolve();
        return selection.promise;
      },
    },
    'world-import',
    { label: 'World archive' },
    {},
    () => active,
  );
  await opened.promise;
  active = false;
  selection.resolve({
    ...source(),
    close: async () => {
      closed++;
    },
  });
  await expect(pending).resolves.toBeNull();
  expect(closed).toBe(1);
});
