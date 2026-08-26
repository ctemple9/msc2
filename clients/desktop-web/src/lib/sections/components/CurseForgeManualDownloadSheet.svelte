<script lang="ts">
  // Ports CurseForgeManualDownloadSheet.swift's purpose (D-027): resolve every
  // CurseForge file an author blocked from API distribution before the
  // modpack-import operation can finish. MSC 1 opens each file's own
  // CurseForge download page directly and watches ~/Downloads for the
  // matching jar to appear; ModpackManualFileEntryDTO carries only
  // fileId/fileName/projectName -- no project or file URL -- so there's
  // nothing to build a real "Open in CurseForge" link from here. Instead:
  // the user finds and downloads the file themselves (named plainly below),
  // then stages it through the same file-picker pattern the rest of this
  // client uses and this sheet binds it to the pending operation via
  // POST /v1/modpacks/{operationId}/manual-file.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import { getPlatform } from '../../platform';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { addonPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let operationId: string;
  export let files: Schema['ModpackManualFileEntryDTO'][];
  export let onClose: () => void;
  export let onAllResolved: () => void;

  let remaining = files;
  let staging: Set<string> = new Set();
  let errorByFile: Record<string, string> = {};
  let fileInput: HTMLInputElement;

  $: allResolved = remaining.length === 0;

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

  async function stageAndBind(entry: Schema['ModpackManualFileEntryDTO']): Promise<void> {
    if (!api?.upload) return;
    const picked = await (
      await getPlatform()
    ).pickFile({ label: `Choose ${entry.fileName}` }, () => pickBrowserFile());
    if (!picked) return;
    staging = new Set(staging).add(entry.fileId);
    const nextErrors = { ...errorByFile };
    delete nextErrors[entry.fileId];
    errorByFile = nextErrors;
    try {
      const staged = await api.upload('curseforge-manual-file', picked.bytes, {
        operationId,
        fileId: entry.fileId,
      });
      const result = await mutate<Schema['ModpackManualFileResultDTO']>(
        api,
        addonPaths.manualFile(operationId),
        { fileId: entry.fileId, stagedUploadId: staged.stagedUploadId },
      );
      remaining = result.remainingManualFiles;
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
      These files' authors disabled CurseForge's API distribution, so they can't be downloaded
      automatically. Find and download each one from CurseForge yourself, then stage it here to
      resume the import.
    </p>
    <div class="list">
      {#each remaining as entry (entry.fileId)}
        <div class="row">
          <div class="info">
            <span class="name">{entry.projectName || entry.fileName}</span>
            <span class="filename">{entry.fileName}</span>
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
            {staging.has(entry.fileId) ? 'Staging…' : 'Choose File…'}
          </Button>
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
  .error {
    font-size: 11px;
    color: var(--msc2-status-error);
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
