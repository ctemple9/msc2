<script lang="ts">
  import Button from '../../components/base/Button.svelte';
  import type { FileUploadProgress } from '../../platform/types';
  import { bytesLabel } from '../shared/types';

  export let fileName: string;
  export let progress: FileUploadProgress;
  export let onCancel: (() => void) | undefined = undefined;

  $: knownTotal =
    Number.isSafeInteger(progress.totalBytes) &&
    progress.totalBytes > 0 &&
    Number.isFinite(progress.bytesUploaded) &&
    progress.bytesUploaded >= 0 &&
    progress.bytesUploaded <= progress.totalBytes;
  $: transferring = ['local-copying', 'transferring', 'uploading', 'reading'].includes(
    progress.phase,
  );
  $: percentage = knownTotal
    ? Math.min(100, Math.floor((progress.bytesUploaded / progress.totalBytes) * 100))
    : undefined;
  $: rate =
    transferring &&
    knownTotal &&
    Number.isFinite(progress.bytesPerSecond) &&
    (progress.bytesPerSecond ?? 0) > 0
      ? progress.bytesPerSecond
      : undefined;
  $: status =
    progress.statusLine ??
    {
      selecting: 'Choose the file…',
      preparing: 'Preparing the selected source…',
      reading: 'Reading the selected file…',
      uploading: 'Transferring file to the host…',
      transferring: 'Transferring file to the host…',
      'local-copying': 'Copying file to the host…',
      verifying: 'Verifying the transferred file…',
      inspecting: 'Inspecting the received file…',
      installing: 'Installing the received file…',
      importing: 'Importing the received file…',
      cancelling: 'Stopping the transfer and waiting for cleanup…',
      complete: 'File transfer verified.',
      cancelled: 'Transfer cancelled.',
      failed: 'Transfer stopped.',
    }[progress.phase];
</script>

<div class="upload-progress" role="status" aria-live="polite">
  <p class="filename">{fileName}</p>
  <p class="status">{status}</p>
  {#if transferring && knownTotal}
    <progress
      aria-label="File transfer bytes"
      max={progress.totalBytes}
      value={progress.bytesUploaded}
    ></progress>
    <div class="counts">
      <span>{bytesLabel(progress.bytesUploaded)} of {bytesLabel(progress.totalBytes)}</span><span
        >{percentage}%{rate ? ` · ${bytesLabel(rate)}/s` : ''}</span
      >
    </div>
    {#if percentage === 100}<p class="status">
        Transfer complete. Verification and import are separate stages.
      </p>{/if}
  {:else if !['complete', 'cancelled', 'failed'].includes(progress.phase)}
    <progress aria-label={status} max="1"></progress>
  {/if}
  {#if progress.phase === 'complete'}<p class="status">
      The file is ready for inspection or import. Transfer completion does not mean installation is
      finished.
    </p>{/if}
  {#if progress.transport === 'legacy-buffered-agent'}<p class="status">
      This older agent buffers incoming files. Update it for streamed receipt.
    </p>{/if}
  {#if onCancel && !['complete', 'cancelling', 'cancelled', 'failed'].includes(progress.phase)}
    <div class="actions"><Button variant="secondary" onclick={onCancel}>Cancel</Button></div>
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
