<script lang="ts">
  import ProjectDetailSheet from './ProjectDetailSheet.svelte';
  import ContentManualDownloadSheet from './ContentManualDownloadSheet.svelte';
  import { onDestroy } from 'svelte';
  import { openExternal } from '../../platform';
  import CatalogBrowseContent from './CatalogBrowseContent.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { can, mutate } from '../shared/types';
  import { ApiError } from '../../api/client';
  import { pollOperation } from './model';

  export let api: ScreenApi | undefined;
  export let kind: 'mod' | 'plugin';
  export let javaFlavor: string | undefined;
  export let minecraftVersion: string | undefined;
  export let serverId = '';
  export let mode: 'install' | 'stage' = 'install';
  export let onStage:
    | ((item: Schema['CatalogItemDTO'], versionId: string, allowIncompatible: boolean) => void)
    | undefined = undefined;
  export let serverName = '';
  export let onInstalled: () => void = () => {};
  export let permissions: readonly string[] | undefined = undefined;

  const connection = api?.snapshot?.() ?? api;
  let query = '';
  let installSupported = false;
  let mismatchSupported = false;
  let adding = '';
  let installingReleaseId = '';
  let status = '';
  let installed = new Set<string>();
  let installedVersions = new Set<string>();
  let manualSelection:
    | {
        project: Schema['CatalogItemDTO'];
        release: Schema['ProviderCatalogReleaseDTO'];
        review: Schema['ContentInstallReviewDTO'];
      }
    | undefined;
  async function installRelease(
    item: Schema['CatalogItemDTO'],
    release: Schema['ProviderCatalogReleaseDTO'],
  ): Promise<boolean> {
    if (mode === 'stage' && onStage && current()) {
      onStage(item, release.version.id, !compatible(release));
      markInstalled(item, release);
      status = 'Added to the server draft.';
      return true;
    }
    if (!connection || !serverId || !current() || !installSupported)
      throw new Error('Connect to the selected server before installing content.');
    if (installingReleaseId) throw new Error('Another content installation is still in progress.');
    const allowIncompatible = !compatible(release);
    if (allowIncompatible && !mismatchSupported)
      throw new Error('Update this agent to install an incompatible release.');
    const projectId = Number(item.projectId);
    const versionId = Number(release.version.id);
    if (![projectId, versionId].every((id) => Number.isSafeInteger(id) && id > 0))
      throw new Error('The provider returned an invalid project or release identity.');
    installingReleaseId = release.version.id;
    status = 'Resolving required dependencies…';
    try {
      const root = `/v1/servers/${encodeURIComponent(serverId)}/content`;
      const review = await connection.post<Schema['ContentInstallReviewDTO']>(
        `${root}/install-review`,
        {
          provider: 'curseforge',
          projectId,
          versionId,
          ...(allowIncompatible ? { allowIncompatible: true } : {}),
        },
      );
      if (!current()) throw new Error('The selected host or server changed.');
      if (review.files.some((file) => file.manualDownloadRequired)) {
        manualSelection = { project: item, release, review };
        status = 'Manual downloads are required to finish this installation.';
        return false;
      }
      const accepted = await connection.post<{ operationId: string }>(`${root}/install`, {
        reviewId: review.reviewId,
      });
      const operation = await pollOperation(connection, accepted.operationId, (value) => {
        if (!current()) throw new Error('The selected host or server changed.');
        status = value.statusLine ?? 'Installing…';
      });
      if (operation?.state !== 'succeeded')
        throw new Error(
          operation?.error?.message ??
            (operation?.state === 'cancelled'
              ? 'Installation cancelled.'
              : 'Installation did not complete.'),
        );
      if (!current()) throw new Error('The selected host or server changed.');
      markInstalled(item, release);
      status = 'Installed — restart the server to apply.';
      return true;
    } catch (cause) {
      status = '';
      throw cause;
    } finally {
      installingReleaseId = '';
    }
  }
  function markInstalled(
    item: Schema['CatalogItemDTO'],
    release: Schema['ProviderCatalogReleaseDTO'],
  ): void {
    installed = new Set(installed).add(item.projectId);
    installedVersions = new Set(installedVersions).add(release.version.id);
    onInstalled();
  }
  async function add(item: Schema['CatalogItemDTO']): Promise<void> {
    if (!connection || adding || (mode === 'install' && !serverId) || !current()) return;
    adding = item.projectId;
    const request = ++generation;
    error = '';
    try {
      let path = `/v1/catalog/curseforge/projects/${encodeURIComponent(item.projectId)}/versions?kind=${kind}`;
      if (minecraftVersion) path += `&gameVersion=${encodeURIComponent(minecraftVersion)}`;
      if (kind === 'mod' && javaFlavor) path += `&loader=${encodeURIComponent(javaFlavor)}`;
      const response = await connection.get<Schema['ProviderCatalogVersionsDTO']>(path);
      if (!current() || request !== generation) return;
      const candidates = response.versions.filter(
        (value) => value.available && !value.serverPack && compatible(value),
      );
      const release =
        candidates.find((value) => value.version.versionType === 'release') ?? candidates[0];
      if (release) await installRelease(item, release);
      else {
        await open(item);
        if (current())
          error =
            'No compatible release was returned. Select a specific version below to review an installation anyway.';
      }
    } catch (cause) {
      if (current()) failure(cause);
    } finally {
      adding = '';
    }
  }
  function installedSelection(): void {
    if (!manualSelection) return;
    markInstalled(manualSelection.project, manualSelection.release);
  }
  let offset = 0;
  let total = 0;
  let results: Schema['CatalogItemDTO'][] = [];
  let selected: Schema['CatalogItemDTO'] | undefined;
  let busy = false;
  let error = '';
  let needsKey = false;
  let key = '';
  let savingKey = false;
  function compatible(release: Schema['ProviderCatalogReleaseDTO']): boolean {
    return (
      !!minecraftVersion &&
      release.version.gameVersions.includes(minecraftVersion) &&
      (kind === 'plugin' || release.version.loaders.includes(javaFlavor ?? ''))
    );
  }
  async function openConsole(event: MouseEvent): Promise<void> {
    event.preventDefault();
    try {
      await openExternal('https://console.curseforge.com/');
    } catch (cause) {
      failure(cause);
    }
  }
  let supported = false;
  let ready = false;
  let generation = 0;
  let disposed = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const current = () => !disposed && connection?.isCurrent?.() !== false;
  function failure(cause: unknown): void {
    error = cause instanceof Error ? cause.message : 'Could not load CurseForge content.';
    needsKey = cause instanceof ApiError && cause.error.code === 'missing_curseforge_api_key';
  }
  async function search(page = 0): Promise<void> {
    if (!connection || !supported || !current()) return;
    const request = ++generation;
    offset = page;
    busy = true;
    error = '';
    selected = undefined;
    try {
      let path = `/v1/catalog/curseforge/search?q=${encodeURIComponent(query)}&kind=${kind}&offset=${page}`;
      if (minecraftVersion) path += `&gameVersion=${encodeURIComponent(minecraftVersion)}`;
      if (['fabric', 'forge', 'neoforge', 'quilt'].includes(javaFlavor ?? ''))
        path += `&loader=${encodeURIComponent(javaFlavor!)}`;
      const response = await connection.get<Schema['ProviderCatalogSearchDTO']>(path);
      if (request !== generation || !current()) return;
      results = response.results;
      total = response.totalCount;
      needsKey = false;
    } catch (cause) {
      if (request === generation && current()) {
        results = [];
        failure(cause);
      }
    } finally {
      if (request === generation && current()) busy = false;
    }
  }
  function open(item: Schema['CatalogItemDTO']): void {
    ++generation;
    clearTimeout(timer);
    selected = item;
  }
  async function installDetailVersion(
    _version: Schema['CatalogVersionDTO'],
    release: Schema['ProviderCatalogReleaseDTO'] | undefined,
  ): Promise<boolean> {
    if (!selected || !release) throw new Error('The selected release is unavailable.');
    return installRelease(selected, release);
  }
  async function initialize(): Promise<void> {
    if (!connection) {
      error = 'Connect to an agent to browse content.';
      return;
    }
    try {
      const caps = await connection.get<Schema['CapabilitiesDTO']>('/v1/capabilities');
      if (!current()) return;
      supported = caps.contentCatalog?.providers?.includes('curseforge') ?? false;
      installSupported =
        mode === 'stage' ||
        (caps.contentCatalog?.reviewedInstalls?.includes('curseforge') ?? false);
      mismatchSupported =
        mode === 'stage' || (caps.contentCatalog?.incompatibleInstallReview ?? false);
      ready = true;
      if (!supported) error = 'Update this agent to browse CurseForge mods and plugins.';
      else busy = true;
    } catch (cause) {
      if (current()) failure(cause);
    }
  }
  async function saveKey(): Promise<void> {
    if (!connection || !key.trim() || !current()) return;
    savingKey = true;
    error = '';
    try {
      const status = await mutate<Schema['CurseForgeApiKeyStatusDTO']>(
        connection,
        '/v1/config/curseforge',
        { apiKey: key.trim() },
      );
      key = '';
      if (!status.configured) throw new Error('The agent did not save the CurseForge key.');
      needsKey = false;
      if (current()) await search();
    } catch (cause) {
      if (current()) failure(cause);
    } finally {
      key = '';
      savingKey = false;
    }
  }
  $: if (ready && supported) {
    query;
    clearTimeout(timer);
    timer = setTimeout(() => void search(), 350);
  }
  void initialize();
  onDestroy(() => {
    disposed = true;
    ++generation;
    clearTimeout(timer);
    key = '';
  });
