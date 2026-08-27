<script lang="ts">
  // Ports ServerEditorWorldTab.swift's importZIPSheetView -- pick an
  // external world archive (not one the agent already knows about, unlike
  // BackupsPanel's "import this backup as a slot"), stage it, name the new
  // slot, import. Same stage-then-mutate shape as
  // ImportModpackSheet.svelte/WorldSlotCard.svelte's thumbnail upload.
  import Sheet from '../../components/base/Sheet.svelte';
  import Field from '../../components/base/Field.svelte';
  import Button from '../../components/base/Button.svelte';
  import { getPlatform } from '../../platform';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { worldPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let onClose: () => void;
  export let onImported: (updated: Schema['WorldSlotsResponseDTO']) => void;

  type Step =
    | { kind: 'pick' }
    | { kind: 'staged'; fileName: string; stagedUploadId: string }
    | { kind: 'importing' }
    | { kind: 'failed'; message: string };

  let step: Step = { kind: 'pick' };
  let name = '';
  let fileInput: HTMLInputElement;

  $: trimmedName = name.trim();

  function browseBrowserFile(): Promise<{ name: string; bytes: Uint8Array } | null> {
    return new Promise((resolve) => {
      fileInput.addEventListener(
        'change',
        async () => {
          const browserFile = fileInput.files?.[0];
          resolve(
            browserFile
              ? { name: browserFile.name, bytes: new Uint8Array(await browserFile.arrayBuffer()) }
              : null,
          );
        },
        { once: true },
      );
      fileInput.click();
    });
  }

  async function chooseAndStage(): Promise<void> {
    if (!api?.upload) return;
    const picked = await (
      await getPlatform()
    ).pickFile({ label: 'Choose a world ZIP', extensions: ['zip'] }, () => browseBrowserFile());
    if (!picked) return;
    try {
      const staged = await api.upload('world-import', picked.bytes);
      name = picked.name.replace(/\.zip$/i, '');
      step = { kind: 'staged', fileName: picked.name, stagedUploadId: staged.stagedUploadId };
    } catch (error) {
      step = {
        kind: 'failed',
        message: error instanceof Error ? error.message : 'Failed to stage this archive.',
      };
    }
  }

  async function submit(): Promise<void> {
    if (step.kind !== 'staged' || !trimmedName) return;
    const { stagedUploadId } = step;
    step = { kind: 'importing' };
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.import, {
        name: trimmedName,
        stagedUploadId,
      });
      if (result.updated) onImported(result.updated);
      onClose();
    } catch (error) {
      step = {
        kind: 'failed',
        message: error instanceof Error ? error.message : 'Failed to import this ZIP as a slot.',
      };
    }
  }
</script>

<Sheet
  title="Import ZIP as New World"
  size="sm"
  onClose={step.kind === 'importing' ? undefined : onClose}
>
  <div class="body">
    <input bind:this={fileInput} type="file" accept=".zip" class="hidden-input" />
    {#if step.kind === 'pick'}
      <p class="explain">Choose an external world archive to import as a new world slot.</p>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button variant="primary" onclick={() => void chooseAndStage()}>Choose ZIP…</Button>
      </div>
    {:else if step.kind === 'staged'}
      <p class="explain">Selected: {step.fileName}</p>
      <div class="field-group">
        <span class="msc2-type-overline">Slot Name</span>
        <Field bind:value={name} placeholder="Slot name" />
      </div>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button variant="primary" disabled={!trimmedName} onclick={() => void submit()}>
          Import
        </Button>
      </div>
    {:else if step.kind === 'importing'}
      <p class="explain">Importing…</p>
    {:else}
      <p class="lede error-text">Import Failed</p>
      <p class="explain">{step.message}</p>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Close</Button>
      </div>
    {/if}
  </div>
</Sheet>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .lede {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .error-text {
    color: var(--msc2-status-warn);
  }
  .explain {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .field-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field-group span {
    color: var(--msc2-text-tertiary);
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .hidden-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    overflow: hidden;
  }
</style>
