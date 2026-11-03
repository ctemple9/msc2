<script lang="ts">
  import { onDestroy } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { bytesLabel, errorMessage } from '../shared/types';
  import { pickUploadSource, type UploadSource } from '../shared/upload-source';
  import StagedUploadSheet from '../components/StagedUploadSheet.svelte';
  let disposed = false;
  onDestroy(() => {
    disposed = true;
  });
  export let api: ScreenApi | undefined = undefined;
  export let purpose:
    | 'world-import'
    | 'active-world-replace'
    | 'modpack-archive'
    | 'addon-local-file'
    | 'curseforge-manual-file' = 'world-import';
  export let label = 'Stage a file';
  export let onComplete: ((id: string, scoped?: ScreenApi) => void | Promise<void>) | undefined =
    undefined;
  let source: UploadSource | undefined;
  let message = '';
  async function chooseFile(): Promise<void> {
    try {
      source = (await pickUploadSource(api, purpose, { label }, {}, () => !disposed)) ?? undefined;
    } catch (error) {
      message = errorMessage(error);
    }
  }
  async function finish(upload: Schema['StagedUploadCompleteResultDTO']): Promise<void> {
    message = `${source?.name} verified (${bytesLabel(upload.receivedBytes)}). Inspection and import are separate.`;
    await onComplete?.(upload.stagedUploadId, source?.uploadApi);
  }
</script>

<div class="inline-form transfer-panel">
  <p>{label}</p>
  <ActionButton label="Choose file" onclick={chooseFile}>Choose file</ActionButton>
  {#if message}<small class="field-help" role="status">{message}</small>{/if}
</div>
{#if source}<StagedUploadSheet
    {api}
    {purpose}
    {source}
    onComplete={finish}
    onClose={() => (source = undefined)}
  />{/if}
