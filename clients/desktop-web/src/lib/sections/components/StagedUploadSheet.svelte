<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Button from '../../components/base/Button.svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import { UploadCancelledError } from '../../api/client';
  import { NativeUploadError } from '../../auth/native-transfer';
  import type { FileUploadProgress } from '../../platform/types';
  import {
    MAX_UPLOAD_CHUNK_MIB,
    MIN_UPLOAD_CHUNK_MIB,
    preferredUploadChunkMiB,
    savePreferredUploadChunkMiB,
  } from '../../platform/upload-settings';
  import type { Schema, ScreenApi } from '../shared/types';
  import { bytesLabel, errorMessage } from '../shared/types';
  import {
    discardUpload,
    guardUploadApi,
    isNativeSource,
    nativeProgress,
    sourceApi,
    type UploadSource,
  } from '../shared/upload-source';
  import { reportTransferFailure } from '../shared/transfer-notices';
  import ModpackUploadProgress from './ModpackUploadProgress.svelte';

  export let api: ScreenApi | undefined = undefined;
  export let purpose: Schema['StagedUploadBeginRequestDTO']['purpose'];
  export let source: UploadSource;
  export let completionStage: 'complete' | 'inspecting' | 'importing' | 'installing' = 'complete';
  export let onComplete: (
    upload: Schema['StagedUploadCompleteResultDTO'],
    api: ScreenApi,
    report: (statusLine: string) => void,
  ) => void | Promise<void>;
  export let onClose: () => void;

  const scoped = sourceApi(source, api);
  const native = isNativeSource(source);
  let chunkSizeMiB = preferredUploadChunkMiB();
  let progress: FileUploadProgress | undefined;
  let error: string | undefined;
  let controller: AbortController | undefined;
  let sourceClosed = false;
  let disposed = false;
  let effectiveChunkSizeBytes: number | undefined;
  let stagedId: string | undefined;
  let task: Promise<void> | undefined;
  let mode: 'settings' | 'uploading' | 'failed' = 'settings';
  let cleanupUnconfirmed = false;

  function setChunkSize(value: number): void {
    chunkSizeMiB = value;
    savePreferredUploadChunkMiB(value);
  }
  async function closeSource(): Promise<void> {
    if (sourceClosed) return;
    await source.close();
    sourceClosed = true;
  }
  async function closeBeforeUpload(): Promise<void> {
    try {
      await closeSource();
      if (!disposed) onClose();
    } catch (cause) {
      error = errorMessage(cause);
      mode = 'failed';
    }
  }
  function cancelUpload(): void {
    if (!controller || mode !== 'uploading') return;
    progress = {
      ...progress,
      phase: 'cancelling',
      bytesUploaded: progress?.bytesUploaded ?? 0,
      totalBytes: progress?.totalBytes ?? source.size,
    };
    controller.abort();
  }
  async function closeFailed(): Promise<void> {
    if (stagedId) {
      try {
        await discardUpload(scoped, stagedId);
        stagedId = undefined;
        cleanupUnconfirmed = false;
      } catch (cause) {
        error = `Agent cleanup could not be confirmed: ${errorMessage(cause)}`;
        return;
      }
    }
    if (!disposed) onClose();
  }
  async function runUpload(): Promise<void> {
    mode = 'uploading';
    error = undefined;
    controller = new AbortController();
    const signal = controller.signal;
    const current = () => !disposed && !signal.aborted && (scoped?.isCurrent?.() ?? true);
    try {
      if (!scoped) throw new Error('Connect to an agent before uploading.');
      if (!current())
        throw new Error('The selected host or server changed. Choose the file again.');
      progress = { phase: 'preparing', bytesUploaded: 0, totalBytes: source.size };
      let uploaded: Schema['StagedUploadCompleteResultDTO'];
      if (isNativeSource(source)) {
        if (!scoped.uploadNative)
          throw new Error('Native uploads are unavailable in this desktop build.');
        uploaded = {
          ...(await scoped.uploadNative(source, {
            signal,
            onProgress: (next) => {
              if (!disposed) progress = nativeProgress(next);
            },
          })),
        };
      } else {
        if (!scoped.uploadFile) throw new Error('File uploads are unavailable.');
        uploaded = await scoped.uploadFile(purpose, source, {
          fileName: source.name,
          operationId: source.uploadBindings?.operationId ?? undefined,
          fileId: source.uploadBindings?.fileId ?? undefined,
          chunkSizeBytes: chunkSizeMiB * 1024 * 1024,
          signal,
          onProgress: (next) => {
            effectiveChunkSizeBytes = next.chunkSizeBytes ?? effectiveChunkSizeBytes;
            if (!disposed) progress = next;
          },
        });
      }
      stagedId = uploaded.stagedUploadId;
      if (!current()) throw new Error('The transfer stopped before import.');
      progress = {
        phase: completionStage,
        bytesUploaded: uploaded.receivedBytes,
        totalBytes: uploaded.receivedBytes,
      };
      await onComplete(uploaded, guardUploadApi(scoped, current), (statusLine) => {
        if (!disposed) progress = { ...progress!, statusLine };
      });
      if (!current()) throw new Error('The selected host or server changed before completion.');
      stagedId = undefined; // The caller now owns the verified input or its import operation.
      await closeSource();
      if (!disposed) onClose();
    } catch (cause) {
      const messages = [errorMessage(cause)];
      cleanupUnconfirmed = cause instanceof NativeUploadError && cause.cleanup === 'unconfirmed';
      if (stagedId) {
        try {
          await discardUpload(scoped, stagedId);
          stagedId = undefined;
        } catch (cleanup) {
          cleanupUnconfirmed = true;
          messages.push(`Agent cleanup could not be confirmed: ${errorMessage(cleanup)}`);
        }
      }
      try {
        await closeSource();
      } catch (cleanup) {
        cleanupUnconfirmed = true;
        messages.push(`Could not release the selected file: ${errorMessage(cleanup)}`);
      }
      const cancelled =
        cause instanceof UploadCancelledError ||
        (cause instanceof NativeUploadError && cause.code === 'cancelled') ||
        (signal.aborted && completionStage === 'inspecting');
      if (cancelled && !cleanupUnconfirmed) {
        if (!disposed) onClose();
        return;
      }
      if (
        cause instanceof NativeUploadError &&
        cause.cleanup === 'unconfirmed' &&
        cause.stagedUploadId
      )
        stagedId ??= cause.stagedUploadId;
      error = messages.join(' ');
      if (disposed && cleanupUnconfirmed) reportTransferFailure(scoped, error);
      mode = 'failed';
    } finally {
      controller = undefined;
    }
  }
  function startUpload(): void {
    if (mode === 'settings' && !task) task = runUpload();
  }
  onMount(() => {
    if (native && !source.olderAgent) startUpload();
  });
  onDestroy(() => {
    disposed = true;
    controller?.abort();
    if (!task) void closeSource().catch((error) => reportTransferFailure(scoped, error));
  });
