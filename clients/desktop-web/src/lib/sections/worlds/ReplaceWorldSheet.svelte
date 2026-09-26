<script lang="ts">
  // Ports ServerEditorWorldTab.swift's replaceWorldSheetView -- swaps the
  // live world's content from an external ZIP, taking a safety backup
  // first. MSC 1 also offers "World Folder…" as a source; a browser file
  // picker has no folder-to-archive equivalent, so this sheet keeps only
  // the ZIP path (WorldReplaceActiveRequestDTO only ever redeems a staged
  // ZIP anyway -- ExistingFolder is unreachable from this route). The level
  // name is read back from the server, unchanged, exactly like the oracle;
  // there's no "rename while replacing" affordance in either.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import { getPlatform } from '../../platform';
  import { onMount } from 'svelte';
  import type { FileChunkSource } from '../../platform/types';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call, mutate } from '../shared/types';
  import StagedUploadSheet from '../components/StagedUploadSheet.svelte';
  import { currentLevelName, pollOperation, settingsPath, worldPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let serverRunning = false;
  export let onClose: () => void;
  export let onReplaced: () => void;

  type Step =
    | { kind: 'pick' }
    | { kind: 'staged'; fileName: string; stagedUploadId: string }
    | { kind: 'replacing'; statusLine: string }
    | { kind: 'failed'; message: string };

  let step: Step = { kind: 'pick' };
  let levelName = 'world';
  let fileInput: HTMLInputElement;
  let pendingWorldSource: FileChunkSource | undefined;
  let picking = false;

  onMount(() => {
    void (async () => {
      const settings = await call<Schema['SettingsResponseDTO'] | undefined>(
        api,
        undefined,
        settingsPath,
      );
      levelName = currentLevelName(settings);
    })();
  });

  function browseBrowserFile(): Promise<FileChunkSource | null> {
    return new Promise((resolve) => {
      fileInput.addEventListener(
        'change',
        () => {
          const browserFile = fileInput.files?.[0];
          resolve(
            browserFile
              ? {
                  name: browserFile.name,
                  size: browserFile.size,
                  readChunk: async (offset, maxBytes) =>
                    new Uint8Array(
                      await browserFile.slice(offset, offset + maxBytes).arrayBuffer(),
                    ),
                  close: async () => undefined,
                }
              : null,
          );
        },
        { once: true },
      );
      fileInput.click();
    });
  }

  async function chooseAndStage(): Promise<void> {
    if (!api?.uploadFile || serverRunning || picking) return;
    picking = true;
    try {
      const picked = await (
        await getPlatform()
      ).pickFileStream({ label: 'Choose a backup ZIP', extensions: ['zip'] }, () =>
        browseBrowserFile(),
      );
      if (picked) pendingWorldSource = picked;
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
    step = {
      kind: 'staged',
      fileName: pendingWorldSource.name,
      stagedUploadId: upload.stagedUploadId,
    };
  }

  function closePendingWorld(): void {
    pendingWorldSource = undefined;
  }

  async function submit(): Promise<void> {
    if (step.kind !== 'staged') return;
    const { stagedUploadId } = step;
    step = { kind: 'replacing', statusLine: 'Replacing world…' };
    try {
      const result = await mutate<Schema['WorldReplaceActiveResultDTO']>(
        api,
        worldPaths.replaceActive,
        { newLevelName: levelName, stagedUploadId },
      );
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId, (tick) => {
          step = { kind: 'replacing', statusLine: tick.statusLine ?? 'Replacing world…' };
        });
        if (operation?.state !== 'succeeded') {
          step = {
            kind: 'failed',
            message: operation?.error?.message ?? 'Replacement did not complete.',
          };
          return;
        }
      }
      onReplaced();
      onClose();
    } catch (error) {
      step = {
        kind: 'failed',
        message: error instanceof Error ? error.message : 'Failed to replace the world.',
      };
    }
  }
</script>

<Sheet
  title="Replace World"
  size="sm"
  visible={!pendingWorldSource}
  onClose={step.kind === 'replacing' ? undefined : onClose}
>
  <div class="body">
    <input bind:this={fileInput} type="file" accept=".zip" class="hidden-input" />
    {#if step.kind === 'pick' || step.kind === 'staged'}
      <p class="explain">
        Swaps in a different world from a backup ZIP. A backup of the current world is taken first.
      </p>
      <p class="warning">
        {serverRunning
          ? 'Stop the server before replacing the world.'
          : 'A safety backup is created automatically before anything changes.'}
      </p>
      {#if step.kind === 'staged'}
        <p class="explain">Selected: {step.fileName}</p>
      {/if}
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        {#if step.kind === 'staged'}
          <Button variant="primary" onclick={() => void submit()}>Apply Replace</Button>
        {:else}
          <Button
            variant="primary"
            disabled={serverRunning || picking}
            onclick={() => void chooseAndStage()}
          >
            {picking ? 'Opening…' : 'Choose ZIP…'}
          </Button>
        {/if}
      </div>
    {:else if step.kind === 'replacing'}
      <p class="explain">{step.statusLine}</p>
    {:else}
      <p class="lede error-text">Replace Failed</p>
      <p class="explain">{step.message}</p>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Close</Button>
      </div>
    {/if}
  </div>
</Sheet>

{#if pendingWorldSource}
  <StagedUploadSheet
    {api}
    purpose="active-world-replace"
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
  .warning {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-status-warn);
    background: var(--msc2-status-warn-tint);
    border-radius: 8px;
    padding: 8px 10px;
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
