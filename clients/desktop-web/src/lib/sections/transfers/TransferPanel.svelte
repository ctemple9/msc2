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
  let fileInput: HTMLInputElement;
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
    file =
      (await (await getPlatform()).pickFile({ label }, () => selectBrowserFile(fileInput))) ??
      undefined;
  }

  async function captureBrowserFile(event: Event): Promise<void> {
    const browserFile = (event.currentTarget as HTMLInputElement).files?.[0];
    file = browserFile ? await toPickedFile(browserFile) : undefined;
  }

  function selectBrowserFile(input: HTMLInputElement): Promise<PickedFile | null> {
    return new Promise((resolve) => {
      input.addEventListener(
        'change',
        async () => resolve(input.files?.[0] ? await toPickedFile(input.files[0]) : null),
        { once: true },
      );
      input.click();
    });
  }

  async function toPickedFile(browserFile: File): Promise<PickedFile> {
    return { name: browserFile.name, bytes: new Uint8Array(await browserFile.arrayBuffer()) };
  }
</script>

<div class="inline-form transfer-panel">
  <div class="field">
    <label for={`file-${purpose}`}>{label}</label><input
      bind:this={fileInput}
      id={`file-${purpose}`}
      type="file"
      onchange={(event) => void captureBrowserFile(event)}
    />
    <ActionButton label="Choose file" onclick={() => void chooseFile()}>Choose file</ActionButton>
  </div>
  <ActionButton label="Stage file" onclick={stage}>Stage</ActionButton>
  {#if progress}<div class="progress-bar" aria-label="Upload progress">
      <span style={`width: ${progress}%`}></span>
    </div>{/if}
  {#if message}<small class="field-help" role="status">{message}</small>{/if}
</div>
