<script lang="ts">
  // Ports CurseForgeManualDownloadSheet.swift's purpose (D-027): resolve
  // provider-blocked or failed modpack files before the import operation can
  // finish. The agent supplies the reason and provider page when it has one;
  // the user can open that page, choose or drop the matching JAR, retry after
  // a failed attempt, or explicitly skip the file. Every upload is bound to
  // POST /v1/modpacks/{operationId}/manual-file for validation on the agent.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import VisibilityIcon from '../../components/base/VisibilityIcon.svelte';
  import { getPlatform } from '../../platform';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';
  import { addonPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let operationId: string;
  export let files: Schema['ModpackManualFileEntryDTO'][];
  export let onClose: () => void;
  export let onAllResolved: () => void;
  export let onRemainingChange: (files: Schema['ModpackManualFileEntryDTO'][]) => void = () => {};

  let remaining = files;
  let staging: Set<string> = new Set();
  let errorByFile: Record<string, string> = {};
  let fileInput: HTMLInputElement;
  let showCurseForgeKeySetup = false;
  let curseforgeApiKey = '';
  let curseforgeApiKeyVisible = false;
  let curseforgeKeySaving = false;
  let curseforgeKeyNotice = '';

  $: allResolved = remaining.length === 0;
  $: hasCurseForgeFiles = remaining.some((entry) => entry.provider === 'CurseForge');

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

  type PickedFile = { name: string; bytes: Uint8Array };

  function entryForFileName(name: string): Schema['ModpackManualFileEntryDTO'] | undefined {
    const lower = name.toLowerCase();
    return (
      remaining.find((entry) => entry.fileName.toLowerCase() === lower) ??
      (remaining.length === 1 ? remaining[0] : undefined)
    );
  }

  async function stageAndBind(
    entry: Schema['ModpackManualFileEntryDTO'],
    supplied?: PickedFile,
  ): Promise<void> {
    if (!api?.upload) return;
    const picked =
      supplied ??
      (await (
        await getPlatform()
      ).pickFile({ label: `Choose ${entry.fileName}` }, () => pickBrowserFile()));
    if (!picked) return;
    staging = new Set(staging).add(entry.fileId);
    const nextErrors = { ...errorByFile };
    delete nextErrors[entry.fileId];
    errorByFile = nextErrors;
    try {
      const purpose =
        entry.provider && entry.provider !== 'CurseForge'
          ? 'modpack-unresolved-file'
          : 'curseforge-manual-file';
      const staged = await api.upload(purpose, picked.bytes, {
        operationId,
        fileId: entry.fileId,
        fileName: picked.name,
      });
      const result = await mutate<Schema['ModpackManualFileResultDTO']>(
        api,
        addonPaths.manualFile(operationId),
        { fileId: entry.fileId, stagedUploadId: staged.stagedUploadId },
      );
      remaining = result.remainingManualFiles;
      onRemainingChange(remaining);
      if (result.allFilesResolved) onAllResolved();
    } catch (error) {
      errorByFile = {
        ...errorByFile,
        [entry.fileId]: error instanceof Error ? error.message : 'That file did not match.',
      };
    } finally {
      const next = new Set(staging);
      next.delete(entry.fileId);
      staging = next;
    }
  }

  async function handleDrop(event: DragEvent): Promise<void> {
    event.preventDefault();
    const file = event.dataTransfer?.files?.[0];
    if (!file) return;
    const entry = entryForFileName(file.name);
    if (!entry) {
      errorByFile = {
        ...errorByFile,
        __drop: 'Drop a file whose name matches one of the unresolved entries.',
      };
      return;
    }
    await stageAndBind(entry, { name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) });
  }

  async function skipEntry(entry: Schema['ModpackManualFileEntryDTO']): Promise<void> {
    try {
      const result = await mutate<Schema['ModpackManualFileResultDTO']>(
        api,
        addonPaths.manualFile(operationId),
        { fileId: entry.fileId, action: 'skip' },
      );
      remaining = result.remainingManualFiles;
      onRemainingChange(remaining);
      if (result.allFilesResolved) onAllResolved();
    } catch (error) {
      errorByFile = {
        ...errorByFile,
        [entry.fileId]: errorMessage(error) || 'That file could not be skipped.',
      };
    }
  }

  async function saveCurseForgeKey(): Promise<void> {
    if (!curseforgeApiKey.trim() || curseforgeKeySaving) return;
    curseforgeKeySaving = true;
    curseforgeKeyNotice = '';
    try {
      const status = await mutate<Schema['CurseForgeApiKeyStatusDTO']>(
        api,
        '/v1/config/curseforge',
        { apiKey: curseforgeApiKey.trim() },
      );
      curseforgeApiKey = '';
      if (status.configured) {
        curseforgeKeyNotice = 'CurseForge API key saved for this agent.';
        showCurseForgeKeySetup = false;
      } else {
        curseforgeKeyNotice = 'The agent did not save a CurseForge API key.';
      }
    } catch (error) {
      curseforgeKeyNotice = errorMessage(error) || 'The CurseForge API key could not be saved.';
    } finally {
      curseforgeKeySaving = false;
    }
  }
