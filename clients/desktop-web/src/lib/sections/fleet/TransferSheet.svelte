<script lang="ts">
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import Field from '../../components/base/Field.svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import { onDestroy } from 'svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { bytesLabel, errorMessage, mutate } from '../shared/types';
  import { pollOperation } from './wizard/model';

  export let api: ScreenApi | undefined = undefined;
  export let servers: readonly Schema['ServerDTO'][] = [];
  export let onClose: () => void;
  export let onImported: () => void = () => {};

  type TransferView = 'home' | 'import';
  type ImportMode = 'merge' | 'replaceAll';

  let view: TransferView = 'home';
  let importMode: ImportMode = 'merge';
  let sourcePath = '';
  let backupPath = '';
  let isWorking = false;
  let notice = '';
  let noticeIsError = false;

  let selected: { stagedUploadId: string; agentHostId: string; fileName: string } | undefined;
  let selectedApi: ScreenApi | undefined;
  let controller: AbortController | undefined;
  let nativeTask: Promise<unknown> | undefined;
  let redeemed = false;
  async function releaseSelection(): Promise<void> {
    if (selected && !redeemed)
      await selectedApi?.delete?.(`/v1/staged-uploads/${selected.stagedUploadId}`);
    selected = undefined;
    selectedApi = undefined;
  }
  onDestroy(() => {
    controller?.abort();
    if (!isWorking) void releaseSelection().catch(() => undefined);
  });
  async function closeSheet(): Promise<void> {
    if (isWorking && !controller) return;
    controller?.abort();
    try {
      await nativeTask;
    } catch (error) {
      if (
        typeof error === 'object' &&
        error !== null &&
        'cleanup' in error &&
        error.cleanup === 'unconfirmed'
      ) {
        notice = errorMessage(error);
        noticeIsError = true;
        return;
      }
    }
    try {
      await releaseSelection();
      onClose();
    } catch (error) {
      notice = errorMessage(error);
      noticeIsError = true;
    }
  }
  async function exportServers(): Promise<void> {
    const scoped = api?.snapshot?.() ?? api;
    if (!scoped?.saveNativeDownload) {
      notice = 'Native transfer saving needs a connected desktop.';
      noticeIsError = true;
      return;
    }
    isWorking = true;
    notice = '';
    controller = new AbortController();
    let exportId: string | undefined;
    try {
      const caps = await scoped.get<Schema['CapabilitiesDTO']>('/v1/capabilities');
      if (!caps.nativeTransfers?.serverArchive)
        throw new Error('Update the agent to export a verified native transfer file.');
      if (controller.signal.aborted) throw new Error('Export cancelled before preparing.');
      const task = (async () => {
        const result =
          await scoped.post<Schema['ServerTransferExportResultDTO']>('/v1/servers/export');
        exportId = result.stagedDownloadId;
        if (controller?.signal.aborted) {
          await scoped.delete?.(`/v1/staged-downloads/${result.stagedDownloadId}`);
          exportId = undefined;
          throw new Error('Export cancelled before saving.');
        }
        await scoped.saveNativeDownload!(
          result.stagedDownloadId,
          result.fileName,
          result.sizeBytes,
          { signal: controller?.signal },
        );
        exportId = undefined;
        return result;
      })();
      nativeTask = task;
      const result = await task;
      if (api?.hostId?.() !== scoped.hostId?.()) return;
      notice = `Exported ${result.serverCount} server${result.serverCount === 1 ? '' : 's'} (${bytesLabel(result.sizeBytes)}).`;
      noticeIsError = false;
    } catch (error) {
      notice = errorMessage(error);
      noticeIsError = true;
      const confirmed =
        typeof error === 'object' &&
        error !== null &&
        'cleanup' in error &&
        error.cleanup === 'confirmed';
      if (exportId && !confirmed) {
        try {
          if (!scoped.delete) throw new Error('Cleanup is unavailable.');
          await scoped.delete(`/v1/staged-downloads/${exportId}`);
        } catch {
          notice += ' Agent export cleanup could not be confirmed.';
        }
      }
    } finally {
      isWorking = false;
      nativeTask = undefined;
      controller = undefined;
    }
  }
  async function chooseTransferFile(): Promise<void> {
    const scoped = api?.snapshot?.() ?? api;
    if (!scoped?.pickNativeUpload) {
      notice = 'Native selection needs a connected desktop.';
      noticeIsError = true;
      return;
    }
    isWorking = true;
    notice = '';
    controller = new AbortController();
    try {
      await releaseSelection();
      redeemed = false;
      const task = scoped.pickNativeUpload('msc-transfer-archive', false, {
        signal: controller.signal,
      });
      nativeTask = task;
      const result = await task;
      if (result) {
        if (api?.hostId?.() !== result.agentHostId) {
          await scoped.delete?.(`/v1/staged-uploads/${result.stagedUploadId}`);
          throw new Error('The selected host changed. Choose the transfer file again.');
        }
        selected = result;
        selectedApi = scoped;
        sourcePath = result.fileName;
      }
    } catch (error) {
      notice = errorMessage(error);
      noticeIsError = true;
    } finally {
      isWorking = false;
      nativeTask = undefined;
      controller = undefined;
    }
  }
  async function importTransfer(): Promise<void> {
    const path = sourcePath.trim();
    if (!path) return;
    if (importMode === 'replaceAll' && !backupPath.trim()) {
      notice = 'Choose a backup path before replacing the current servers.';
      noticeIsError = true;
      return;
    }
    const native = selected && path === selected.fileName ? selected : undefined;
    const scoped = native ? selectedApi : (api?.snapshot?.() ?? api);
    if (native && api?.hostId?.() !== native.agentHostId) {
      notice = 'This source belongs to another host. Choose it again.';
      noticeIsError = true;
      return;
    }
    isWorking = true;
    notice = '';
    try {
      const result = await mutate<Schema['ServerImportResultDTO']>(scoped, '/v1/servers/import', {
        action: 'importTransfer',
        ...(native ? { stagedUploadId: native.stagedUploadId } : { sourcePath: path }),
        importKind: 'transfer',
        transferMode: importMode,
        ...(importMode === 'replaceAll' ? { backupPath: backupPath.trim() } : {}),
      });
      if (native) redeemed = true;
      const operation = result.operationId
        ? await pollOperation(scoped, result.operationId)
        : undefined;
      if (operation?.state === 'failed' || operation?.state === 'cancelled')
        throw new Error(operation.error?.message ?? 'Transfer import did not complete.');
      if (api?.hostId?.() !== scoped?.hostId?.()) return;
      notice = result.message;
      noticeIsError = false;
      onImported();
    } catch (error) {
      notice = errorMessage(error);
      noticeIsError = true;
    } finally {
      isWorking = false;
    }
  }
