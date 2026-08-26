<script lang="ts">
  // Ports DetailsComponentsTabView.swift's isShowingModpackImporter flow
  // (stage the archive, sniff mrpack vs CurseForge zip, import) as its own
  // step sequence: stage -> inspect -> review -> importing -> done/failed.
  // When the import reaches a CurseForge author-blocked-file checkpoint
  // (D-027), ModpackImportResultDTO.pendingManualFiles hands off to
  // CurseForgeManualDownloadSheet instead of finishing here.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Select from '../../components/base/Select.svelte';
  import { getPlatform } from '../../platform';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { addonPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let onClose: () => void;
  export let onImported: () => void;
  export let onManualFilesPending: (
    operationId: string,
    files: Schema['ModpackManualFileEntryDTO'][],
  ) => void;

  type Step =
    | { kind: 'stage' }
    | { kind: 'inspecting' }
    | { kind: 'review'; inspection: Schema['ModpackInspectionResultDTO']; stagedUploadId: string }
    | { kind: 'importing' }
    | { kind: 'done'; message: string }
    | { kind: 'failed'; message: string };

  let step: Step = { kind: 'stage' };
  let action: 'import' | 'replace' = 'import';
  let fileInput: HTMLInputElement;

  function pickBrowserFile(): Promise<{ name: string; bytes: Uint8Array } | null> {
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
    ).pickFile({ label: 'Choose a modpack archive', extensions: ['mrpack', 'zip'] }, () =>
      pickBrowserFile(),
    );
    if (!picked) return;
    step = { kind: 'inspecting' };
    try {
      const staged = await api.upload('modpack-archive', picked.bytes);
      const inspection = await mutate<Schema['ModpackInspectionResultDTO']>(
        api,
        addonPaths.inspectPack,
        { stagedUploadId: staged.stagedUploadId },
      );
      step = { kind: 'review', inspection, stagedUploadId: staged.stagedUploadId };
    } catch (error) {
      step = {
        kind: 'failed',
        message: error instanceof Error ? error.message : 'Failed to inspect this archive.',
      };
    }
  }

  async function startImport(): Promise<void> {
    if (step.kind !== 'review') return;
    const { stagedUploadId } = step;
    step = { kind: 'importing' };
    try {
      const result = await mutate<Schema['ModpackImportResultDTO']>(api, addonPaths.importPack, {
        action,
        stagedUploadId,
      });
      const pendingManualFiles = result.pendingManualFiles ?? [];
      if (pendingManualFiles.length > 0) {
        onManualFilesPending(result.operationId, pendingManualFiles);
        onClose();
        return;
      }
      step = { kind: 'done', message: result.message };
      onImported();
    } catch (error) {
      step = {
        kind: 'failed',
        message: error instanceof Error ? error.message : 'Failed to start the import.',
      };
    }
  }
</script>

<Sheet
  title="Import Modpack"
  size="md"
  onClose={step.kind === 'inspecting' || step.kind === 'importing' ? undefined : onClose}
>
  {#if step.kind === 'stage'}
    <div class="body">
      <p class="explain">
        Choose a Modrinth (.mrpack) or CurseForge (.zip) modpack archive. It's inspected before
        anything changes on the server.
      </p>
      <input bind:this={fileInput} type="file" accept=".mrpack,.zip" class="hidden-input" />
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button variant="primary" onclick={() => void chooseAndStage()}>Choose Archive…</Button>
      </div>
    </div>
  {:else if step.kind === 'inspecting'}
    <p class="explain">Inspecting archive…</p>
  {:else if step.kind === 'review'}
    <div class="body">
      <div class="summary">
        <div class="row">
          <span class="label">Pack</span>
          <span class="value">{step.inspection.packName ?? 'Unnamed pack'}</span>
        </div>
        {#if step.inspection.packVersion}
          <div class="row">
            <span class="label">Version</span>
            <span class="value">{step.inspection.packVersion}</span>
          </div>
        {/if}
        <div class="row">
          <span class="label">Minecraft</span>
          <span class="value"
            >{step.inspection.minecraftVersion ?? 'Not reported'}{step.inspection.loaderName
              ? ` · ${step.inspection.loaderName}`
              : ''}</span
          >
        </div>
        <div class="row">
          <span class="label">Files</span>
          <span class="value">{step.inspection.fileCount}</span>
        </div>
        {#if step.inspection.manualFiles.length > 0}
          <div class="row">
            <span class="label">Manual downloads</span>
            <span class="value warn">{step.inspection.manualFiles.length} blocked by author</span>
          </div>
        {/if}
      </div>
      {#if step.inspection.warnings && step.inspection.warnings.length > 0}
        <ul class="warnings">
          {#each step.inspection.warnings as warning}
            <li>{warning}</li>
          {/each}
        </ul>
      {/if}
      <div class="action-row">
        <span class="label">Apply as</span>
        <Select
          options={[
            { value: 'import', label: 'New server' },
            { value: 'replace', label: 'Replace this server' },
          ]}
          bind:value={action}
          width="auto"
        />
      </div>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button variant="primary" onclick={() => void startImport()}>Import</Button>
      </div>
    </div>
  {:else if step.kind === 'importing'}
    <p class="explain">Starting import…</p>
  {:else if step.kind === 'done'}
    <div class="body">
      <p class="lede">Import Started</p>
      <p class="explain">{step.message}</p>
      <div class="footer">
        <Button variant="primary" onclick={onClose}>Done</Button>
      </div>
    </div>
  {:else}
    <div class="body">
      <p class="lede error-text">Import Failed</p>
      <p class="explain">{step.message}</p>
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
  .summary {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--msc2-tier-chrome);
    border-radius: 8px;
    padding: 12px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .label {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .value {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .value.warn {
    color: var(--msc2-status-warn);
  }
  .warnings {
    margin: 0;
    padding-left: 18px;
    font-size: 12px;
    color: var(--msc2-status-warn);
    line-height: 1.6;
  }
  .action-row {
    display: flex;
    align-items: center;
    gap: 10px;
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
