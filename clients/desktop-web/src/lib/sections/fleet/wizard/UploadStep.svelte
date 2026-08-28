<script lang="ts">
  // Real port of AddServerWizardView.swift's step2ImportUpload, existing-
  // server branch only -- modpack detection is the merged step-2 variant
  // P12.18i owns, per this step's own plan text: "Port the Import path's
  // Upload step ... calling POST /v1/servers/import with action: 'scan'".
  // Drop zone plus Browse for a folder or .zip, either way a real local
  // path: `sourcePath` names a path on the *agent's own host filesystem* to
  // scan on disk, not bytes to upload (confirmed against `perform_raw_scan`,
  // crates/msc-agent/src/routes/servers.rs). Browse already works on both
  // platforms via the established `PlatformAdapter.pickFolder`/
  // `pickFilePath` vocabulary -- a real native picker on Tauri, a typed-path
  // `window.prompt` on browser, the same "ask for a path" fallback
  // `platform/browser.ts` already uses for `JavaTab.svelte`'s Java-
  // executable field.
  //
  // Drag-and-drop needed a new platform primitive: Tauri's webview drag-drop
  // carries real absolute paths (`@tauri-apps/api/webview`'s
  // `onDragDropEvent`), but a browser's HTML5 drop event never exposes a
  // real filesystem path at all (the same reason `PickedFile` returns bytes,
  // not a path). Added `PlatformAdapter.onFileDrop` (platform/types.ts,
  // tauri.ts, browser.ts) rather than reaching into `@tauri-apps/api`
  // directly here -- no section or component in this codebase imports Tauri
  // APIs itself; they all go through `getPlatform()`. The browser adapter's
  // implementation never calls the handler, so this drop zone shows a
  // "use Browse" hint there instead of an inert target; a plain
  // `ondrop`/`ondragover` pair still exists for the hover highlight and to
  // stop the browser navigating to the dropped file, but never reads
  // `dataTransfer` for a path (there isn't a real one to read).
  import { onDestroy, onMount } from 'svelte';
  import Button from '../../../components/base/Button.svelte';
  import { getPlatform } from '../../../platform';
  import type { ScreenApi } from '../../shared/types';
  import { errorMessage } from '../../shared/types';
  import { scanImportSource, type WizardDraft } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;

  let isScanning = false;
  let scanError: string | undefined;
  let dropTargeted = false;
  let supportsDrop = false;
  let unsubscribeDrop: (() => void) | undefined;

  onMount(async () => {
    const platform = await getPlatform();
    supportsDrop = platform.kind === 'tauri';
    unsubscribeDrop = await platform.onFileDrop((paths) => {
      dropTargeted = false;
      const first = paths[0];
      if (first) void handleSource(first);
    });
  });

  onDestroy(() => unsubscribeDrop?.());

  async function handleSource(path: string): Promise<void> {
    const isZip = path.toLowerCase().endsWith('.zip');
    isScanning = true;
    scanError = undefined;
    draft.importScan = undefined;
    try {
      const scan = await scanImportSource(api, path, isZip);
      draft.importSourcePath = path;
      draft.importIsZip = isZip;
      draft.importScan = scan;
      draft.serverType = scan.serverType === 'bedrock' ? 'bedrock' : 'java';
      if (draft.serverType === 'bedrock') {
        if (scan.port !== undefined) draft.bedrockPort = scan.port;
      } else if (scan.port !== undefined) {
        draft.javaPort = scan.port;
      }
      draft.importMaxPlayers = scan.maxPlayers ?? draft.importMaxPlayers;
      draft.importEulaAccepted = scan.eulaAccepted ?? false;
      draft.importActiveWorldName = scan.defaultWorldName;
    } catch (error) {
      scanError = errorMessage(error);
    } finally {
      isScanning = false;
    }
  }

  async function browseFolder(): Promise<void> {
    const path = await (await getPlatform()).pickFolder('Choose Server Folder');
    if (path) void handleSource(path);
  }

  async function browseZip(): Promise<void> {
    const path = await (
      await getPlatform()
    ).pickFilePath({ label: 'Choose Server .zip', extensions: ['zip'] });
    if (path) void handleSource(path);
  }

  function tryAgain(): void {
    scanError = undefined;
  }
</script>

<div class="upload">
  <div class="intro">
    <h2>Drop your server folder or archive</h2>
    <p>Drop a server folder or .zip to import an existing server.</p>
  </div>

  {#if isScanning}
    <div class="status">
      <span class="spinner" aria-hidden="true"></span>
      <span class="hint">Scanning server folder…</span>
    </div>
  {:else if scanError}
    <div class="status column">
      <p class="hint warn">{scanError}</p>
      <Button variant="secondary" size="sm" onclick={tryAgain}>Try Again</Button>
    </div>
  {:else}
    <div
      class="dropzone"
      class:targeted={dropTargeted}
      role="group"
      aria-label="Drop a server folder or archive"
      ondragover={(event) => event.preventDefault()}
      ondragenter={() => (dropTargeted = true)}
      ondragleave={() => (dropTargeted = false)}
      ondrop={(event) => {
        event.preventDefault();
        dropTargeted = false;
      }}
    >
      <p class="dropzone-title">
        {supportsDrop ? 'Drop server folder or .zip here' : 'Browse for a server folder or .zip'}
      </p>
      {#if !supportsDrop}
        <p class="hint">Dragging a file in isn't available in the browser — use Browse below.</p>
      {/if}
      <div class="actions">
        <Button variant="secondary" onclick={() => void browseFolder()}>Choose Folder…</Button>
        <Button variant="secondary" onclick={() => void browseZip()}>Choose .zip…</Button>
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
  }

  .actions {
    display: flex;
    gap: 10px;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 44px 0;
    justify-content: center;
  }
  .status.column {
    flex-direction: column;
    gap: 10px;
  }

  .spinner {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    border-radius: 50%;
    border: 2px solid var(--msc2-hairline-subtle);
    border-top-color: var(--msc2-text-secondary);
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
</style>
