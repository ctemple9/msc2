<script lang="ts">
  import ProjectDetailSheet from './ProjectDetailSheet.svelte';
  import ContentManualDownloadSheet from './ContentManualDownloadSheet.svelte';
  import { onDestroy } from 'svelte';
  import { openExternal } from '../../platform';
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { can, mutate } from '../shared/types';
  import { ApiError } from '../../api/client';
  import { formatCount, pollOperation } from './model';

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
  <Field bind:value={query} placeholder={`Search ${kind === 'mod' ? 'mods' : 'plugins'}…`} />
  <p class="context">
    CurseForge · {[javaFlavor, minecraftVersion].filter(Boolean).join(' · ') || 'All versions'}
  </p>
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
  {#if busy}<p class="context" role="status">Loading CurseForge content…</p>{/if}
  <div class="results">
    {#each results as item (item.projectId)}
      <div class="result">
        <button
          type="button"
          class="result-link"
          disabled={busy || !!adding}
          onclick={() => void open(item)}
        >
          <div class="icon">
            {#if item.iconURL}<img
                src={item.iconURL}
                alt=""
                width="40"
                height="40"
                loading="lazy"
              />{:else}<Icon name="box" size={18} />{/if}
          </div>
          <div class="info">
            <div class="title-row">
              <span class="title">{item.title}</span><Icon name="chevron" size={10} />
            </div>
            <p class="meta">by {item.author} · {formatCount(item.downloads)} downloads</p>
            <p class="description">{item.description}</p>
          </div>
        </button>
        {#if installed.has(item.projectId)}
          <span class="context">Added</span>
        {:else}
          <Button
            size="sm"
            variant="secondary"
            disabled={busy || !!adding || (mode === 'install' && !serverId) || !installSupported}
            onclick={() => void add(item)}
            >{adding === item.projectId
              ? installingReleaseId
                ? 'Installing…'
                : 'Checking…'
              : 'Add'}</Button
          >
        {/if}
      </div>
    {/each}
  </div>
  {#if !busy && ready && supported && !error && results.length === 0}<p class="context">
      No results. Try another search.
    </p>{/if}
  <div class="pagination">
    <Button
      size="sm"
      variant="secondary"
      disabled={busy || !supported || offset === 0}
      onclick={() => void search(Math.max(0, offset - 50))}>Previous</Button
    >
    <span class="context"
      >{offset + (results.length ? 1 : 0)}–{offset + results.length} of {total}</span
    >
    <Button
      size="sm"
      variant="secondary"
      disabled={busy || !supported || offset + results.length >= total}
      onclick={() => void search(offset + 50)}>Next</Button
    >
  </div>
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
  .catalog {
    display: grid;
    gap: 10px;
  }
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
  .results {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 480px;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .results::-webkit-scrollbar {
    display: none;
    width: 0;
  }
  .result {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .result:first-child {
    border-top: none;
  }
  .result-link {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    min-width: 0;
    flex: 1;
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .icon {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    border-radius: 8px;
    overflow: hidden;
    background: var(--msc2-tier-chrome);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--msc2-text-tertiary);
  }
  .icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    min-width: 0;
    flex: 1;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .meta {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .description {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
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
  .pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 12px;
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
