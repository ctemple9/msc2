<script lang="ts">
  import { onDestroy } from 'svelte';
  import Button from '../../components/base/Button.svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import { UploadCancelledError } from '../../api/client';
  import type { FileChunkSource, FileUploadProgress } from '../../platform/types';
  import {
    MAX_UPLOAD_CHUNK_MIB,
    MIN_UPLOAD_CHUNK_MIB,
    preferredUploadChunkMiB,
    savePreferredUploadChunkMiB,
  } from '../../platform/upload-settings';
  import type { Schema, ScreenApi } from '../shared/types';
  import { bytesLabel, errorMessage } from '../shared/types';
  import ModpackUploadProgress from './ModpackUploadProgress.svelte';

  export let api: ScreenApi | undefined = undefined;
  export let purpose: Schema['StagedUploadBeginRequestDTO']['purpose'];
  export let source: FileChunkSource;
  export let onComplete: (upload: Schema['StagedUploadCompleteResultDTO']) => void | Promise<void>;
  export let onClose: () => void;

  let chunkSizeMiB = preferredUploadChunkMiB();
  let progress: FileUploadProgress | undefined;
  let error: string | undefined;
  let controller: AbortController | undefined;
  let sourceClosed = false;
  let effectiveChunkSizeBytes: number | undefined;
  let mode: 'settings' | 'uploading' | 'failed' = 'settings';

  function setChunkSize(value: number): void {
    chunkSizeMiB = value;
    savePreferredUploadChunkMiB(value);
  }

  async function closeSource(): Promise<void> {
    if (sourceClosed) return;
    sourceClosed = true;
    try {
      await source.close();
    } catch {
      // Closing a file handle must not hide the transfer result.
    }
  }

  function closeBeforeUpload(): void {
    void closeSource();
    onClose();
  }

  function cancelUpload(): void {
    if (!controller || mode !== 'uploading') return;
    progress = {
      phase: 'cancelling',
      bytesUploaded: progress?.bytesUploaded ?? 0,
      totalBytes: source.size,
    };
    controller.abort();
  }

  async function startUpload(): Promise<void> {
    if (!api?.uploadFile || mode !== 'settings') return;
    mode = 'uploading';
    error = undefined;
    controller = new AbortController();
    try {
      const uploaded = await api.uploadFile(purpose, source, {
        chunkSizeBytes: chunkSizeMiB * 1024 * 1024,
        signal: controller.signal,
        onProgress: (next) => {
          effectiveChunkSizeBytes = next.chunkSizeBytes ?? effectiveChunkSizeBytes;
          progress = next;
        },
      });
      progress = { phase: 'complete', bytesUploaded: source.size, totalBytes: source.size };
      await onComplete(uploaded);
      await closeSource();
      onClose();
    } catch (cause) {
      await closeSource();
      if (cause instanceof UploadCancelledError) {
        onClose();
        return;
      }
      error = errorMessage(cause);
      mode = 'failed';
    } finally {
      controller = undefined;
    }
  }

  onDestroy(() => {
    if (controller) {
      controller.abort();
    } else {
      void closeSource().catch(() => undefined);
    }
  });
</script>

<Sheet
  title={mode === 'settings'
    ? 'Prepare upload'
    : mode === 'failed'
      ? 'Upload failed'
      : 'Uploading file'}
  size="sm"
  onClose={mode === 'settings' ? closeBeforeUpload : undefined}
>
  {#if mode === 'settings'}
    <div class="body">
      <p class="file">{source.name} · {bytesLabel(source.size)}</p>
      <p class="explain">
        Choose a chunk size. Larger chunks can reduce request overhead, but use more temporary
        memory and may make the app less responsive. They do not guarantee a faster connection.
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
      <div class="range-hints">
        <span>Lower memory use</span>
        <span>Fewer requests</span>
      </div>
      <div class="footer">
        <Button variant="secondary" onclick={closeBeforeUpload}>Cancel</Button>
        <Button variant="primary" onclick={() => void startUpload()}>Start upload</Button>
      </div>
    </div>
  {:else if mode === 'uploading' && progress}
    <ModpackUploadProgress
      fileName={source.name}
      {progress}
      onCancel={progress.phase === 'complete' || progress.phase === 'cancelling'
        ? undefined
        : cancelUpload}
    />
    {#if effectiveChunkSizeBytes && effectiveChunkSizeBytes < chunkSizeMiB * 1024 * 1024}
      <p class="explain host-limit">
        This host supports {Math.floor(effectiveChunkSizeBytes / (1024 * 1024))} MiB chunks, so MSC is
        using that size for this upload.
      </p>
    {/if}
  {:else}
    <div class="body">
      <p class="explain" role="alert">{error}</p>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Close</Button>
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
