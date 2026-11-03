<script lang="ts">
  import { onDestroy } from 'svelte';
  import { reportTransferFailure } from '../shared/transfer-notices';
  // Ports DetailsComponentsTabView.swift's isShowingModpackImporter flow
  // (stage the archive, sniff mrpack vs CurseForge zip, import) as its own
  // step sequence: stage -> inspect -> review -> importing -> done/failed.
  // When the import reaches a CurseForge author-blocked-file checkpoint
  // (D-027), ModpackImportResultDTO.pendingManualFiles hands off to
  // CurseForgeManualDownloadSheet instead of finishing here.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import Select from '../../components/base/Select.svelte';
  import VisibilityIcon from '../../components/base/VisibilityIcon.svelte';
  import { ApiError } from '../../api/client';
  import {
    discardUpload,
    pickUploadSource,
    sourceApi,
    type UploadSource,
  } from '../shared/upload-source';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';
  import { pollOperation } from '../worlds/model';
  import { addonPaths } from './model';
  import StagedUploadSheet from './StagedUploadSheet.svelte';

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
    | ReviewStep
    | {
        kind: 'curseforge-key';
        inspection: Schema['ModpackInspectionResultDTO'];
        stagedUploadId: string;
        reason?: string;
      }
    | { kind: 'importing' }
    | { kind: 'done'; message: string }
    | { kind: 'failed'; message: string };

  type ReviewStep = {
    kind: 'review';
    inspection: Schema['ModpackInspectionResultDTO'];
    stagedUploadId: string;
  };

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
  let step: Step = { kind: 'stage' };
  let importStatus = 'Importing modpack and resolving provider files…';
  let action: 'import' | 'replace' = 'import';
  let curseforgeApiKey = '';
  let curseforgeApiKeyVisible = false;
  let curseforgeKeySaving = false;
  let curseforgeKeyNotice = '';
  let pendingModpackSource: UploadSource | undefined;

  async function chooseAndStage(): Promise<void> {
    if (!api || (!api.selectUploadSource && !api.uploadFile)) return;
    try {
      const picked = await pickUploadSource(
        api,
        'modpack-archive',
        {
          label: 'Choose a modpack archive',
          extensions: ['mrpack', 'zip'],
        },
        {},
        () => !disposed,
      );
      if (picked) {
        pendingModpackSource = picked;
        transferApi = sourceApi(picked, api);
      }
    } catch (error) {
      step = {
        kind: 'failed',
        message: errorMessage(error) || 'Could not select this archive.',
      };
    }
  }

  async function inspectUploadedModpack(
    stagedUploadId: string,
    callbackApi?: ScreenApi,
  ): Promise<void> {
    if (!api || !pendingModpackSource)
      throw new Error('The selected modpack is no longer available.');
    stagedInput = stagedUploadId;
    consumed = false;
    step = { kind: 'inspecting' };
    const inspection = await mutate<Schema['ModpackInspectionResultDTO']>(
      callbackApi ?? transferApi ?? api,
      addonPaths.inspectPack,
      { stagedUploadId },
    );
    const review: ReviewStep = {
      kind: 'review',
      inspection,
      stagedUploadId,
    };
    if (inspection.format === 'curseforge') {
      try {
        const keyStatus = await (callbackApi ?? transferApi ?? api)!.get<
          Schema['CurseForgeApiKeyStatusDTO']
        >('/v1/config/curseforge');
        if (!keyStatus.configured) {
          step = {
            kind: 'curseforge-key',
            inspection,
            stagedUploadId,
          };
          return;
        }
      } catch {
        // The import request remains the authoritative check if the status
        // endpoint is unavailable or the client is talking to an older agent.
      }
    }
    step = review;
  }

  function closePendingModpack(): void {
    pendingModpackSource = undefined;
    if (step.kind === 'inspecting') {
      stagedInput = undefined;
      step = { kind: 'stage' };
    }
  }

  async function startImport(): Promise<void> {
    if (step.kind !== 'review') return;
    const review = step;
    const { stagedUploadId } = review;
    step = { kind: 'importing' };
    try {
      const result = await mutate<Schema['ModpackImportResultDTO']>(
        transferApi ?? api,
        addonPaths.importPack,
        {
          action,
          stagedUploadId,
        },
      );
      consumed = true;
      stagedInput = undefined;
      if (disposed) return;
      const pendingManualFiles = result.pendingManualFiles ?? [];
      if (pendingManualFiles.length > 0) {
        onManualFilesPending(result.operationId, pendingManualFiles);
        onClose();
        return;
      }
      const operation = await pollOperation(transferApi ?? api, result.operationId, (tick) => {
        importStatus = tick.statusLine ?? 'Importing modpack…';
      });
      if (operation?.state !== 'succeeded')
        throw new Error(operation?.error?.message ?? 'Modpack import did not complete.');
      if (disposed) return;
      step = { kind: 'done', message: operation.statusLine ?? 'Modpack imported.' };
      onImported();
    } catch (error) {
      if (error instanceof ApiError && error.error.code === 'missing_curseforge_api_key') {
        step = {
          kind: 'curseforge-key',
          inspection: review.inspection,
          stagedUploadId: review.stagedUploadId,
          reason: error.error.message,
        };
        return;
      }
      step = {
        kind: 'failed',
        message: errorMessage(error) || 'Failed to start the import.',
      };
    }
  }

  function skipCurseForgeKeySetup(): void {
    if (step.kind !== 'curseforge-key') return;
    step = {
      kind: 'review',
      inspection: step.inspection,
      stagedUploadId: step.stagedUploadId,
    };
    curseforgeKeyNotice = '';
  }

  async function saveCurseForgeKeyAndResume(): Promise<void> {
    if (step.kind !== 'curseforge-key' || !curseforgeApiKey.trim() || curseforgeKeySaving) return;
    const pending = step;
    curseforgeKeySaving = true;
    curseforgeKeyNotice = '';
    try {
      const status = await mutate<Schema['CurseForgeApiKeyStatusDTO']>(
        transferApi ?? api,
        '/v1/config/curseforge',
        { apiKey: curseforgeApiKey.trim() },
      );
      curseforgeApiKey = '';
      if (!status.configured) {
        curseforgeKeyNotice = 'The agent did not save a CurseForge API key.';
        return;
      }
      step = {
        kind: 'review',
        inspection: pending.inspection,
        stagedUploadId: pending.stagedUploadId,
      };
      await startImport();
    } catch (error) {
      curseforgeKeyNotice = errorMessage(error) || 'The CurseForge API key could not be saved.';
    } finally {
      curseforgeKeySaving = false;
    }
  }
