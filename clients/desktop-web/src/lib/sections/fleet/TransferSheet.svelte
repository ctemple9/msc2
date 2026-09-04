<script lang="ts">
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import Field from '../../components/base/Field.svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import { getPlatform } from '../../platform';
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

  const defaultDownloadLimit = 512 * 1024 * 1024;
  const transferExportPath = '/v1/servers/export';

  async function exportServers(): Promise<void> {
    if (!api?.download) {
      notice = 'Transfer downloads need a connected client.';
      noticeIsError = true;
      return;
    }
    isWorking = true;
    notice = '';
    try {
      const result = await mutate<Schema['ServerTransferExportResultDTO']>(api, transferExportPath);
      const bytes = await api.download(
        result.stagedDownloadId,
        Math.max(defaultDownloadLimit, result.sizeBytes + 16 * 1024 * 1024),
      );
      saveDownload(bytes, result.fileName);
      notice = `Exported ${result.serverCount} server${result.serverCount === 1 ? '' : 's'} (${bytesLabel(result.sizeBytes)}).`;
      noticeIsError = false;
    } catch (error) {
      notice = errorMessage(error);
      noticeIsError = true;
    } finally {
      isWorking = false;
    }
  }

  async function chooseTransferFile(): Promise<void> {
    const path = await (
      await getPlatform()
    ).pickFilePath({ label: 'Choose MSC transfer file', extensions: ['msctransfer'] });
    if (path) sourcePath = path;
  }

  async function importTransfer(): Promise<void> {
    const path = sourcePath.trim();
    if (!path) return;
    if (importMode === 'replaceAll' && !backupPath.trim()) {
      notice = 'Choose a backup path before replacing the current servers.';
      noticeIsError = true;
      return;
    }
    isWorking = true;
    notice = '';
    try {
      const result = await mutate<Schema['ServerImportResultDTO']>(api, '/v1/servers/import', {
        action: 'importTransfer',
        sourcePath: path,
        importKind: 'transfer',
        transferMode: importMode,
        ...(importMode === 'replaceAll' ? { backupPath: backupPath.trim() } : {}),
      });
      const operation = result.operationId
        ? await pollOperation(api, result.operationId)
        : undefined;
      if (operation?.state === 'failed' || operation?.state === 'cancelled') {
        throw new Error(operation.error?.message ?? 'Transfer import did not complete.');
      }
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

  function saveDownload(bytes: Uint8Array, fileName: string): void {
    const buffer = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(buffer).set(bytes);
    const url = URL.createObjectURL(new Blob([buffer], { type: 'application/octet-stream' }));
    const link = document.createElement('a');
    link.href = url;
    link.download = fileName;
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  }
</script>

<Sheet title="Server Transfer" size="md" {onClose}>
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
          <Field bind:value={sourcePath} placeholder="Path to .msctransfer on the agent" />
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
          disabled={isWorking || !sourcePath.trim()}
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
