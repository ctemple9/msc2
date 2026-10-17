import type {
  NativeFileSource,
  NativeFileUploader,
  NativeUploadFailure,
  NativeUploadOptions,
  NativeUploadProgress,
  NativeUploadResult,
} from '../platform/types';

export class NativeUploadError extends Error {
  readonly code: string;
  readonly cleanup: NativeUploadFailure['cleanup'];
  readonly agentHostId?: string;
  readonly operationId?: string;
  readonly stagedUploadId?: string;

  constructor(failure: NativeUploadFailure) {
    super(failure.message);
    this.name = 'NativeUploadError';
    this.code = failure.code;
    this.cleanup = failure.cleanup;
    this.agentHostId = failure.agentHostId;
    this.operationId = failure.operationId;
    this.stagedUploadId = failure.stagedUploadId;
  }
}

export interface NativeTransferBridge {
  start(
    agentHostId: string,
    sourceId: string,
    onProgress: (progress: NativeUploadProgress) => void,
  ): Promise<{ operationId: string; agentHostId: string }>;
  wait(agentHostId: string, operationId: string): Promise<NativeUploadResult>;
  cancel(agentHostId: string, operationId: string): Promise<NativeUploadResult>;
}

function nativeFailure(error: unknown): error is NativeUploadFailure {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'cleanup' in error &&
    'message' in error
  );
}

/** Every callback and cancellation retains the initiating host, even after navigation. */
export function createNativeFileUploader(bridge: NativeTransferBridge): NativeFileUploader {
  return {
    async upload(source: NativeFileSource, options: NativeUploadOptions = {}) {
      const agentHostId = source.binding.agentHostId;
      // Rust uses this single-use source identity as the operation identity too.
      // Cancellation can therefore retire a selection even if start's IPC reply is lost.
      const operationId = source.sourceId;
      const size = source.size;
      if (options.signal?.aborted) {
        await source.close();
        throw new NativeUploadError({
          code: 'cancelled',
          message: 'The transfer was cancelled.',
          cleanup: 'not-admitted',
        });
      }
      let started = false;
      let abortRequested = false;
      type CancellationOutcome = { result?: NativeUploadResult; error?: unknown };
      let cancellation: Promise<CancellationOutcome> | undefined;
      const cancel = (): Promise<CancellationOutcome> => {
        cancellation ??= bridge.cancel(agentHostId, operationId).then(
          (result): CancellationOutcome => ({ result }),
          (error: unknown): CancellationOutcome => ({ error }),
        );
        return cancellation;
      };
      const abort = () => {
        abortRequested = true;
        void cancel();
      };
      const finish = (result: NativeUploadResult) => {
        if (
          result.agentHostId !== agentHostId ||
          result.operationId !== operationId ||
          result.receivedBytes !== size
        ) {
          throw new Error('The native upload returned an unrelated or incomplete result.');
        }
        options.onProgress?.({
          operationId,
          agentHostId,
          phase: 'complete',
          bytesTransferred: result.receivedBytes,
          totalBytes: size,
          bytesPerSecond: 0,
          transport: result.transport,
        });
        return result;
      };
      options.signal?.addEventListener('abort', abort, { once: true });
      try {
        const operation = await bridge.start(agentHostId, source.sourceId, (progress) => {
          if (progress.agentHostId === agentHostId && progress.operationId === operationId)
            options.onProgress?.(progress);
        });
        started = true;
        if (operation.agentHostId !== agentHostId || operation.operationId !== operationId) {
          await cancel();
          throw new Error('The native operation returned an unrelated identity.');
        }
        if (abortRequested || options.signal?.aborted) abort();
        // A completed operation wins late cancellation; do not claim rollback.
        return finish(await bridge.wait(agentHostId, operationId));
      } catch (error) {
        if (!started && (abortRequested || !nativeFailure(error))) {
          const recovery = await cancel();
          if (recovery.result) return finish(recovery.result);
          if (nativeFailure(recovery.error)) throw new NativeUploadError(recovery.error);
          throw new NativeUploadError({
            code: 'start_unconfirmed',
            message: 'The desktop could not confirm startup or cleanup of this native operation.',
            cleanup: 'unconfirmed',
          });
        }
        if (nativeFailure(error)) throw new NativeUploadError(error);
        throw error;
      } finally {
        options.signal?.removeEventListener('abort', abort);
        await cancellation;
        // This releases only an unused local handle; the native worker owns
        // all agent staging cleanup and reports its outcome through wait().
        await source.close().catch(() => undefined);
      }
    },
  };
}

export async function loadTauriNativeFileUploader(): Promise<NativeFileUploader> {
  const { invoke, Channel } = await import('@tauri-apps/api/core');
  return createNativeFileUploader({
    start: (agentHostId, sourceId, onProgress) => {
      const progress = new Channel<NativeUploadProgress>();
      progress.onmessage = onProgress;
      return invoke('start_native_upload', { request: { agentHostId, sourceId }, progress });
    },
    wait: (agentHostId, operationId) => invoke('wait_native_upload', { agentHostId, operationId }),
    cancel: (agentHostId, operationId) =>
      invoke('cancel_native_upload', { agentHostId, operationId }),
  });
}
