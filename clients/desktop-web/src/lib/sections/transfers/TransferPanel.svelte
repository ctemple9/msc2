<script lang="ts">
  import ActionButton from '../../components/ActionButton.svelte';
  import { getPlatform, type PickedFile } from '../../platform';
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

  let file: PickedFile | undefined;
  let progress = 0;
  let message = '';

  async function stage(): Promise<void> {
    if (!file || !api?.upload) {
      message = 'Choose a file while connected to an agent.';
      return;
    }
    try {
      progress = 20;
      const result = await api.upload(purpose, file.bytes);
      progress = 100;
      message = `${file.name} staged (${bytesLabel(result.receivedBytes)}).`;
      onComplete?.(result.stagedUploadId);
    } catch (error) {
      progress = 0;
      message = errorMessage(error);
    }
  }

  async function chooseFile(): Promise<void> {
    file = (await (await getPlatform()).pickFile({ label })) ?? undefined;
  }
</script>

<div class="inline-form transfer-panel">
  <div class="field">
    <label>{label}</label>
    <ActionButton label="Choose file" onclick={() => void chooseFile()}>Choose file</ActionButton>
  </div>
  <ActionButton label="Stage file" onclick={stage}>Stage</ActionButton>
  {#if progress}<div class="progress-bar" aria-label="Upload progress">
      <span style={`width: ${progress}%`}></span>
    </div>{/if}
  {#if message}<small class="field-help" role="status">{message}</small>{/if}
</div>
