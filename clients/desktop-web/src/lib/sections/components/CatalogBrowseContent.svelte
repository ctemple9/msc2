<script lang="ts">
  import Field from '../../components/base/Field.svelte';
  import Button from '../../components/base/Button.svelte';
  import Badge from '../../components/base/Badge.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import type { Schema } from '../shared/types';
  import { formatCount } from './model';
  export let query = '';
  export let noun: string;
  export let subtitle = '';
  export let results: Schema['CatalogItemDTO'][] = [];
  export let loading = false;
  export let disabled = false;
  export let canAdd = true;
  export let notice = '';
  export let emptyTitle = '';
  export let showEmpty = true;
  export let installed = new Set<string>();
  export let working = new Set<string>();
  export let workingLabel = 'Installing…';
  export let offset = 0;
  export let total: number | undefined = undefined;
  export let onPage: (offset: number) => void = () => {};
  export let onDetail: (item: Schema['CatalogItemDTO']) => void;
  export let onAdd: (item: Schema['CatalogItemDTO']) => void;
</script>

<div class="header">
  <Field bind:value={query} placeholder={`Search ${noun}…`} />
  {#if subtitle}<p class="subtitle">{subtitle}</p>{/if}
</div>
{#if notice}<p class="notice" role="status">{notice}</p>{/if}
<slot />
{#if loading && results.length === 0}
  <p class="explain" role="status">Searching…</p>
{:else if results.length === 0 && showEmpty}
  <EmptyState
    title={emptyTitle || `No ${noun} found`}
    message={emptyTitle ? undefined : 'Try a different search term.'}
  >
    <Icon name="box" size={26} slot="icon" />
  </EmptyState>
{:else}
  <div class="results">
    {#each results as item (item.projectId)}
      <div class="result">
        <button
          type="button"
          class="result-link"
          disabled={disabled || loading}
          onclick={() => onDetail(item)}
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
              <span class="title">{item.title}</span>
              {#if item.isClientOnly}<Badge variant="status" tone="warn">Client-only</Badge>{/if}
              <Icon name="chevron" size={10} />
            </div>
            <p class="meta">by {item.author} · {formatCount(item.downloads)} downloads</p>
            <p class="description">{item.description}</p>
          </div>
        </button>
        {#if installed.has(item.projectId)}<span class="added">Added</span>
        {:else if working.has(item.projectId)}<span class="added">{workingLabel}</span>
        {:else}<Button
            size="sm"
            variant="secondary"
            disabled={disabled || loading || !canAdd}
            onclick={() => onAdd(item)}>Add</Button
          >{/if}
      </div>
    {/each}
  </div>
{/if}
{#if total !== undefined}
  <div class="pagination">
    <Button
      size="sm"
      variant="secondary"
      disabled={loading || disabled || offset === 0}
      onclick={() => onPage(Math.max(0, offset - 50))}>Previous</Button
    >
    <span class="subtitle"
      >{offset + (results.length ? 1 : 0)}–{offset + results.length} of {total}</span
    >
    <Button
      size="sm"
      variant="secondary"
      disabled={loading || disabled || offset + results.length >= total}
      onclick={() => onPage(offset + 50)}>Next</Button
    >
  </div>
{/if}

<style>
  .pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 12px;
  }

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
  .added {
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-status-ok);
  }
</style>
