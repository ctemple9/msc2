<script lang="ts">
  // Ports the browsing half of ModrinthBrowserView.swift: search the active
  // server's add-on catalog and install a result. MSC 1's per-project detail
  // page (ModrinthProjectDetailView -- gallery, full version list, individual
  // version installs) has no backing route in the frozen contract: GET
  // /v1/catalog/search returns search hits only, and POST
  // /v1/components/install takes a projectId/slug/title and resolves the
  // latest compatible version itself. There is no GET for a single project's
  // detail or version list to browse. That richer page is a real, documented
  // gap -- not built here -- rather than faked with client-side guesses.
  //
  // The per-result icon here is real Modrinth artwork (CatalogItemDTO
  // .iconURL), the same category of content as a world slot's thumbnail --
  // not the rule #6 tell (a generic icon in a same-hue tinted box standing
  // in for an informational element). A flat neutral placeholder tile
  // covers projects with no icon, same shape as ModrinthBrowserView's own
  // `Color.secondary.opacity(0.15)` fallback.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import Badge from '../../components/base/Badge.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { addonPaths, pollOperation } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let addOnKind: 'mod' | 'plugin' = 'plugin';
  export let onClose: () => void;
  export let onInstalled: () => void;

  const noun = addOnKind === 'mod' ? 'mods' : 'plugins';

  let query = '';
  let results: Schema['CatalogItemDTO'][] = [];
  let supportsAddons = true;
  let subtitle = '';
  let note: string | undefined;
  let loading = false;
  let installing: Set<string> = new Set();
  let installed: Set<string> = new Set();
  let notice = '';
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  async function search(): Promise<void> {
    if (!api) return;
    loading = true;
    try {
      const response = await api.get<Schema['CatalogSearchResponseDTO']>(
        `${addonPaths.search}${encodeURIComponent(query)}`,
      );
      supportsAddons = response.supportsAddons;
      results = response.results ?? [];
      note = response.note;
      subtitle = [response.loaderName, response.gameVersion].filter(Boolean).join(' · ');
    } catch (error) {
      note = error instanceof Error ? error.message : 'Could not reach the add-on catalog.';
      results = [];
    } finally {
      loading = false;
    }
  }

  function scheduleSearch(): void {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void search(), 350);
  }

  // Field.svelte only binds `value` (no change-event prop), so the search is
  // driven off the reactive dependency on `query` -- fires once immediately
  // (debounced 350ms, same as the initial load) and again on every edit.
  $: {
    query;
    scheduleSearch();
  }

  async function install(item: Schema['CatalogItemDTO']): Promise<void> {
    if (!api) return;
    installing = new Set(installing).add(item.projectId);
    try {
      const result = await mutate<Schema['CatalogInstallResultDTO']>(api, addonPaths.install, {
        projectId: item.projectId,
        slug: item.slug,
        title: item.title,
      });
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId);
        notice =
          operation?.state === 'succeeded'
            ? `${result.message} — restart the server to apply.`
            : (operation?.error?.message ?? result.message);
      } else {
        notice = result.message;
      }
      installed = new Set(installed).add(item.projectId);
      onInstalled();
    } catch (error) {
      notice = error instanceof Error ? error.message : `Failed to install ${item.title}.`;
    } finally {
      const next = new Set(installing);
      next.delete(item.projectId);
      installing = next;
    }
  }

  function formatDownloads(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return `${n}`;
  }
</script>

<Sheet title={`Browse ${noun}`} size="lg" {onClose}>
  <div class="header">
    <Field bind:value={query} placeholder={`Search ${noun}…`} />
    <p class="subtitle">Modrinth{subtitle ? ` · ${subtitle}` : ''}</p>
  </div>
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  {#if !supportsAddons}
    <EmptyState title={`This server doesn't accept ${noun}`}>
      <Icon name="box" size={26} slot="icon" />
    </EmptyState>
  {:else if loading && results.length === 0}
    <p class="explain">Searching…</p>
  {:else if results.length === 0}
    <EmptyState
      title={note ?? `No ${noun} found`}
      message={note ? undefined : 'Try a different search term.'}
    >
      <Icon name="box" size={26} slot="icon" />
    </EmptyState>
  {:else}
    <div class="results">
      {#each results as item (item.projectId)}
        <div class="result">
          <div class="icon">
            {#if item.iconURL}
              <img src={item.iconURL} alt="" width="40" height="40" loading="lazy" />
            {:else}
              <Icon name="box" size={18} />
            {/if}
          </div>
          <div class="info">
            <div class="title-row">
              <span class="title">{item.title}</span>
              {#if item.isClientOnly}<Badge variant="status" tone="warn">Client-only</Badge>{/if}
            </div>
            <p class="meta">by {item.author} · {formatDownloads(item.downloads)} downloads</p>
            <p class="description">{item.description}</p>
          </div>
          {#if installed.has(item.projectId)}
            <span class="added">Added</span>
          {:else if installing.has(item.projectId)}
            <span class="added">Installing…</span>
          {:else}
            <Button size="sm" variant="secondary" onclick={() => void install(item)}>Add</Button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</Sheet>

<style>
  .header {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }
  .subtitle {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .notice {
    margin: 0 0 10px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .explain {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 480px;
    overflow-y: auto;
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
  .added {
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-status-ok);
  }
</style>
