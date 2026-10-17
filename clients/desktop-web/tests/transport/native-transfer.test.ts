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
