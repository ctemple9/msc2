<script lang="ts">
  import Sheet from '../../../components/base/Sheet.svelte';
  import Button from '../../../components/base/Button.svelte';
  import CurseForgeManualDownloadSheet from '../../components/CurseForgeManualDownloadSheet.svelte';
  import { openExternal } from '../../../platform';
  import type { Schema, ScreenApi } from '../../shared/types';
  import type { ModpackCreationSummary } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let summary: ModpackCreationSummary;
  export let onClose: () => void;

  let showingRecovery = false;
  let linkError = '';

  $: unresolvedFiles = summary.unresolvedFiles;
  $: hasProblems = unresolvedFiles.length > 0;

  async function openProviderPage(url: string): Promise<void> {
    linkError = '';
    try {
      await openExternal(url);
    } catch {
      linkError = 'MSC could not open that provider page.';
    }
  }

  function updateRemaining(files: Schema['ModpackManualFileEntryDTO'][]): void {
    summary = { ...summary, unresolvedFiles: files };
  }

  function finishRecovery(): void {
    updateRemaining([]);
    showingRecovery = false;
  }
</script>

<Sheet title="Modpack installed" size="lg" {onClose}>
  <div class="summary">
    <div class="intro">
      <h2>{summary.packName}</h2>
      <p>{summary.packVersion} · {summary.provider}</p>
    </div>

    {#if hasProblems}
      <section class="attention" aria-labelledby="modpack-attention-title">
        <div class="section-heading">
          <div>
            <h3 id="modpack-attention-title">Needs attention</h3>
            <p>
              {unresolvedFiles.length} file{unresolvedFiles.length === 1 ? '' : 's'} could not be downloaded
              automatically.
            </p>
          </div>
          <Button variant="primary" size="sm" onclick={() => (showingRecovery = true)}>
            Resolve now
          </Button>
        </div>

        <div class="problem-list">
          {#each unresolvedFiles as file (file.fileId)}
            <div class="problem-row">
              <div class="file-info">
                <span class="file-name">{file.projectName || file.fileName}</span>
                <span class="file-detail">{file.fileName}</span>
                {#if file.provider || file.reason}
                  <span class="file-reason"
                    >{file.provider ?? 'Provider'}: {file.reason ?? 'Needs attention'}</span
                  >
                {/if}
              </div>
              {#if file.projectUrl}
                <button
                  type="button"
                  class="provider-link"
                  onclick={() => void openProviderPage(file.projectUrl ?? '')}
                >
                  Open {file.provider ?? 'provider'} page
                </button>
              {/if}
            </div>
          {/each}
        </div>
        {#if linkError}<p class="error" role="status">{linkError}</p>{/if}
      </section>
    {:else}
      <section class="complete" aria-label="Modpack download status">
        <h3>All pack files downloaded</h3>
        <p>MSC installed every manifest file it could identify for this pack.</p>
      </section>
    {/if}

    {#if summary.notInstalledFiles.length > 0}
      <section class="not-installed" aria-labelledby="modpack-not-installed-title">
        <h3 id="modpack-not-installed-title">Not installed on the server</h3>
        <p>
          These exact matches are client-only and belong in the player’s modpack, not the server.
        </p>
        <div class="file-list">
          {#each summary.notInstalledFiles as file}
            <span>{file}</span>
          {/each}
        </div>
      </section>
    {/if}

    <section class="downloaded" aria-labelledby="modpack-downloaded-title">
      <div class="section-heading">
        <div>
          <h3 id="modpack-downloaded-title">Downloaded</h3>
          <p>
            {summary.installedFiles.length} file{summary.installedFiles.length === 1 ? '' : 's'} installed
            in the server.
          </p>
        </div>
      </div>
      {#if summary.installedFiles.length > 0}
        <div class="file-list">
          {#each summary.installedFiles as file}
            <span>{file}</span>
          {/each}
        </div>
      {:else}
        <p class="empty">No manifest files were downloaded.</p>
      {/if}
    </section>

    <div class="footer">
      <Button variant="primary" onclick={onClose}>Done</Button>
    </div>
  </div>
</Sheet>

{#if showingRecovery}
  <CurseForgeManualDownloadSheet
    {api}
    operationId={summary.operationId}
    files={unresolvedFiles}
    onClose={() => (showingRecovery = false)}
    onRemainingChange={updateRemaining}
    onAllResolved={finishRecovery}
  />
{/if}

<style>
  .summary {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .intro h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .intro p,
  .section-heading p,
  .complete p,
  .empty {
    margin: 4px 0 0;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.5;
  }
  .attention,
  .complete,
  .not-installed,
  .downloaded {
    padding-top: 16px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .attention {
    border-top-color: var(--msc2-status-warn);
  }
  .section-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }
  h3 {
    margin: 0;
    color: var(--msc2-text-primary);
    font-size: 13px;
    font-weight: 500;
  }
  .problem-list,
  .file-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 14px;
  }
  .problem-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .file-info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .file-name,
  .file-list span {
    color: var(--msc2-text-primary);
    font-size: 12px;
  }
  .file-detail,
  .file-reason {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .provider-link {
    flex: 0 0 auto;
    border: 0;
    padding: 0;
    background: transparent;
    color: var(--msc2-text-secondary);
    font: inherit;
    font-size: 11px;
    text-decoration: underline;
    cursor: pointer;
  }
  .provider-link:hover {
    color: var(--msc2-text-primary);
  }
  .file-list span {
    padding-bottom: 6px;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .error {
    margin: 10px 0 0;
    color: var(--msc2-status-error);
    font-size: 11px;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    padding-top: 2px;
  }
</style>