</script>

<div class="catalog">
  <CatalogBrowseContent
    bind:query
    noun={kind === 'mod' ? 'mods' : 'plugins'}
    subtitle={`CurseForge · ${[javaFlavor, minecraftVersion].filter(Boolean).join(' · ') || 'All versions'}`}
    {results}
    loading={busy}
    disabled={!!adding}
    canAdd={(mode === 'stage' || !!serverId) && installSupported}
    {installed}
    working={new Set(adding ? [adding] : [])}
    workingLabel={installingReleaseId ? 'Installing…' : 'Checking…'}
    onDetail={(item) => void open(item)}
    onAdd={(item) => void add(item)}
    showEmpty={ready && supported && !error}
    {offset}
    {total}
    onPage={(page) => {
      if (supported) void search(page);
    }}
  >
    {#if status}<p class="context" role="status">{status}</p>{/if}
    {#if error}
      <p class="error" role="alert">{error}</p>
      {#if !needsKey}<Button
          size="sm"
          variant="secondary"
          onclick={() => (supported ? void search(offset) : void initialize())}>Retry</Button
        >{/if}
    {/if}
    {#if needsKey && can(permissions, 'admin')}
      <button
        type="button"
        class="key-link"
        onclick={() => document.getElementById('provider-api-key')?.focus()}
        >CurseForge API key…</button
      >
    {/if}
    {#if needsKey}
      <p class="context">
        CurseForge requires an API key for catalog access. Save it on this agent to browse and
        download supported content.
      </p>
      <a
        class="key-link"
        href="https://console.curseforge.com/"
        target="_blank"
        rel="noopener noreferrer"
        onclick={openConsole}>Open CurseForge console</a
      >
      {#if can(permissions, 'admin')}
        <form
          onsubmit={(event) => {
            event.preventDefault();
            void saveKey();
          }}
        >
          <label for="provider-api-key">API key for this agent</label>
          <input id="provider-api-key" type="password" autocomplete="off" bind:value={key} />
          <Button size="sm" type="submit" disabled={savingKey || !key.trim()}
            >{savingKey ? 'Saving…' : 'Save key'}</Button
          >
        </form>
      {:else}<p class="context">Ask this host's owner to configure its CurseForge key.</p>{/if}
    {/if}
    {#if ready && supported && !installSupported}<p class="context">
        Update this agent to install reviewed CurseForge content.
      </p>{/if}
    {#if ready && installSupported && !mismatchSupported}<p class="context">
        Update this agent to enable Install anyway for incompatible releases.
      </p>{/if}
  </CatalogBrowseContent>
</div>

{#if selected && connection}
  <ProjectDetailSheet
    api={connection}
    item={selected}
    provider="curseforge"
    {mode}
    {javaFlavor}
    serverMinecraftVersion={minecraftVersion}
    installActionsAvailable={installSupported && (mode === 'stage' || !!serverId)}
    incompatibleInstallAvailable={mismatchSupported}
    knownInstalledVersionIds={[...installedVersions]}
    onInstallProviderVersion={installDetailVersion}
    onClose={() => (selected = undefined)}
    onInstalled={() => onInstalled()}
  />
{/if}

{#if manualSelection && connection}
  <ContentManualDownloadSheet
    api={connection}
    {serverId}
    {serverName}
    {kind}
    project={manualSelection.project}
    release={manualSelection.release}
    review={manualSelection.review}
    onClose={() => (manualSelection = undefined)}
    onInstalled={installedSelection}
  />
{/if}

<style>
  .context {
    margin: 0;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .error {
    margin: 0;
    color: var(--msc2-status-error);
    font-size: 12px;
  }
  .key-link {
    width: fit-content;
    background: none;
    border: 0;
    padding: 0;
    color: var(--msc2-text-secondary);
    font: inherit;
    font-size: 11px;
    text-decoration: underline;
    cursor: pointer;
  }
  form {
    display: grid;
    gap: 8px;
  }
  label {
    font-size: 12px;
  }
  input {
    background: var(--msc2-tier-content);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 6px;
    padding: 8px;
    color: var(--msc2-text-primary);
    font: inherit;
  }
</style>
