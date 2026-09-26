<script lang="ts">
  // Real port of AddServerWizardView.swift's merged Import step: a server
  // folder/archive is scanned in place, while a Modrinth or CurseForge
  // archive chosen through the modpack action is staged and inspected before
  // the wizard continues. The staged pack is redeemed by the existing create
  // operation after this step.
  import { onDestroy, onMount, tick } from 'svelte';
  import Button from '../../../components/base/Button.svelte';
  import Field from '../../../components/base/Field.svelte';
  import Sheet from '../../../components/base/Sheet.svelte';
  import VisibilityIcon from '../../../components/base/VisibilityIcon.svelte';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import { getPlatform, openExternal } from '../../../platform';
  import type { FileChunkSource, FileUploadProgress } from '../../../platform/types';
  import type { Schema, ScreenApi } from '../../shared/types';
  import { errorMessage, mutate } from '../../shared/types';
  import { addonPaths } from '../../addons/model';
  import ModpackUploadProgress from '../../components/ModpackUploadProgress.svelte';
  import { scanImportSource, type JavaCategory, type JavaFlavor, type WizardDraft } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;
  /** Called after an existing server folder/archive has been scanned. */
  export let onScanned: () => void = () => {};

  let fileInput: HTMLInputElement;
  let isScanning = false;
  let modpackProgress: FileUploadProgress | undefined;
  let modpackFileName = '';
  let scanError: string | undefined;
  let dropTargeted = false;
  let supportsDrop = false;
  let unsubscribeDrop: (() => void) | undefined;
  let fileFilter = '';
  let curseforgeApiKey = '';
  let curseforgeApiKeyVisible = false;
  let curseforgeKeySaving = false;
  let curseforgeKeyNotice = '';

  const curseforgeApiConsoleUrl = 'https://console.curseforge.com/';

  onMount(async () => {
    const platform = await getPlatform();
    supportsDrop = platform.kind === 'tauri';
    unsubscribeDrop = await platform.onFileDrop((paths) => {
      dropTargeted = false;
      const first = paths[0];
      if (first) void handlePath(first);
    });
  });

  onDestroy(() => unsubscribeDrop?.());

  function baseName(path: string): string {
    return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
  }

  async function openCurseForgeConsole(event: MouseEvent): Promise<void> {
    event.preventDefault();
    try {
      await openExternal(curseforgeApiConsoleUrl);
    } catch (error) {
      curseforgeKeyNotice =
        errorMessage(error) || 'The CurseForge API Console could not be opened.';
    }
  }

  function flavorForLoader(
    loaderName: string | undefined,
  ): { javaCategory: JavaCategory; javaFlavor: JavaFlavor } | undefined {
    const loader = loaderName?.toLowerCase().replace(/[^a-z]/g, '');
    if (loader?.includes('neoforge')) return { javaCategory: 'modded', javaFlavor: 'neoforge' };
    if (loader?.includes('forge')) return { javaCategory: 'modded', javaFlavor: 'forge' };
    if (loader?.includes('fabric')) return { javaCategory: 'modded', javaFlavor: 'fabric' };
    if (loader?.includes('purpur')) return { javaCategory: 'standard', javaFlavor: 'purpur' };
    if (loader?.includes('paper')) return { javaCategory: 'standard', javaFlavor: 'paper' };
    if (loader?.includes('vanilla')) return { javaCategory: 'standard', javaFlavor: 'vanilla' };
    return undefined;
  }

  async function scanServerPath(path: string, isZip: boolean): Promise<void> {
    isScanning = true;
    scanError = undefined;
    draft = {
      ...draft,
      stagedModpack: undefined,
      importSourcePath: undefined,
      importIsZip: false,
      importScan: undefined,
    };
    try {
      const scan = await scanImportSource(api, path, isZip);
      draft = {
        ...draft,
        importSourcePath: path,
        importIsZip: isZip,
        importScan: scan,
        serverType: scan.serverType === 'bedrock' ? 'bedrock' : 'java',
        ...(scan.serverType === 'bedrock'
          ? scan.port === undefined
            ? {}
            : { bedrockPort: scan.port }
          : scan.port === undefined
            ? {}
            : { javaPort: scan.port }),
        importMaxPlayers: scan.maxPlayers ?? draft.importMaxPlayers,
        importEulaAccepted: scan.eulaAccepted ?? false,
        importActiveWorldName: scan.defaultWorldName,
      };
      onScanned();
    } catch (error) {
      scanError = errorMessage(error);
    } finally {
      isScanning = false;
    }
  }

  async function inspectModpack(fileName: string, source: FileChunkSource): Promise<void> {
    isScanning = true;
    scanError = undefined;
    curseforgeKeyNotice = '';
    modpackFileName = fileName;
    modpackProgress = {
      phase: 'preparing',
      bytesUploaded: 0,
      totalBytes: source.size,
    };
    try {
      if (!api?.uploadFile) throw new Error('Modpack staging needs a connected agent.');
      await tick();
      const staged = await api.uploadFile('modpack-archive', source, {
        onProgress: (progress) => (modpackProgress = progress),
      });
      modpackProgress = {
        phase: 'complete',
        bytesUploaded: source.size,
        totalBytes: source.size,
      };
      await inspectStagedModpack(fileName, staged.stagedUploadId);
    } finally {
      try {
        await source.close();
      } finally {
        modpackProgress = undefined;
        isScanning = false;
      }
    }
  }

  async function inspectStagedModpack(fileName: string, stagedUploadId: string): Promise<void> {
    if (!api) throw new Error('Modpack inspection needs a connected agent.');
    const inspection = await mutate<Schema['ModpackInspectionResultDTO']>(
      api,
      addonPaths.inspectPack,
      { stagedUploadId },
    );
    const detected = flavorForLoader(inspection.loaderName);
    draft = {
      ...draft,
      serverName: inspection.packName?.trim() || baseName(fileName).replace(/\.[^.]+$/, ''),
      serverType: 'java',
      ...(detected ?? {}),
      stagedModpack: {
        fileName,
        stagedUploadId,
        inspection,
      },
      importSourcePath: undefined,
      importIsZip: false,
      importScan: undefined,
      importActiveWorldName: undefined,
    };
  }

  async function saveCurseForgeKey(): Promise<void> {
    if (!api || !curseforgeApiKey.trim() || curseforgeKeySaving || !draft.stagedModpack) return;
    curseforgeKeySaving = true;
    curseforgeKeyNotice = '';
    try {
      const status = await mutate<Schema['CurseForgeApiKeyStatusDTO']>(
        api,
        '/v1/config/curseforge',
        { apiKey: curseforgeApiKey.trim() },
      );
      if (!status.configured) {
        curseforgeKeyNotice = 'MSC did not save a CurseForge API key.';
        return;
      }
      curseforgeApiKey = '';
      curseforgeKeyNotice = 'Key saved. Checking CurseForge files…';
      await inspectStagedModpack(draft.stagedModpack.fileName, draft.stagedModpack.stagedUploadId);
    } catch (error) {
      curseforgeKeyNotice = errorMessage(error) || 'The CurseForge API key could not be saved.';
    } finally {
      curseforgeKeySaving = false;
    }
  }

  async function handlePath(path: string): Promise<void> {
    const lower = path.toLowerCase();
    if (lower.endsWith('.zip')) {
      // Server archives can contain hundreds of megabytes of mods and
      // libraries. Inspect them through the agent's path-based scan instead
      // of first loading and uploading the entire archive as a modpack.
      await scanServerPath(path, true);
      return;
    }
    if (!lower.endsWith('.mrpack')) {
      await scanServerPath(path, false);
      return;
    }

    try {
      const readFileStream = (await getPlatform()).readFileStream;
      if (!readFileStream)
        throw new Error('Reading a dropped file is unavailable in this desktop build.');
      await inspectModpack(baseName(path), await readFileStream(path));
    } catch (error) {
      scanError = errorMessage(error);
    }
  }

  function browseBrowserFile(): Promise<FileChunkSource | null> {
    return new Promise((resolve) => {
      fileInput.addEventListener(
        'change',
        async () => {
          const file = fileInput.files?.[0];
          resolve(
            file
              ? {
                  name: file.name,
                  size: file.size,
                  readChunk: async (offset, maxBytes) =>
                    new Uint8Array(await file.slice(offset, offset + maxBytes).arrayBuffer()),
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

  async function browseFolder(): Promise<void> {
    const path = await (await getPlatform()).pickFolder('Choose Server Folder');
    if (path) await scanServerPath(path, false);
  }

  async function browseServerArchive(): Promise<void> {
    const path = await (
      await getPlatform()
    ).pickFilePath({ label: 'Choose Server .zip', extensions: ['zip'] });
    if (path) await scanServerPath(path, true);
  }

  async function browseModpack(): Promise<void> {
    isScanning = true;
    scanError = undefined;
    modpackFileName = 'Waiting for file selection…';
    modpackProgress = { phase: 'selecting', bytesUploaded: 0, totalBytes: 0 };
    let picked: FileChunkSource | null = null;
    try {
      picked = await (
        await getPlatform()
      ).pickFileStream(
        { label: 'Choose a modpack archive', extensions: ['mrpack', 'zip'] },
        browseBrowserFile,
      );
      if (picked) await inspectModpack(picked.name, picked);
    } catch (error) {
      scanError = errorMessage(error);
    } finally {
      if (!picked) {
        modpackProgress = undefined;
        isScanning = false;
      }
    }
  }

  function chooseDifferentFile(): void {
    draft = {
      ...draft,
      serverName: '',
      stagedModpack: undefined,
      importSourcePath: undefined,
      importIsZip: false,
      importScan: undefined,
    };
    scanError = undefined;
    curseforgeApiKey = '';
    curseforgeKeyNotice = '';
  }
</script>

<div class="upload" use:onboardingAnchor={'ob_wizard_body'}>
  <input bind:this={fileInput} type="file" accept=".mrpack,.zip" class="hidden-input" />

  {#if draft.stagedModpack}
    {@const inspection = draft.stagedModpack.inspection}
    <div class="intro">
      <h2>Modpack detected</h2>
      <p>{inspection.message}</p>
    </div>

    <div class="summary">
      <div class="row">
        <span class="label">Pack</span>
        <span class="value">{inspection.packName ?? draft.stagedModpack.fileName}</span>
      </div>
      {#if inspection.packVersion}
        <div class="row">
          <span class="label">Version</span>
          <span class="value">{inspection.packVersion}</span>
        </div>
      {/if}
      <div class="row">
        <span class="label">Software</span>
        <span class="value"
          >{inspection.loaderName ?? 'Java'}{inspection.loaderVersion
            ? ` · ${inspection.loaderVersion}`
            : ''}</span
        >
      </div>
      {#if inspection.minecraftVersion}
        <div class="row">
          <span class="label">Minecraft</span>
          <span class="value">{inspection.minecraftVersion}</span>
        </div>
      {/if}
      <div class="row">
        <span class="label">Manifest files</span>
        <span class="value">{inspection.fileCount}</span>
      </div>
      <div class="row">
        <span class="label">Installed on server</span>
        <span class="value"
          >{inspection.fileCount - (inspection.clientOnlyFileCount ?? 0)} manifest files</span
        >
      </div>
      {#if inspection.clientOnlyFileCount}
        <div class="row">
          <span class="label">Client-only files</span>
          <span class="value">{inspection.clientOnlyFileCount} skipped</span>
        </div>
      {/if}
      {#if inspection.overrideFileCount}
        <div class="row">
          <span class="label">Included overrides</span>
          <span class="value">{inspection.overrideFileCount} files</span>
        </div>
      {/if}
    </div>

    {#if inspection.warnings?.length}
      <ul class="warnings">
        {#each inspection.warnings as warning}
          <li>{warning}</li>
        {/each}
      </ul>
    {/if}

    {#if inspection.format === 'curseforge' && inspection.curseforgeLookupAvailable === false}
      <div class="curseforge-setup">
        <p class="hint warn">
          MSC could not check this pack's CurseForge files. Enter or update the API key below and
          MSC will check them again. You can manage this key later in MSC Settings → Modpack
          Imports.
        </p>
        <p class="key-explain">
          Create or manage the key in the
          <a
            href={curseforgeApiConsoleUrl}
            target="_blank"
            rel="noreferrer"
            onclick={(event) => void openCurseForgeConsole(event)}>CurseForge API Console</a
          >. The key is saved securely on this agent and is never shown again.
        </p>
        <div class="key-control">
          <Field
            bind:value={curseforgeApiKey}
            type={curseforgeApiKeyVisible ? 'text' : 'password'}
            placeholder="Paste API key"
            width="100%"
            disabled={curseforgeKeySaving}
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
            >{curseforgeKeySaving ? 'Checking…' : 'Save key and retry'}</Button
          >
        </div>
      </div>
    {:else if inspection.format === 'mrpack'}
      <p class="hint">
        This Modrinth pack downloads from its manifest; no CurseForge API key is needed.
      </p>
    {/if}

    {#if inspection.files?.length}
      <details class="disclosure contents">
        <summary>View pack contents ({inspection.files.length})</summary>
        <input
          class="file-filter"
          type="search"
          bind:value={fileFilter}
          placeholder="Filter files"
          aria-label="Filter modpack files"
        />
        <div class="file-list">
          {#each inspection.files.filter((file) => file.path
              .toLowerCase()
              .includes(fileFilter.trim().toLowerCase())) as file (file.path)}
            <div class="file-row" class:client-only={file.clientOnly}>
              <span>{file.path}</span>
              {#if file.clientOnly}<span class="file-note">client-only</span>{/if}
            </div>
          {/each}
        </div>
      </details>
    {/if}

    <Button
      variant="secondary"
      size="sm"
      disabled={curseforgeKeySaving}
      onclick={chooseDifferentFile}>Choose a different file</Button
    >
  {:else if isScanning}
    <div class="status">
      <span class="spinner" aria-hidden="true"></span>
      <span class="hint">Inspecting archive or scanning server folder…</span>
    </div>
  {:else if scanError}
    <div class="status column">
      <p class="hint warn">{scanError}</p>
      <Button variant="secondary" size="sm" onclick={chooseDifferentFile}>Try Again</Button>
    </div>
  {:else}
    <div class="intro">
      <h2>Drop your server folder, archive, or modpack</h2>
      <p>
        Drop a server folder or server .zip to import an existing server. For a .mrpack or
        CurseForge .zip modpack, use Choose Modpack below.
      </p>
    </div>

    <div
      class="dropzone"
      class:targeted={dropTargeted}
      role="group"
      aria-label="Drop a server folder, archive, or modpack"
      ondragover={(event) => event.preventDefault()}
      ondragenter={() => (dropTargeted = true)}
      ondragleave={() => (dropTargeted = false)}
      ondrop={(event) => {
        event.preventDefault();
        dropTargeted = false;
      }}
    >
      <p class="dropzone-title">
        {supportsDrop
          ? 'Drop server folder or server .zip here'
          : 'Browse for a server folder, archive, or modpack'}
      </p>
      {#if !supportsDrop}
        <p class="hint">Dragging a file in isn't available in the browser — use Browse below.</p>
      {/if}
      <div class="actions">
        <Button variant="secondary" onclick={() => void browseFolder()}>Choose Folder…</Button>
        <Button variant="secondary" onclick={() => void browseServerArchive()}
          >Choose Server .zip…</Button
        >
        <Button variant="secondary" onclick={() => void browseModpack()}>Choose Modpack…</Button>
      </div>
    </div>
  {/if}
</div>

{#if modpackProgress}
  <Sheet title="Staging modpack" size="sm">
    <ModpackUploadProgress fileName={modpackFileName} progress={modpackProgress} />
  </Sheet>
{/if}

<style>
  .upload {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .intro {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .intro h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .intro p {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }

  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 44px 20px;
    background: var(--msc2-tier-chrome);
    border: 1.5px dashed var(--msc2-hairline-subtle);
    border-radius: 12px;
  }
  .dropzone.targeted {
    border-color: rgba(255, 255, 255, 0.4);
    background: rgba(255, 255, 255, 0.05);
  }
  .dropzone-title {
    margin: 0;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--msc2-text-primary);
    text-align: center;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 10px;
  }

  .summary {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    background: var(--msc2-tier-chrome);
    border-radius: 10px;
  }
  .curseforge-setup {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
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
  .key-notice {
    color: var(--msc2-status-warn);
  }
  .key-control {
    position: relative;
  }
  .key-control :global(.field) {
    padding-right: 42px;
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
  .visibility-toggle:hover {
    color: var(--msc2-text-primary);
  }
  .visibility-toggle:focus-visible {
    outline: 2px solid var(--msc2-hairline);
    outline-offset: 2px;
    border-radius: 3px;
  }
  .key-footer {
    display: flex;
    justify-content: flex-end;
  }
  .contents {
    gap: 10px;
  }
  .file-filter {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 10px;
    color: var(--msc2-text-primary);
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 7px;
    font: inherit;
    font-size: 11.5px;
  }
  .file-list {
    display: flex;
    flex-direction: column;
    max-height: 180px;
    overflow: auto;
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 7px;
  }
  .file-row {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 6px 9px;
    border-top: 1px solid var(--msc2-hairline-subtle);
    font-size: 10.5px;
    color: var(--msc2-text-secondary);
  }
  .file-row:first-child {
    border-top: none;
  }
  .file-row.client-only {
    color: var(--msc2-text-tertiary);
  }
  .file-note {
    flex-shrink: 0;
    color: var(--msc2-status-warn);
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 16px;
  }
  .label {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .value {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-primary);
    text-align: right;
  }
  .warnings {
    margin: 0;
    padding-left: 18px;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-status-warn);
  }
  .disclosure {
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .disclosure summary {
    cursor: pointer;
    color: var(--msc2-text-primary);
  }

  .status {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 44px 0;
  }
  .status.column {
    flex-direction: column;
    gap: 10px;
  }
  .spinner {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    border: 2px solid var(--msc2-hairline-subtle);
    border-top-color: var(--msc2-text-secondary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
    text-align: center;
  }
  .hint.warn {
    color: var(--msc2-status-warn);
  }
  .hidden-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    overflow: hidden;
  }
</style>
