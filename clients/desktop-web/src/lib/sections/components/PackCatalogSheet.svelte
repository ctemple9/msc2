<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
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
  onMount(() => void search());
  onDestroy(() => {
    disposed = true;
    generation++;
  });
</script>

<Sheet title="Browse modpacks" size="lg" onClose={busy ? undefined : onClose}>
  <div class="content">
    <div class="providers">
      {#each ['modrinth', 'curseforge'] as name}
        <button
          type="button"
          class:chosen={provider === name}
          disabled={busy}
          onclick={() => {
            provider = name as typeof provider;
            void search();
          }}>{name === 'modrinth' ? 'Modrinth' : 'CurseForge'}</button
        >
      {/each}
    </div>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void search();
      }}
    >
      <input
        aria-label="Search modpacks"
        bind:value={query}
        placeholder="Search modpacks…"
      /><Button type="submit" disabled={busy}>Search</Button>
    </form>
    <p>
      All packs are shown. Choose an exact release, then review its Minecraft and loader setup
      before adding it to the draft.
    </p>
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
    {#if status}<p role="status">{status}</p>{/if}
    {#if operationId}<Button onclick={() => void cancel()}>Cancel download</Button>{/if}
    {#each results as item (item.projectId)}
      <button type="button" class="row" disabled={busy} onclick={() => (selected = item)}>
        {#if item.iconURL}<img
            src={item.iconURL}
            alt=""
            width="40"
            height="40"
            loading="lazy"
          />{/if}
        <span
          ><strong>{item.title}</strong><small>by {item.author}</small><span
            >{item.description}</span
          ></span
        ><span>Versions</span>
      </button>
    {/each}
    <div class="pages">
      <Button disabled={busy || offset === 0} onclick={() => void search(Math.max(0, offset - 50))}
        >Previous</Button
      ><span>{offset + (results.length ? 1 : 0)}–{offset + results.length} of {total}</span><Button
        disabled={busy || offset + results.length >= total}
        onclick={() => void search(offset + 50)}>Next</Button
      >
    </div>
  </div>
</Sheet>
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
  .content {
    display: grid;
    gap: 12px;
  }
  .providers,
  form,
  .pages {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .pages {
    justify-content: space-between;
  }
  .providers button {
    background: none;
    border: 0;
    color: var(--msc2-text-secondary);
    padding: 6px 0;
    cursor: pointer;
  }
  .providers button.chosen {
    color: var(--msc2-text-primary);
    border-bottom: 2px solid var(--msc2-accent);
  }
  input {
    flex: 1;
    min-width: 0;
    padding: 8px 10px;
    border-radius: 6px;
    border: 1px solid var(--msc2-hairline-subtle);
    background: var(--msc2-tier-content);
    color: var(--msc2-text-primary);
  }
  p,
  small,
  .pages {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .error {
    color: var(--msc2-status-error);
  }
  .row {
    display: flex;
    gap: 12px;
    align-items: start;
    text-align: left;
    background: none;
    color: var(--msc2-text-primary);
    border: 0;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
    padding: 10px 0;
    cursor: pointer;
  }
  .row > span:first-of-type {
    display: grid;
    flex: 1;
    gap: 4px;
    font-size: 13px;
  }
  img {
    border-radius: 6px;
    object-fit: cover;
  }
</style>
