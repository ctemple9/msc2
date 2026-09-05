<script lang="ts">
  // Ports PreferencesJavaSection (MSCSettingsSections.swift:4-108) --
  // executable path, extra JVM flags, Detect
  // (JavaRuntimeManager.detectInstalledJavaRuntimes ->
  // JavaRuntimePickerSheet/-Row), Install Java... (JavaInstaller's
  // Minecraft-framed major picker -> Adoptium Temurin download) -- as a
  // Server Editor tab per the 2026-08-27 "Java tab decision" recorded at
  // the top of rolling-plan.md: the value (`AppConfig.javaPath`/
  // `extraFlags`) is genuinely host-wide, edited here only because a Java
  // server is the moment it's actually relevant. Per Cameron's 2026-08-27
  // review, the tab carries no host-wide-warning banner -- the path and
  // flags sections speak for themselves.
  //
  // MSC 1's installer downloads a macOS .pkg for a human to double-click.
  // MSC 2 installs Java itself (D-006's P7.1 addendum) via
  // POST /v1/java-runtimes/install{major}, which always returns an
  // operationId -- polled the same way VersionPickerSheet.svelte polls a
  // version change. The four offered majors (25/21/17/8) are kept in the
  // shared JavaInstallSheet so the same installer surface can be used during
  // server creation recovery.
  //
  // The executable path and extra-flags fields save independently (each
  // its own Field + Save), so `POST /v1/config/java-runtime` had to stop
  // unconditionally applying `executablePath` (previously defaulting to
  // "java" when absent) -- see that route's own comment.
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

  // SetupIntro.svelte's probeJava/browseJava treat the bare 'java' sentinel
  // (PATH lookup) as an empty executablePath on the wire -- mirrored here so
  // this field and the setup wizard's field behave identically.
  function normalizeForSave(path: string): string {
    const trimmed = path.trim();
    return trimmed === 'java' ? '' : trimmed;
  }

  async function loadConfig(): Promise<void> {
    const config = await call<Schema['JavaConfigResponseDTO']>(
      api,
      {},
      serverEditorPaths.javaConfig,
    );
    currentPath = config.executablePath?.trim() || 'java';
    pathDraft = currentPath;
    currentFlags = config.extraFlags?.trim() ?? '';
    flagsDraft = currentFlags;
  }

  async function saveExecutablePath(path: string, successMessage: string): Promise<boolean> {
    try {
      const result = await mutate<Schema['JavaConfigResponseDTO']>(
        api,
        serverEditorPaths.javaConfig,
        { executablePath: normalizeForSave(path) },
      );
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
      const result = await mutate<Schema['JavaConfigResponseDTO']>(
        api,
        serverEditorPaths.javaConfig,
        { extraFlags: flagsDraft.trim() },
      );
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
    <p class="hint">Java servers require JDK 21 or later.</p>
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
      Optional flags passed to Java when starting servers. Leave blank unless you know what you're
      doing (e.g. GC tuning flags).
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
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
</style>