</script>

<Sheet title="Server Transfer" size="md" onClose={() => void closeSheet()}>
  {#if view === 'home'}
    <div class="transfer">
      <div class="intro">
        <h2>Move servers between MSC installations</h2>
        <p>
          A transfer file bundles server settings, worlds, backups, mods, plugins, and config files.
          Machine-specific credentials stay behind.
        </p>
      </div>

      <div class="actions">
        <Card>
          <div class="action-copy">
            <p class="overline">FROM THIS INSTALLATION</p>
            <h3>Export transfer file</h3>
            <p>
              {servers.length} configured server{servers.length === 1 ? '' : 's'} will be bundled into
              a downloadable <code>.msctransfer</code> file.
            </p>
          </div>
          <Button
            variant="primary"
            onclick={() => void exportServers()}
            disabled={isWorking || servers.length === 0}
          >
            {isWorking ? 'Exporting…' : 'Export…'}
          </Button>
        </Card>

        <Card>
          <div class="action-copy">
            <p class="overline">TO THIS INSTALLATION</p>
            <h3>Import transfer file</h3>
            <p>
              Bring servers from another MSC installation. You can merge them or replace the current
              set after saving a backup.
            </p>
          </div>
          <Button variant="secondary" onclick={() => (view = 'import')} disabled={isWorking}
            >Import…</Button
          >
        </Card>
      </div>

      {#if notice}<p class:error={noticeIsError} class="notice" role="status">{notice}</p>{/if}
    </div>
  {:else}
    <div class="transfer">
      <div class="intro">
        <h2>Import a transfer file</h2>
        <p>Choose a <code>.msctransfer</code> exported from another MSC installation.</p>
      </div>

      <div class="form">
        <div class="field-row">
          <Field
            bind:value={sourcePath}
            placeholder="Choose a transfer file, or enter a path on the agent"
            disabled={isWorking}
          />
          <Button variant="secondary" onclick={() => void chooseTransferFile()} disabled={isWorking}
            >Choose…</Button
          >
        </div>
        <label class="radio-row">
          <input type="radio" bind:group={importMode} value="merge" disabled={isWorking} />
          <span
            ><strong>Merge</strong><small
              >Add the transferred servers alongside the current servers.</small
            ></span
          >
        </label>
        <label class="radio-row">
          <input type="radio" bind:group={importMode} value="replaceAll" disabled={isWorking} />
          <span
            ><strong>Replace all</strong><small
              >Save a backup first, then replace the current server set.</small
            ></span
          >
        </label>
        {#if importMode === 'replaceAll'}
          <Field
            bind:value={backupPath}
            placeholder="Backup path, e.g. /path/MSC-before-replace.msctransfer"
            disabled={isWorking}
          />
        {/if}
      </div>

      {#if notice}<p class:error={noticeIsError} class="notice" role="status">{notice}</p>{/if}
      <div class="footer">
        <Button variant="secondary" onclick={() => (view = 'home')} disabled={isWorking}
          >Back</Button
        >
        <div class="spacer"></div>
        <Button
          variant="primary"
          onclick={() => void importTransfer()}
          disabled={isWorking || !sourcePath.trim() || redeemed}
        >
          {isWorking ? 'Importing…' : 'Import'}
        </Button>
      </div>
    </div>
  {/if}
</Sheet>

<style>
  .transfer {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .intro {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  h2,
  h3,
  p {
    margin: 0;
  }
  h2 {
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  h3 {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .intro p,
  .action-copy p,
  .notice {
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .actions :global(.card) {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 18px;
    min-height: 150px;
  }
  .action-copy {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .overline {
    font-size: 10px !important;
    letter-spacing: 0.08em;
    color: var(--msc2-text-tertiary);
  }
  code {
    font-family: var(--msc2-font-mono, monospace);
    color: var(--msc2-text-secondary);
  }
  .notice.error {
    color: var(--msc2-status-warn);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .field-row {
    display: flex;
    gap: 8px;
  }
  .field-row :global(.field) {
    flex: 1;
  }
  .radio-row {
    display: flex;
    gap: 9px;
    align-items: flex-start;
    font-size: 12px;
    color: var(--msc2-text-primary);
  }
  .radio-row input {
    margin-top: 2px;
    accent-color: var(--msc2-status-ok);
  }
  .radio-row span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .radio-row small {
    color: var(--msc2-text-tertiary);
    line-height: 1.4;
  }
  .footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .spacer {
    flex: 1;
  }
  @media (max-width: 620px) {
    .actions {
      grid-template-columns: 1fr;
    }
  }
</style>
