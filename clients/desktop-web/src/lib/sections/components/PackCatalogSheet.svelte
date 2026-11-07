<script lang="ts">
  import { onDestroy } from 'svelte';
  import CatalogBrowseSheet from './CatalogBrowseSheet.svelte';
  import CatalogBrowseContent from './CatalogBrowseContent.svelte';
  import Button from '../../components/base/Button.svelte';
  import ProjectDetailSheet from './ProjectDetailSheet.svelte';
  import { ApiError } from '../../api/client';
  import { openExternal } from '../../platform';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage } from '../shared/types';
  import { pollOperation } from './model';
  export let api: ScreenApi;
  export let javaFlavor: string;
  export let minecraftVersion: string | undefined;
  export let onClose: () => void;
  export let onSelected: (
    id: string,
    name: string,
    inspection: Schema['ModpackInspectionResultDTO'],
    api: ScreenApi,
  ) => void;
  const connection = api.snapshot?.() ?? api;
  let provider: 'modrinth' | 'curseforge' = 'modrinth';
  let query = '',
    error = '',
    status = '',
    key = '';
  let needsKey = false,
    busy = false,
    disposed = false;
  let offset = 0,
    total = 0;
  let results: Schema['CatalogItemDTO'][] = [];
  let selected: Schema['CatalogItemDTO'] | undefined;
  let operationId: string | undefined;
  let generation = 0;
  const current = () => !disposed && connection.isCurrent?.() !== false;
  async function search(page = 0): Promise<void> {
    if (!current() || operationId) return;
    const request = ++generation;
    busy = true;
    error = '';
    needsKey = false;
    try {
      const response = await connection.get<Schema['ProviderCatalogSearchDTO']>(
        `/v1/catalog/${provider}/search?kind=modpack&q=${encodeURIComponent(query)}&offset=${page}`,
      );
      if (current() && request === generation) {
        results = response.results;
        total = response.totalCount;
        offset = page;
      }
    } catch (cause) {
      if (current() && request === generation) {
        error = errorMessage(cause);
        results = [];
        needsKey = cause instanceof ApiError && cause.error.code === 'missing_curseforge_api_key';
      }
    } finally {
      if (current() && request === generation) busy = false;
    }
  }
  async function saveKey(): Promise<void> {
    if (!key.trim() || busy) return;
    busy = true;
    try {
      const result = await connection.post<Schema['CurseForgeApiKeyStatusDTO']>(
        '/v1/config/curseforge',
        { apiKey: key.trim() },
      );
      if (!result.configured) throw new Error('The agent did not save the key.');
      key = '';
      needsKey = false;
      await search();
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      key = '';
      if (current()) busy = false;
    }
  }
  async function acquire(versionId: string): Promise<void> {
    if (!selected || busy || !current()) return;
    const item = selected;
    selected = undefined;
    busy = true;
    error = '';
    let stagedId: string | undefined;
    let retained = false;
    try {
      const result = await connection.post<Schema['CatalogAcquireResultDTO']>(
        '/v1/catalog/modpacks/acquire',
        { provider, projectId: item.projectId, versionId },
      );
      operationId = result.operationId;
      const operation = await pollOperation(connection, operationId, (value) => {
        if (current()) status = value.statusLine ?? 'Downloading pack…';
      });
      if (operation?.state !== 'succeeded')
        throw new Error(operation?.error?.message ?? 'Pack download did not complete.');
      const id = operation.result?.stagedUploadId;
      if (typeof id !== 'string') throw new Error('The agent returned no staged pack.');
      stagedId = id;
      if (!current()) throw new Error('The selected host changed.');
      const inspection = await connection.post<Schema['ModpackInspectionResultDTO']>(
        '/v1/modpacks/inspect',
        { stagedUploadId: id },
      );
      if (!current()) throw new Error('The selected host changed.');
      if (!inspection.success) throw new Error(inspection.message);
      onSelected(id, item.title, inspection, connection);
      retained = true;
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (stagedId && !retained) {
        try {
          if (!connection.delete) throw new Error('Cleanup is unavailable.');
          await connection.delete(`/v1/staged-uploads/${encodeURIComponent(stagedId)}`);
        } catch (cause) {
          if (current()) error += ` Pack cleanup could not be confirmed: ${errorMessage(cause)}`;
        }
      }
      if (current()) {
        busy = false;
        operationId = undefined;
      }
    }
  }
  async function cancel(): Promise<void> {
    if (!operationId) return;
    try {
      await connection.post(`/v1/operations/${encodeURIComponent(operationId)}/cancel`, {});
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    }
  }
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  $: {
    query;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void search(), 350);
  }
  onDestroy(() => {
    disposed = true;
    generation++;
    clearTimeout(searchTimer);
  });
</script>

<CatalogBrowseSheet
  title="Browse modpacks"
  bind:provider
  disabled={busy}
  onClose={busy ? undefined : onClose}
  onProviderChanged={() => {
    selected = undefined;
    clearTimeout(searchTimer);
    void search();
  }}
>
  <CatalogBrowseContent
    bind:query
    noun="modpacks"
    subtitle={`${provider === 'modrinth' ? 'Modrinth' : 'CurseForge'}${[javaFlavor, minecraftVersion].filter(Boolean).length ? ` · ${[javaFlavor, minecraftVersion].filter(Boolean).join(' · ')}` : ''} · All packs shown`}
    {results}
    loading={busy}
    disabled={!!operationId}
    onDetail={(item) => (selected = item)}
    onAdd={(item) => (selected = item)}
    showEmpty={!error}
    notice={status}
    {offset}
    {total}
    onPage={(page) => void search(page)}
  >
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if needsKey}
      <form
        onsubmit={(event) => {
          event.preventDefault();
          void saveKey();
        }}
      >
        <input
          aria-label="CurseForge API key"
          type="password"
          autocomplete="off"
          bind:value={key}
        /><Button type="submit" disabled={busy}>Save key</Button>
      </form>
      <Button
        onclick={() =>
          void openExternal('https://console.curseforge.com/').catch(
            (cause) => (error = errorMessage(cause)),
          )}>Open CurseForge console</Button
      >
    {/if}
    {#if operationId}<Button onclick={() => void cancel()}>Cancel download</Button>{/if}
  </CatalogBrowseContent>
</CatalogBrowseSheet>
{#if selected}
  <ProjectDetailSheet
    api={connection}
    item={selected}
    {provider}
    {javaFlavor}
    serverMinecraftVersion={minecraftVersion}
    mode="stage"
    onStaged={(id) => void acquire(id)}
    onInstalled={() => {}}
    onClose={() => (selected = undefined)}
  />
{/if}

<style>
  form {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  input {
    flex: 1;
    min-width: 0;
    padding: 8px 10px;
    border-radius: 6px;
    border: 1px solid var(--msc2-hairline-subtle);
    background: var(--msc2-tier-content);
    color: var(--msc2-text-primary);
    font: inherit;
  }
  .error {
    margin: 0 0 10px;
    font-size: 12px;
    color: var(--msc2-status-error);
  }
</style>
