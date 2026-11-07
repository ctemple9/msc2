<script lang="ts">
  import { bytesLabel } from '../shared/types';
  import type { ExportNode } from './export-file-tree';
  export let nodes: ExportNode[];
  let open = new Set<string>();
  function expanded(path: string, value: boolean): void {
    const next = new Set(open);
    if (value) next.add(path);
    else next.delete(path);
    open = next;
  }
</script>

<ul>
  {#each nodes as node (node.path)}
    <li>
      {#if node.children}
        <details ontoggle={(event) => expanded(node.path, event.currentTarget.open)}>
          <summary><span>{node.name}</span><small>{node.count} files</small></summary>
          {#if open.has(node.path)}<svelte:self nodes={node.children} />{/if}
        </details>
      {:else if node.file}
        <div class="file" title={node.path}>
          <span>{node.name}</span><small
            >{bytesLabel(node.file.byteSize)}{node.file.enabled === false
              ? ' · disabled'
              : node.file.optional
                ? ' · optional'
                : ''}</small
          >
        </div>
      {/if}
    </li>
  {/each}
</ul>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    margin: 0;
    padding: 0;
  }
  summary {
    cursor: pointer;
    padding: 7px 0;
    font-size: 12px;
    color: var(--msc2-text-primary);
  }
  summary span {
    font-weight: 500;
  }
  small {
    font-size: 11px;
    color: var(--msc2-text-secondary);
    margin-left: 10px;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  details > :global(ul) {
    margin-left: 15px;
    padding-left: 9px;
    border-left: 1px solid var(--msc2-hairline-subtle);
  }
  .file {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 0 6px 15px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .file span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
</style>