</script>

<Sheet
  title={mode === 'settings'
    ? 'Prepare transfer'
    : mode === 'failed'
      ? 'Transfer stopped'
      : 'Transfer file'}
  size="sm"
  onClose={mode === 'settings'
    ? closeBeforeUpload
    : mode === 'uploading'
      ? progress?.phase === 'installing' ||
        progress?.phase === 'importing' ||
        progress?.phase === 'complete'
        ? undefined
        : cancelUpload
      : closeFailed}
>
  {#if mode === 'settings'}
    <div class="body">
      <p class="file">{source.name} · {bytesLabel(source.size)}</p>
      {#if native}
        <p class="explain">
          This host uses older transfer support. MSC will use its supported native fallback. Update
          the agent for streamed receipt; some older file routes buffer input in host memory.
        </p>
      {:else}
        <p class="explain">
          This older file adapter sends one chunk at a time. Chunk size changes temporary memory and
          request overhead.
        </p>
        <label for="upload-chunk-size">Chunk size: {chunkSizeMiB} MiB</label>
        <input
          id="upload-chunk-size"
          type="range"
          min={MIN_UPLOAD_CHUNK_MIB}
          max={MAX_UPLOAD_CHUNK_MIB}
          step="1"
          value={chunkSizeMiB}
          oninput={(event) => setChunkSize(Number(event.currentTarget.value))}
        />
        <div class="range-hints"><span>Lower memory use</span><span>Fewer requests</span></div>
      {/if}
      <div class="footer">
        <Button variant="secondary" onclick={closeBeforeUpload}>Cancel</Button><Button
          variant="primary"
          onclick={startUpload}>Start transfer</Button
        >
      </div>
    </div>
  {:else if mode === 'uploading' && progress}
    <ModpackUploadProgress
      fileName={source.name}
      {progress}
      onCancel={['cancelling', 'installing', 'importing', 'complete'].includes(progress.phase)
        ? undefined
        : cancelUpload}
    />
    {#if !native && effectiveChunkSizeBytes && effectiveChunkSizeBytes < chunkSizeMiB * 1024 * 1024}
      <p class="explain host-limit">
        This host limits chunks to {bytesLabel(effectiveChunkSizeBytes)}.
      </p>
    {/if}
  {:else}
    <div class="body">
      <p class="explain" role="alert">{error}</p>
      {#if cleanupUnconfirmed}<p class="explain">
          Cleanup is unconfirmed. The original source is preserved; the host may still hold staging
          data.
        </p>{/if}
      <div class="footer">
        <Button variant="secondary" onclick={closeFailed}
          >{stagedId ? 'Retry cleanup' : 'Close'}</Button
        >
      </div>
    </div>
  {/if}
</Sheet>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .file {
    margin: 0;
    color: var(--msc2-text-primary);
    font-size: 13px;
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .explain,
  label,
  .range-hints {
    margin: 0;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.5;
  }

  label {
    color: var(--msc2-text-secondary);
  }

  input[type='range'] {
    width: 100%;
    accent-color: var(--msc2-accent);
  }

  .range-hints,
  .footer {
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }

  .footer {
    justify-content: flex-end;
    padding-top: 6px;
  }

  .host-limit {
    margin-top: 10px;
  }
</style>
