<script lang="ts">
  // Real port of AddServerWizardView.swift's merged Import step: a server
  // folder/archive is scanned in place, while a Modrinth or CurseForge
  // archive chosen through the modpack action is staged and inspected before
  // the wizard continues. The staged pack is redeemed by the existing create
  // operation after this step.
  import { onDestroy, onMount } from 'svelte';
  import Button from '../../../components/base/Button.svelte';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import { getPlatform } from '../../../platform';
  import type { PickedFile } from '../../../platform/types';
  import type { Schema, ScreenApi } from '../../shared/types';
  import { errorMessage, mutate } from '../../shared/types';
  import { addonPaths } from '../../addons/model';
  import { scanImportSource, type JavaCategory, type JavaFlavor, type WizardDraft } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;
  /** Called after an existing server folder/archive has been scanned. */
  export let onScanned: () => void = () => {};

  let fileInput: HTMLInputElement;
  let isScanning = false;
  let scanError: string | undefined;
  let dropTargeted = false;
  let supportsDrop = false;
  let unsubscribeDrop: (() => void) | undefined;
  let fileFilter = '';

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

  async function inspectModpack(fileName: string, bytes: Uint8Array): Promise<void> {
    if (!api?.upload) throw new Error('Modpack staging needs a connected agent.');
    isScanning = true;
    scanError = undefined;
    try {
      const staged = await api.upload('modpack-archive', bytes);
      const inspection = await mutate<Schema['ModpackInspectionResultDTO']>(
        api,
        addonPaths.inspectPack,
        { stagedUploadId: staged.stagedUploadId },
      );
      const detected = flavorForLoader(inspection.loaderName);
      draft = {
        ...draft,
        serverName: inspection.packName?.trim() || baseName(fileName).replace(/\.[^.]+$/, ''),
        serverType: 'java',
        ...(detected ?? {}),
        stagedModpack: {
          fileName,
          stagedUploadId: staged.stagedUploadId,
          inspection,
        },
        importSourcePath: undefined,
        importIsZip: false,
        importScan: undefined,
        importActiveWorldName: undefined,
      };
    } catch (error) {
      throw error;
    } finally {
      isScanning = false;
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
      const readFile = (await getPlatform()).readFile;
      if (!readFile)
        throw new Error('Reading a dropped file is unavailable in this desktop build.');
      const bytes = await readFile(path);
      await inspectModpack(baseName(path), bytes);
    } catch (error) {
      scanError = errorMessage(error);
    }
  }

  async function handlePickedFile(file: PickedFile): Promise<void> {
    try {
      await inspectModpack(file.name, file.bytes);
    } catch (error) {
      scanError = errorMessage(error);
    }
  }

  function browseBrowserFile(): Promise<PickedFile | null> {
    return new Promise((resolve) => {
      fileInput.addEventListener(
        'change',
        async () => {
          const file = fileInput.files?.[0];
          resolve(
            file ? { name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) } : null,
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
    const picked = await (
      await getPlatform()
    ).pickFile(
      { label: 'Choose a modpack archive', extensions: ['mrpack', 'zip'] },
      browseBrowserFile,
    );
    if (picked) await handlePickedFile(picked);
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

    {#if inspection.format === 'curseforge'}
      <p class="hint warn">
        CurseForge packs need an API key before creation. Save it in MSC Settings → Modpack Imports,
        then return here to retry.
      </p>
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

    <Button variant="secondary" size="sm" onclick={chooseDifferentFile}
      >Choose a different file</Button
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