</script>

<Sheet
  title="Import Modpack"
  size="md"
  visible={!pendingModpackSource}
  onClose={step.kind === 'inspecting' || step.kind === 'importing' ? undefined : closeSelection}
>
  {#if step.kind === 'stage'}
    <div class="body">
      <p class="explain">
        Choose a Modrinth (.mrpack) or CurseForge (.zip) modpack archive. It's inspected before
        anything changes on the server.
      </p>
      <div class="footer">
        <Button variant="secondary" onclick={closeSelection}>Cancel</Button>
        <Button variant="primary" onclick={() => void chooseAndStage()}>Choose Archive…</Button>
      </div>
    </div>
  {:else if step.kind === 'inspecting'}
    <p class="explain">Inspecting archive…</p>
  {:else if step.kind === 'curseforge-key'}
    <div class="body">
      <p class="lede">CurseForge key required</p>
      <p class="explain">
        This archive is a CurseForge modpack. MSC needs an API key to look up its files before the
        import can begin. The key is saved on the connected agent and is never shown again.
      </p>
      {#if step.reason}<p class="key-notice" role="alert">{step.reason}</p>{/if}
      <p class="key-link">
        <a href="https://console.curseforge.com/" target="_blank" rel="noreferrer"
          >Open CurseForge API Console</a
        >
      </p>
      <div class="key-control">
        <Field
          bind:value={curseforgeApiKey}
          type={curseforgeApiKeyVisible ? 'text' : 'password'}
          placeholder="Paste API key"
          width="100%"
          onkeydown={(event) => event.key === 'Enter' && void saveCurseForgeKeyAndResume()}
        />
        <button
          type="button"
          class="visibility-toggle"
          aria-label={curseforgeApiKeyVisible ? 'Hide API key' : 'Show API key'}
          aria-pressed={curseforgeApiKeyVisible}
          title={curseforgeApiKeyVisible ? 'Hide API key' : 'Show API key'}
          onclick={() => (curseforgeApiKeyVisible = !curseforgeApiKeyVisible)}
        >
          <VisibilityIcon visible={curseforgeApiKeyVisible} />
        </button>
      </div>
      {#if curseforgeKeyNotice}<p class="key-notice" role="alert">{curseforgeKeyNotice}</p>{/if}
      <div class="footer">
        <Button variant="secondary" onclick={skipCurseForgeKeySetup}>Skip for now</Button>
        <Button
          variant="primary"
          disabled={curseforgeKeySaving || !curseforgeApiKey.trim()}
          onclick={() => void saveCurseForgeKeyAndResume()}
          >{curseforgeKeySaving ? 'Saving…' : 'Save key and continue'}</Button
        >
      </div>
    </div>
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
        <Button variant="secondary" onclick={closeSelection}>Cancel</Button>
        <Button variant="primary" onclick={() => void startImport()}>Import</Button>
      </div>
    </div>
  {:else if step.kind === 'importing'}
    <p class="explain">{importStatus}</p>
  {:else if step.kind === 'done'}
    <div class="body">
      <p class="lede">Import Complete</p>
      <p class="explain">{step.message}</p>
      <div class="footer">
        <Button variant="primary" onclick={closeSelection}>Done</Button>
      </div>
    </div>
  {:else}
    <div class="body">
      <p class="lede error-text">Import Failed</p>
      <p class="explain">{step.message}</p>
      <div class="footer">
        <Button variant="secondary" onclick={closeSelection}>Close</Button>
      </div>
    </div>
  {/if}
</Sheet>

{#if pendingModpackSource}
  <StagedUploadSheet
    {api}
    purpose="modpack-archive"
    completionStage="inspecting"
    source={pendingModpackSource}
    onComplete={(upload, scoped) => inspectUploadedModpack(upload.stagedUploadId, scoped)}
    onClose={closePendingModpack}
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
  .key-notice {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-status-warn);
  }
  .key-link {
    margin: 0;
    font-size: 12px;
  }
  .key-link a {
    color: var(--msc2-text-secondary);
  }
  .key-control {
    position: relative;
  }
  .visibility-toggle {
    position: absolute;
    top: 50%;
    right: 8px;
    display: grid;
    width: 28px;
    height: 28px;
    padding: 0;
    transform: translateY(-50%);
    place-items: center;
    border: 0;
    background: transparent;
    color: var(--msc2-text-tertiary);
    cursor: pointer;
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
