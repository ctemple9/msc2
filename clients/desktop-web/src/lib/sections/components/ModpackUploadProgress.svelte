<script lang="ts">
  import Button from '../../components/base/Button.svelte';
  import type { FileUploadProgress } from '../../platform/types';
  import { bytesLabel } from '../shared/types';

  export let fileName: string;
  export let progress: FileUploadProgress;
  export let onCancel: (() => void) | undefined = undefined;

  $: percentage =
    progress.totalBytes > 0 ? Math.floor((progress.bytesUploaded / progress.totalBytes) * 100) : 0;
  $: status =
    progress.phase === 'selecting'
      ? 'Choose the archive in the file picker…'
      : progress.phase === 'preparing'
        ? 'Preparing the upload…'
        : progress.phase === 'reading'
          ? 'Reading the next part of the archive…'
          : progress.phase === 'uploading'
            ? 'Sending archive data to the host…'
            : progress.phase === 'cancelling'
              ? 'Stopping the upload and removing its partial file from the host…'
              : 'File received. Finishing the host-side check…';
</script>

<div class="upload-progress" role="status" aria-live="polite">
  <p class="filename">{fileName}</p>
  <p class="status">{status}</p>
  {#if progress.phase === 'selecting'}
    <progress aria-label="File upload progress" max="1"></progress>
  {:else}
    <progress
      aria-label="File upload progress"
      max={Math.max(progress.totalBytes, 1)}
      value={progress.bytesUploaded}
    ></progress>
    <div class="counts">
      <span>{bytesLabel(progress.bytesUploaded)} of {bytesLabel(progress.totalBytes)} sent</span>
      <span>{percentage}%</span>
    </div>
  {/if}
  {#if onCancel && progress.phase !== 'complete' && progress.phase !== 'cancelling'}
    <div class="actions">
      <Button variant="secondary" onclick={onCancel}>Cancel upload</Button>
    </div>
  {/if}
</div>

<style>
  .upload-progress {
    display: flex;
    flex-direction: column;
    gap: 9px;
    min-width: 280px;
  }

  p {
    margin: 0;
  }

  .filename {
    overflow-wrap: anywhere;
    color: var(--msc2-text-primary);
    font-size: 13px;
    font-weight: 500;
  }

  .status,
  .counts {
    color: var(--msc2-text-tertiary);
    font-size: 12px;
  }

  progress {
    display: block;
    width: 100%;
    height: 8px;
    accent-color: var(--msc2-accent);
  }

  .counts {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    padding-top: 4px;
  }
</style>
