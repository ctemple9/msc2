<script lang="ts">
  import Button from '../../components/base/Button.svelte';
  import { bytesLabel } from '../shared/types';
  import type { ContentEntry } from './content-model';
  import { itemState, needsAttention, type PlayerContentItem } from './player-content-view';
  export let items: PlayerContentItem[];
  export let selected: Set<string>;
  export let busy: boolean;
  export let onSelect: (paths: string[], selected: boolean) => void;
  export let onEvidence: (entry: ContentEntry, evidence: string) => void;
  let page = 0;
  let fileEvidence: Record<string, string> = {};
  $: page = Math.min(page, Math.max(0, Math.ceil(items.length / 40) - 1));
  $: visible = items.slice(page * 40, page * 40 + 40);
  $: allSelected = visible.length > 0 && visible.every((item) => selected.has(item.path));
</script>

{#if items.length}
  <div class="list-actions">
    <label
      ><input
        type="checkbox"
        checked={allSelected}
        disabled={busy}
        onchange={(event) =>
          onSelect(
            visible.map((item) => item.path),
            event.currentTarget.checked,
          )}
      />Select this page</label
    >
    <span>{items.length} files</span>
  </div>
  <div class="files">
    {#each visible as item (item.path)}
      <div class="file">
        <div class="file-line">
          <label class="selection"
            ><input
              type="checkbox"
              checked={selected.has(item.path)}
              disabled={busy}
              onchange={(event) => onSelect([item.path], event.currentTarget.checked)}
            /><span class="name" title={item.path}>{item.path.split('/').pop()}</span></label
          >
          <span class="state">{itemState(item)}</span>
        </div>
        {#if needsAttention(item)}<p class="attention">
            {item.requirement?.reason ??
              item.player?.unresolvedReasons[0] ??
              item.server?.unresolvedReasons[0]}
          </p>{/if}
        <details>
          <summary
            >File details{item.player && item.server && item.player.sha512 !== item.server.sha512
              ? ' · player customization'
              : ''}</summary
          >
          <div class="details">
            <p class="path">{item.path}</p>
            <p>
              {bytesLabel((item.player ?? item.server)!.byteSize)}{item.server
                ? ' · Present on server'
                : ' · Player only'}{item.player ? ' · Included for players' : ''}
            </p>
            {#each [...new Set( [...(item.server?.unresolvedReasons ?? []), ...(item.player?.unresolvedReasons ?? [])] )] as reason}<p
              >
                {reason}
              </p>{/each}
            {#if item.player}
              <label
                >Redistribution permission or license evidence<input
                  disabled={busy}
                  bind:value={fileEvidence[item.path]}
                  placeholder={item.player.redistributionEvidence ?? 'Not reviewed'}
                /></label
              >
              <Button
                size="sm"
                variant="secondary"
                disabled={busy || !fileEvidence[item.path]?.trim()}
                onclick={() => onEvidence(item.player!, fileEvidence[item.path].trim())}
                >Review permission for this file</Button
              >
            {/if}
          </div>
        </details>
      </div>
    {/each}
  </div>
  {#if items.length > 40}<div class="pages">
      <Button size="sm" variant="secondary" disabled={busy || page === 0} onclick={() => page--}
        >Previous</Button
      >
      <span>{page * 40 + 1}–{Math.min((page + 1) * 40, items.length)} of {items.length}</span>
      <Button
        size="sm"
        variant="secondary"
        disabled={busy || (page + 1) * 40 >= items.length}
        onclick={() => page++}>Next</Button
      >
    </div>{/if}
{/if}

<style>
  .list-actions,
  .file-line,
  .pages {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .list-actions,
  .pages {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    padding: 8px 0;
  }
  .files {
    max-height: 360px;
    overflow-y: auto;
  }
  .file {
    padding: 9px 0;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  label {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 12px;
    min-width: 0;
  }
  input[type='checkbox'] {
    margin: 0;
    flex-shrink: 0;
  }
  .name {
    overflow-wrap: anywhere;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .state {
    font-size: 11px;
    color: var(--msc2-text-secondary);
    flex-shrink: 0;
  }
  p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-secondary);
  }
  .attention {
    margin: 4px 0 0 21px;
    font-size: 11px;
    color: var(--msc2-status-warn);
  }
  details {
    margin: 4px 0 0 21px;
  }
  summary {
    width: fit-content;
    cursor: pointer;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .details {
    display: grid;
    gap: 8px;
    padding: 8px 0;
  }
  .details label {
    display: grid;
    gap: 4px;
  }
  .path {
    overflow-wrap: anywhere;
  }
  input:not([type='checkbox']) {
    background: var(--msc2-tier-content);
    color: var(--msc2-text-primary);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 6px;
    padding: 7px 9px;
    font: inherit;
  }
</style>
