<script lang="ts">
  import { onDestroy } from 'svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { bytesLabel, errorMessage } from '../shared/types';

  export let api: ScreenApi | undefined;
  export let hostName: string;
  type Preview = Schema['StorageCleanupPreviewDTO'];
  type Result = Schema['StorageCleanupResultDTO'];
  let preview: Preview | undefined;
  let busy = false;
  let notice = '';
  let alive = true;
  onDestroy(() => {
    alive = false;
  });

  async function scan(): Promise<void> {
    if (!api || busy) return;
    busy = true;
    preview = undefined;
    notice = '';
    try {
      const result = await api.post<Preview>('/v1/host/storage/preview');
      if (alive) preview = result;
    } catch (error) {
      if (alive)
        notice = `${errorMessage(error)} Both local and remote cleanup require an updated host agent.`;
    } finally {
      if (alive) busy = false;
    }
  }

  async function clean(): Promise<void> {
    if (!api || !preview || busy) return;
    busy = true;
    notice = '';
    try {
      const result = await api.post<Result>('/v1/host/storage/cleanup', {
        previewToken: preview.previewToken,
      });
      if (!alive) return;
      preview = undefined;
      notice = `Removed ${result.removed.length} map staging folder${result.removed.length === 1 ? '' : 's'} (${bytesLabel(result.removedBytes)}).${result.retained.length ? ` Retained: ${result.retained.join('; ')}. Scan again to review.` : ''}`;
    } catch (error) {
      if (alive) {
        preview = undefined;
        notice = errorMessage(error);
      }
    } finally {
      if (alive) busy = false;
    }
  }
</script>

<section class="storage">
  <p class="msc2-type-overline">Disk space on {hostName}</p>
  <p>
    Check for abandoned map copies and render output on this host. This data is rebuilt when you
    open the map. Server worlds, backups, mods, settings and credentials are preserved.
  </p>
  <Button variant="secondary" size="sm" disabled={!api || busy} onclick={() => void scan()}
    >{busy ? 'Working…' : 'Check reclaimable space…'}</Button
  >
  {#if preview}
    <p>{bytesLabel(preview.reclaimableBytes)} can be reclaimed on {hostName}.</p>
    {#if preview.entries.length === 0}
      <p class="hint">No abandoned map staging was found.</p>
    {:else}
      <ul>
        {#each preview.entries as entry (entry.id)}
          <li>
            <span class="path">{entry.path}</span>
            <span
              >{bytesLabel(entry.sizeBytes)} — {entry.removable
                ? 'Will delete'
                : 'Will retain'}</span
            >
            <span class="hint">{entry.reason}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#if preview.reclaimableBytes > 0}
      <p class="hint">
        Confirmation expires after five minutes. Files that become active or change after this scan
        will be retained.
      </p>
      <Button variant="secondary" size="sm" disabled={busy} onclick={() => void clean()}
        >Delete listed rebuildable data ({bytesLabel(preview.reclaimableBytes)})</Button
      >
    {/if}
  {/if}
  {#if notice}<p role="status">{notice}</p>{/if}
</section>

<style>
  .storage {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  p {
    margin: 0;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    width: 100%;
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 0;
  }
  .path {
    overflow-wrap: anywhere;
    font-size: 12px;
  }
  .hint {
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
</style>