</script>

<Sheet
  title={allResolved ? 'All Files Resolved' : `${remaining.length} File(s) Need a Manual Download`}
  size="md"
  {onClose}
>
  <input bind:this={fileInput} type="file" class="hidden-input" />
  {#if allResolved}
    <p class="explain">
      Every blocked file is staged. The import continues in the background — check the Plugins list
      once it finishes.
    </p>
    <div class="footer">
      <Button variant="primary" onclick={onClose}>Done</Button>
    </div>
  {:else}
    <p class="explain">
      These files could not be downloaded automatically. Use the provider link for each file to
      download it, then stage it here to resume the import.
    </p>
    {#if hasCurseForgeFiles}
      <div class="key-actions">
        <span class="key-hint">Need to update the provider credential?</span>
        <Button
          variant="secondary"
          size="sm"
          onclick={() => (showCurseForgeKeySetup = !showCurseForgeKeySetup)}
          >{showCurseForgeKeySetup ? 'Hide key setup' : 'Set up CurseForge key…'}</Button
        >
      </div>
    {/if}
    {#if showCurseForgeKeySetup && hasCurseForgeFiles}
      <div class="key-setup">
        <p class="key-explain">
          The key is saved on the connected agent and is never shown again.
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
            onkeydown={(event) => event.key === 'Enter' && void saveCurseForgeKey()}
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
        {#if curseforgeKeyNotice}<p class="key-notice" role="status">{curseforgeKeyNotice}</p>{/if}
        <div class="key-footer">
          <Button
            variant="primary"
            size="sm"
            disabled={curseforgeKeySaving || !curseforgeApiKey.trim()}
            onclick={() => void saveCurseForgeKey()}
            >{curseforgeKeySaving ? 'Saving…' : 'Save key'}</Button
          >
        </div>
      </div>
    {/if}
    <div
      class="drop-area"
      role="button"
      tabindex="0"
      aria-label="Drop a downloaded modpack file"
      ondragover={(event) => event.preventDefault()}
      ondrop={handleDrop}
    >
      Drop a downloaded JAR here, or choose it beside the matching file below.
    </div>
    {#if errorByFile.__drop}<p class="error drop-error">{errorByFile.__drop}</p>{/if}
    <div class="list">
      {#each remaining as entry (entry.fileId)}
        <div class="row">
          <div class="info">
            <span class="name">{entry.projectName || entry.fileName}</span>
            <span class="filename">{entry.fileName}</span>
            {#if entry.provider || entry.reason}
              <span class="reason"
                >{entry.provider ?? 'Provider'}: {entry.reason ?? 'Needs attention'}</span
              >
            {/if}
            {#if entry.projectUrl}
              <a class="provider-link" href={entry.projectUrl} target="_blank" rel="noreferrer">
                Open {entry.provider ?? 'provider'} page
              </a>
            {/if}
            {#if errorByFile[entry.fileId]}
              <span class="error">{errorByFile[entry.fileId]}</span>
            {/if}
          </div>
          <Button
            size="sm"
            variant="secondary"
            disabled={staging.has(entry.fileId)}
            onclick={() => void stageAndBind(entry)}
          >
            {staging.has(entry.fileId)
              ? 'Staging…'
              : errorByFile[entry.fileId]
                ? 'Retry'
                : 'Choose File…'}
          </Button>
          <Button size="sm" variant="secondary" onclick={() => void skipEntry(entry)}>Skip</Button>
        </div>
      {/each}
    </div>
    <div class="footer">
      <Button variant="secondary" onclick={onClose}>Close</Button>
    </div>
  {/if}
</Sheet>

<style>
  .explain {
    margin: 0 0 12px;
    font-size: 12px;
    line-height: 1.6;
    color: var(--msc2-text-tertiary);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
  }
  .drop-area {
    margin: 4px 0 10px;
    padding: 12px;
    border: 1px dashed var(--msc2-hairline);
    border-radius: 7px;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
    text-align: center;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .row:first-child {
    border-top: none;
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .filename {
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .reason {
    font-size: 11px;
    color: var(--msc2-status-warn);
  }
  .provider-link {
    width: fit-content;
    font-size: 11px;
    color: var(--msc2-text-secondary);
  }
  .error {
    font-size: 11px;
    color: var(--msc2-status-error);
  }
  .drop-error {
    margin: 0 0 8px;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .key-actions,
  .key-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .key-hint,
  .key-explain,
  .key-notice {
    margin: 0;
    font-size: 11px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .key-explain a {
    color: var(--msc2-text-secondary);
  }
  .key-setup {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .key-notice {
    color: var(--msc2-status-warn);
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
  .hidden-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    overflow: hidden;
  }
</style>
