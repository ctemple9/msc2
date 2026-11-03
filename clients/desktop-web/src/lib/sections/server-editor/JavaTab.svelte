<script lang="ts">
  // The editor targets this server explicitly, including when it is not
  // active. Older servers inherit the host default until a path is saved.
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import JavaInstallSheet from './JavaInstallSheet.svelte';
  import { getPlatform } from '../../platform';
  import type { PlatformKind } from '../../platform/types';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let canControl = true;
  export let serverId: string;

  $: javaConfigPath = `${serverEditorPaths.javaConfig}?serverId=${encodeURIComponent(serverId)}`;

  let platformKind: PlatformKind | null = null;
  let currentPath = 'java';
  let pathDraft = 'java';
  let saving = false;
  let detecting = false;
  let browsing = false;
  let notice = '';

  let currentFlags = '';
  let flagsDraft = '';
  let savingFlags = false;

  let showDetectPicker = false;
  let detectedRuntimes: Schema['JavaRuntimeDTO'][] = [];
  let detectError = '';

  let showInstallPicker = false;

  $: pathDirty = pathDraft.trim() !== currentPath;
  $: flagsDirty = flagsDraft.trim() !== currentFlags;

  onMount(async () => {
    void getPlatform().then((platform) => (platformKind = platform.kind));
    await loadConfig();
  });

  // Keep PATH selection explicit; an empty field restores the host default.
  function normalizeForSave(path: string): string {
    const trimmed = path.trim();
    return trimmed;
  }

  async function loadConfig(): Promise<void> {
    const config = await call<Schema['JavaConfigResponseDTO']>(api, {}, javaConfigPath);
    currentPath = config.executablePath?.trim() || 'java';
    pathDraft = currentPath;
    currentFlags = config.extraFlags?.trim() ?? '';
    flagsDraft = currentFlags;
  }

  async function saveExecutablePath(path: string, successMessage: string): Promise<boolean> {
    try {
      const result = await mutate<Schema['JavaConfigResponseDTO']>(api, javaConfigPath, {
        executablePath: normalizeForSave(path),
      });
      currentPath = result.executablePath?.trim() || 'java';
      pathDraft = currentPath;
      notice = successMessage;
      return true;
    } catch (error) {
      notice = errorMessage(error);
      return false;
    }
  }

  async function saveManualPath(): Promise<void> {
    if (!pathDirty || saving) return;
    saving = true;
    await saveExecutablePath(pathDraft, 'Java executable path saved.');
    saving = false;
  }

  async function saveFlags(): Promise<void> {
    if (!flagsDirty || savingFlags) return;
    savingFlags = true;
    try {
      const result = await mutate<Schema['JavaConfigResponseDTO']>(api, javaConfigPath, {
        extraFlags: flagsDraft.trim(),
      });
      currentFlags = result.extraFlags?.trim() ?? '';
      flagsDraft = currentFlags;
      notice = 'Java arguments saved.';
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      savingFlags = false;
    }
  }

  async function openDetectPicker(): Promise<void> {
    if (detecting || !api) return;
    detecting = true;
    detectError = '';
    try {
      const response = await api.get<Schema['JavaRuntimesResponseDTO']>(
        serverEditorPaths.javaRuntimes,
      );
      detectedRuntimes = response.runtimes ?? [];
      showDetectPicker = true;
    } catch (error) {
      detectError = errorMessage(error);
      showDetectPicker = true;
    } finally {
      detecting = false;
    }
  }

  async function selectDetected(runtime: Schema['JavaRuntimeDTO']): Promise<void> {
    const ok = await saveExecutablePath(runtime.executablePath, `Using ${runtime.name}.`);
    if (ok) showDetectPicker = false;
  }

  async function browseForPath(): Promise<void> {
    if (browsing) return;
    browsing = true;
    try {
      const chosen = await (await getPlatform()).pickFilePath({ label: 'Choose Java executable' });
      if (chosen) await saveExecutablePath(chosen, 'Java executable path saved.');
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      browsing = false;
    }
  }

  async function handleJavaInstalled(event: {
    major: number;
    runtimePath?: string;
  }): Promise<void> {
    if (!event.runtimePath) {
      notice = `Java ${event.major} installed. Click Detect to select it as this host's executable.`;
      return;
    }
    const ok = await saveExecutablePath(
      event.runtimePath,
      `Java ${event.major} installed and selected for this host.`,
    );
    if (!ok) throw new Error(notice);
  }

  function closeInstallPicker(): void {
    showInstallPicker = false;
  }
</script>

