<script lang="ts">
  import { onDestroy } from 'svelte';
  import { reportTransferFailure } from '../shared/transfer-notices';
  // Ports ServerEditorWorldTab.swift's importZIPSheetView -- pick an
  // external world archive (not one the agent already knows about, unlike
  // BackupsPanel's "import this backup as a slot"), stage it, name the new
  // slot, import. Same stage-then-mutate shape as
  // ImportModpackSheet.svelte/WorldSlotCard.svelte's thumbnail upload.
  import Sheet from '../../components/base/Sheet.svelte';
  import Field from '../../components/base/Field.svelte';
  import Button from '../../components/base/Button.svelte';
  import {
    discardUpload,
    pickUploadSource,
    sourceApi,
    type UploadSource,
  } from '../shared/upload-source';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import StagedUploadSheet from '../components/StagedUploadSheet.svelte';
  import { worldPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let onClose: () => void;
  export let onImported: (updated: Schema['WorldSlotsResponseDTO']) => void;

  type Step =
    | { kind: 'pick' }
    | { kind: 'staged'; fileName: string; stagedUploadId: string }
    | { kind: 'importing' }
    | { kind: 'failed'; message: string };

  let stagedInput: string | undefined;
  let consumed = false;
  let disposed = false;
  async function closeSelection(): Promise<void> {
    if (stagedInput && !consumed) {
      try {
        await discardUpload(transferApi, stagedInput);
        stagedInput = undefined;
      } catch (error) {
        reportTransferFailure(transferApi, error);
        return;
      }
    }
    onClose();
  }
  onDestroy(() => {
    disposed = true;
    if (stagedInput && !consumed)
      void discardUpload(transferApi, stagedInput).catch((error) =>
        reportTransferFailure(transferApi, error),
      );
  });
  let transferApi: ScreenApi | undefined;
  let step: Step = { kind: 'pick' };
  let name = '';
  let pendingWorldSource: UploadSource | undefined;
  let picking = false;

  $: trimmedName = name.trim();

  async function chooseAndStage(): Promise<void> {
    if (!api || (!api.selectUploadSource && !api.uploadFile) || picking) return;
    picking = true;
    try {
      const picked = await pickUploadSource(
        api,
        'world-import',
        {
          label: 'Choose a world archive',
          extensions: ['zip', 'mcworld'],
        },
        {},
        () => !disposed,
      );
      if (picked) {
        pendingWorldSource = picked;
        transferApi = sourceApi(picked, api);
      }
    } catch (error) {
      step = {
        kind: 'failed',
        message: error instanceof Error ? error.message : 'Could not select this archive.',
      };
    } finally {
      picking = false;
    }
  }

  function finishWorldUpload(upload: Schema['StagedUploadCompleteResultDTO']): void {
    if (!pendingWorldSource) throw new Error('The selected world archive is no longer available.');
    name = pendingWorldSource.name.replace(/\.(zip|mcworld)$/i, '');
    stagedInput = upload.stagedUploadId;
    consumed = false;
    step = {
      kind: 'staged',
      fileName: pendingWorldSource.name,
      stagedUploadId: upload.stagedUploadId,
    };
  }

  function closePendingWorld(): void {
    pendingWorldSource = undefined;
    if (step.kind === 'importing') step = { kind: 'pick' };
  }

  async function submit(): Promise<void> {
    if (step.kind !== 'staged' || !trimmedName) return;
    const { stagedUploadId } = step;
    step = { kind: 'importing' };
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(
        transferApi ?? api,
        worldPaths.import,
        {
          name: trimmedName,
          stagedUploadId,
        },
      );
      consumed = true;
      stagedInput = undefined;
      if (disposed) return;
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
  title="Import Archive as New World"
  size="sm"
  visible={!pendingWorldSource}
  onClose={step.kind === 'importing' ? undefined : closeSelection}
>
  <div class="body">
    {#if step.kind === 'pick'}
      <p class="explain">
        Choose a ZIP or .mcworld archive. Enclosing folders are handled automatically.
      </p>
      <div class="footer">
        <Button variant="secondary" onclick={closeSelection}>Cancel</Button>
        <Button variant="primary" disabled={picking} onclick={() => void chooseAndStage()}>
          {picking ? 'Opening…' : 'Choose Archive…'}
        </Button>
      </div>
    {:else if step.kind === 'staged'}
      <p class="explain">Selected: {step.fileName}</p>
      <div class="field-group">
        <span class="msc2-type-overline">Slot Name</span>
        <Field bind:value={name} placeholder="Slot name" />
      </div>
      <div class="footer">
        <Button variant="secondary" onclick={closeSelection}>Cancel</Button>
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
        <Button variant="secondary" onclick={closeSelection}>Close</Button>
      </div>
    {/if}
  </div>
</Sheet>

{#if pendingWorldSource}
  <StagedUploadSheet
    {api}
    purpose="world-import"
    source={pendingWorldSource}
    onComplete={finishWorldUpload}
    onClose={closePendingWorld}
  />
{/if}

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
