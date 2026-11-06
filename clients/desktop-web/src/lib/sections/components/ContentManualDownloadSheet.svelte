<script lang="ts">
  import { onDestroy } from 'svelte';
  import { getPlatform, openExternal } from '../../platform';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { bytesLabel, errorMessage } from '../shared/types';
  import type { FileChunkSource } from '../../platform/types';
  import { reportTransferFailure } from '../shared/transfer-notices';

  export let api: ScreenApi;
  export let serverId: string;
  export let serverName = '';
  export let project: Pick<Schema['CatalogItemDTO'], 'projectId' | 'title'>;
  export let release: Schema['ProviderCatalogReleaseDTO'] | undefined = undefined;
  export let review: Schema['ContentInstallReviewDTO'];
  export let kind: 'mod' | 'plugin';
  export let onClose: () => void;
  export let onInstalled: () => void;

  const connection = api.snapshot?.() ?? api;
  const root = `/v1/servers/${encodeURIComponent(serverId)}/content`;
  let error = '';
  let busy = false;
  let acceptedManual = new Set<string>();
  let uploading: string | undefined;
  let uploadedBytes = 0;
  let operation: Schema['OperationDTO'] | undefined;
  let operationId: string | undefined;
  let observing = false;
  let disposed = false;
  let uploadAbort: AbortController | undefined;
  let source: FileChunkSource | undefined;
  $: pendingManual =
    review?.files.filter(
      (file) => file.manualDownloadRequired && !acceptedManual.has(file.versionId),
    ) ?? [];
  $: terminal = operation && ['succeeded', 'failed', 'cancelled'].includes(operation.state);
  $: installStarted = !!operationId;
  const current = () => !disposed && connection.isCurrent?.() !== false;

  async function downloadPage(file: Schema['ContentInstallFileDTO']): Promise<void> {
    error = '';
    try {
      const response = await connection.get<Schema['ProviderCatalogProjectDTO']>(
        `/v1/catalog/curseforge/projects/${encodeURIComponent(file.projectId)}`,
      );
      if (!current()) return;
      const category = kind === 'plugin' ? 'bukkit-plugins' : 'mc-mods';
      await openExternal(
        `https://www.curseforge.com/minecraft/${category}/${encodeURIComponent(response.project.slug)}/files/${encodeURIComponent(file.versionId)}`,
      );
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    }
  }
  async function chooseManual(file: Schema['ContentInstallFileDTO']): Promise<void> {
    if (!review || uploading || busy || !current()) return;
    uploading = file.versionId;
    uploadedBytes = 0;
    error = '';
    uploadAbort = new AbortController();
    let stagedId: string | undefined;
    let consumed = false;
    try {
      source =
        (await (
          await getPlatform()
        ).pickFileStream({ label: `Choose ${file.fileName}`, extensions: ['jar'] })) ?? undefined;
      if (!source) return;
      if (!current() || uploadAbort.signal.aborted) return;
      if (source.size !== file.byteSize || source.name !== file.fileName)
        throw new Error(
          `Choose the exact ${file.fileName} file (${bytesLabel(file.byteSize)}). MSC will verify its checksum.`,
        );
      if (!connection.uploadAdmittedFile)
        throw new Error('Update this desktop app to upload reviewed manual files.');
      const admission = await connection.post<Schema['StagedUploadBeginResultDTO']>(
        `${root}/install-manual/admit`,
        {
          reviewId: review.reviewId,
          versionId: file.versionId,
        },
      );
      stagedId = admission.stagedUploadId;
      const receipt = await connection.uploadAdmittedFile(admission, source, {
        signal: uploadAbort.signal,
        onProgress: (value) => {
          uploadedBytes = value.bytesUploaded;
        },
      });
      const result = await connection.post<{ success: boolean }>(
        `${root}/install-manual/complete`,
        {
          reviewId: review.reviewId,
          versionId: file.versionId,
          stagedUploadId: receipt.stagedUploadId,
        },
      );
      if (!result.success) throw new Error('The agent did not confirm this manual file.');
      consumed = true;
      if (current()) acceptedManual = new Set(acceptedManual).add(file.versionId);
    } catch (cause) {
      if (current()) error = errorMessage(cause);
      else reportTransferFailure(connection, cause);
    } finally {
      if (stagedId && !consumed) {
        try {
          if (!connection.delete) throw new Error('Partial upload cleanup is unavailable.');
          await connection.delete(`/v1/staged-uploads/${encodeURIComponent(stagedId)}`);
        } catch (cause) {
          reportTransferFailure(connection, cause);
          if (current()) error += ' Partial upload cleanup could not be confirmed.';
        }
      }
      try {
        await source?.close();
      } catch (cause) {
        reportTransferFailure(connection, cause);
      }
      source = undefined;
      uploading = undefined;
      uploadAbort = undefined;
    }
  }
  async function observe(): Promise<void> {
    if (!operationId || observing) return;
    observing = true;
    error = '';
    try {
      while (current()) {
        const value = await connection.get<Schema['OperationDTO']>(
          `/v1/operations/${encodeURIComponent(operationId)}`,
        );
        if (!current()) break;
        operation = value;
        if (value.state === 'succeeded') {
          onInstalled();
          break;
        }
        if (value.state === 'failed') {
          error = value.error?.message ?? 'Installation failed.';
          break;
        }
        if (value.state === 'cancelled') break;
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      observing = false;
    }
  }
  async function install(): Promise<void> {
    if (!review || busy || uploading || pendingManual.length) return;
    busy = true;
    error = '';
    try {
      const accepted = await connection.post<{ operationId: string }>(`${root}/install`, {
        reviewId: review.reviewId,
      });
      operationId = accepted.operationId;
      await observe();
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
  async function cancel(): Promise<void> {
    if (!operationId) return;
    try {
      operation = await connection.post<Schema['OperationDTO']>(
        `/v1/operations/${encodeURIComponent(operationId)}/cancel`,
        {},
      );
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    }
  }
  onDestroy(() => {
    disposed = true;
    uploadAbort?.abort();
  });
</script>

<Sheet
  title="Manual downloads required"
  size="md"
  onClose={busy || uploading ? undefined : onClose}
>
  <div class="identity">
    <h2>{project.title}</h2>
    <p>
      {release?.version.name ?? review.files[0]?.versionId ?? 'Reviewed release'} · {release
        ?.version.versionType ?? 'exact files'} · {kind === 'mod'
        ? 'Server mods'
        : 'Server plugins'}
    </p>
    <p>Selected server: {serverName || serverId}</p>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if review}
    <p class="explain">
      The author requires a browser download for the files below. Select each exact file to verify
      it, then finish the installation. MSC will download the remaining required dependencies
      automatically.
    </p>
    {#each review.warnings as warning}<p class="warning">{warning}</p>{/each}
    <div class="files">
      {#each review.files.filter((file) => file.manualDownloadRequired) as file (file.versionId)}
        <div class="file">
          <div class="file-info">
            <span class="file-title"
              >{file.title}{file.projectId !== project.projectId
                ? ' · Required dependency'
                : ''}</span
            >
            <span class="explain">{file.fileName} · {bytesLabel(file.byteSize)}</span>
            <details>
              <summary>File identity and dependencies</summary>
              <p class="explain">SHA1: {file.sha1}</p>
              <p class="explain">
                Required projects: {file.requiredProjects.join(', ') || 'None declared'}
              </p>
              <p class="explain">
                Incompatible projects: {file.incompatibleProjects.join(', ') || 'None declared'}
              </p>
            </details>
            {#if file.manualDownloadRequired}
              {#if acceptedManual.has(file.versionId)}<span class="verified"
                  >Manual file verified</span
                >
              {:else}
                <p class="warning">
                  The author requires a manual download. Download this exact file in your browser,
                  then select it here.
                </p>
                <div class="actions">
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={!!uploading || busy || installStarted}
                    onclick={() => void downloadPage(file)}>Open download page</Button
                  >
                  <Button
                    size="sm"
                    variant="secondary"
                    disabled={!!uploading || busy || installStarted}
                    onclick={() => void chooseManual(file)}>Select downloaded file</Button
                  >
                </div>
              {/if}
            {/if}
            {#if uploading === file.versionId}
              <p class="explain" role="status">
                Uploading {bytesLabel(uploadedBytes)} of {bytesLabel(file.byteSize)}…
              </p>
              <Button size="sm" variant="secondary" onclick={() => uploadAbort?.abort()}
                >Cancel upload</Button
              >
            {/if}
          </div>
        </div>
      {/each}
    </div>
    {#if operation}
      <p class="explain" role="status">{operation.statusLine ?? operation.state}</p>
      {#if operation.progress}<progress
          value={operation.progress.current}
          max={operation.progress.total}
        ></progress>{/if}
      {#if operation.state === 'succeeded'}<p class="verified">
          Installed. Start or restart the server to use the new content.
        </p>{/if}
    {/if}
    <div class="footer">
      {#if !installStarted}
        <Button
          variant="primary"
          disabled={busy || !!uploading || pendingManual.length > 0}
          onclick={() => void install()}>Finish installation</Button
        >
      {:else if !terminal}
        {#if !observing}<Button size="sm" variant="secondary" onclick={() => void observe()}
            >Reconnect to operation</Button
          >{/if}
        <Button
          size="sm"
          variant="secondary"
          disabled={operation?.cancelable === false}
          onclick={() => void cancel()}>Cancel installation</Button
        >
      {:else}
        <Button variant="secondary" onclick={onClose}>Close</Button>
      {/if}
    </div>
  {/if}
</Sheet>

<style>
  .identity {
    display: grid;
    gap: 4px;
    margin-bottom: 16px;
  }
  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  .identity p,
  .explain {
    margin: 0;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .error,
  .warning,
  .verified {
    font-size: 12px;
    line-height: 1.5;
  }
  .error {
    color: var(--msc2-status-error);
  }
  .warning {
    color: var(--msc2-status-warn);
  }
  .verified {
    color: var(--msc2-status-ok);
  }
  .files {
    margin: 16px 0;
  }
  .file {
    padding: 12px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .file-info {
    display: grid;
    gap: 5px;
    min-width: 0;
  }
  .file-title {
    font-size: 13px;
    font-weight: 500;
    overflow-wrap: anywhere;
  }
  details {
    font-size: 11px;
    color: var(--msc2-text-secondary);
  }
  summary {
    cursor: pointer;
  }
  .actions,
  .footer {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .footer {
    justify-content: flex-end;
    margin-top: 18px;
  }
  progress {
    width: 100%;
    accent-color: var(--msc2-neutral-fill);
  }
</style>
