<script lang="ts">
  import ActionButton from '../../components/ActionButton.svelte';
  import type { ScreenApi } from '../shared/types';
  import { bytesLabel, errorMessage } from '../shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let purpose:
    | 'world-import'
    | 'active-world-replace'
    | 'modpack-archive'
    | 'addon-local-file'
    | 'curseforge-manual-file' = 'world-import';
  export let label = 'Stage a file';
  export let onComplete: ((id: string) => void) | undefined = undefined;

  let file: File | undefined;
  let progress = 0;
  let message = '';

  async function stage(): Promise<void> {
    if (!file || !api?.upload) {
      message = 'Choose a file while connected to an agent.';
      return;
    }
    try {
      progress = 20;
      const result = await api.upload(purpose, new Uint8Array(await file.arrayBuffer()));
      progress = 100;
      message = `${file.name} staged (${bytesLabel(result.receivedBytes)}).`;
      onComplete?.(result.stagedUploadId);
    } catch (error) {
      progress = 0;
      message = errorMessage(error);
    }
  }
</script>

<div class="inline-form transfer-panel">
  <div class="field">
    <label for={`file-${purpose}`}>{label}</label><input
      id={`file-${purpose}`}
      type="file"
      onchange={(event) => (file = (event.currentTarget as HTMLInputElement).files?.[0])}
    />
  </div>
  <ActionButton label="Stage file" onclick={stage}>Stage</ActionButton>
  {#if progress}<div class="progress-bar" aria-label="Upload progress">
      <span style={`width: ${progress}%`}></span>
    </div>{/if}
  {#if message}<small class="field-help" role="status">{message}</small>{/if}
</div>