<div class="tab">
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  <section class="zone">
    <p class="msc2-type-overline">Java Executable</p>
    <Card padding="0">
      <div class="row">
        <span class="name">Executable Path</span>
        <div class="control">
          <Field bind:value={pathDraft} width="240px" placeholder="java" />
          <Button
            variant="secondary"
            size="sm"
            disabled={!pathDirty || saving || !canControl}
            onclick={saveManualPath}>{saving ? 'Saving…' : 'Save'}</Button
          >
        </div>
      </div>
    </Card>
    <div class="button-row">
      <Button
        variant="secondary"
        size="sm"
        disabled={detecting || !canControl}
        onclick={openDetectPicker}>{detecting ? 'Detecting…' : 'Detect'}</Button
      >
      {#if platformKind === 'tauri'}
        <Button
          variant="secondary"
          size="sm"
          disabled={browsing || !canControl}
          onclick={browseForPath}>{browsing ? 'Choosing…' : 'Browse…'}</Button
        >
      {/if}
      <Button
        variant="secondary"
        size="sm"
        disabled={!canControl}
        onclick={() => (showInstallPicker = true)}>Install Java…</Button
      >
    </div>
    <p class="hint">
      Used by this server. Leave the path blank to use the host default, or enter java to use PATH.
      The required Java version depends on the Minecraft version.
    </p>
  </section>

  <section class="zone">
    <p class="msc2-type-overline">Java Arguments</p>
    <Card padding="0">
      <div class="row">
        <span class="name">Extra JVM Flags</span>
        <div class="control">
          <Field bind:value={flagsDraft} width="240px" multiline placeholder="e.g. -XX:+UseG1GC" />
          <Button
            variant="secondary"
            size="sm"
            disabled={!flagsDirty || savingFlags || !canControl}
            onclick={saveFlags}>{savingFlags ? 'Saving…' : 'Save'}</Button
          >
        </div>
      </div>
    </Card>
    <p class="hint">
      Optional flags passed to Java when starting this server. Leave blank unless you know what
      you're doing (e.g. GC tuning flags).
    </p>
  </section>
</div>

{#if showDetectPicker}
  <Sheet title="Detected Java Runtimes" size="sm" onClose={() => (showDetectPicker = false)}>
    {#if detectError}
      <p class="explain warn">{detectError}</p>
      <div class="footer">
        <Button variant="secondary" onclick={() => (showDetectPicker = false)}>Close</Button>
      </div>
    {:else if detectedRuntimes.length === 0}
      <EmptyState
        title="No Java runtimes found"
        message="MSC checked the common install locations on this host. You can still paste a path manually."
      />
      <div class="footer">
        <Button variant="secondary" onclick={() => (showDetectPicker = false)}>Close</Button>
      </div>
    {:else}
      <div class="list" role="listbox" aria-label="Detected Java runtimes">
        {#each detectedRuntimes as runtime (runtime.executablePath)}
          <button
            type="button"
            class="row runtime-row"
            class:selected={currentPath === runtime.executablePath}
            disabled={!canControl}
            onclick={() => void selectDetected(runtime)}
          >
            <span class="runtime-info">
              <span class="runtime-heading">
                {#if runtime.majorVersion}<span class="runtime-version"
                    >Java {runtime.majorVersion}</span
                  >{/if}
                <span class="runtime-name">{runtime.name}</span>
              </span>
              <span class="runtime-path">{runtime.executablePath}</span>
            </span>
            <span class="tag">Use</span>
          </button>
        {/each}
      </div>
      <div class="footer">
        <Button variant="secondary" onclick={() => (showDetectPicker = false)}>Close</Button>
      </div>
    {/if}
  </Sheet>
{/if}

{#if showInstallPicker}
  <JavaInstallSheet
    {api}
    {canControl}
    onClose={closeInstallPicker}
    onInstalled={handleJavaInstalled}
  />
{/if}

<style>
  .tab {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .notice {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
  }
  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .button-row {
    display: flex;
    gap: 8px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 280px;
    overflow-y: auto;
  }
  .runtime-row {
    width: 100%;
    padding: 11px 14px;
    background: var(--msc2-tier-chrome);
    border: 1px solid transparent;
    border-radius: 8px;
    color: var(--msc2-text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .runtime-row:hover,
  .runtime-row.selected {
    border-color: rgba(255, 255, 255, 0.28);
  }
  .runtime-row:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .runtime-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .runtime-heading {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .runtime-version {
    font-size: 13px;
    font-weight: 500;
  }
  .runtime-name,
  .runtime-path {
    overflow: hidden;
    color: var(--msc2-text-tertiary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .runtime-name {
    font-size: 11px;
  }
  .runtime-path {
    font-family: var(--msc2-font-mono, monospace);
    font-size: 10.5px;
  }
  .tag {
    flex-shrink: 0;
    color: var(--msc2-text-tertiary);
    font-size: 10px;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
  .explain {
    margin: 0 0 12px;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.5;
  }
  .explain.warn {
    color: var(--msc2-status-warn);
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
</style>
