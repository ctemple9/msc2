<script lang="ts">
  // Ports the read-only half of ServerFilesTabView.swift's TextPreviewSheet
  // (ServerFilesTabView.swift:504-653) -- edit/save has no backing route in
  // the frozen contract at all (not even a reserved-but-unimplemented one,
  // unlike browse/read), so it stays out of scope here rather than being
  // faked; see rolling-plan.md's P12.9 entry.
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import { ApiError } from '../../api/client';
  import type { ScreenApi } from '../shared/types';
  import { readErrorMessage, readFile } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let path: string;
  export let onClose: () => void;

  let loading = true;
  let content = '';
  let truncated = false;
  let error: string | undefined;

  $: name = path.split('/').at(-1) ?? path;

  onMount(() => {
    void load();
  });

  async function load(): Promise<void> {
    loading = true;
    error = undefined;
    if (!api) {
      error = 'Connect to an agent to preview files.';
      loading = false;
      return;
    }
    try {
      const response = await readFile(api, path);
      content = response?.content ?? '';
      truncated = response?.truncated ?? false;
    } catch (err) {
      error =
        err instanceof ApiError ? readErrorMessage(err.error.message) : 'Could not open this file.';
    } finally {
      loading = false;
    }
  }
</script>

<Sheet title={name} size="lg" {onClose}>
  {#if loading}
    <p class="status">Loading…</p>
  {:else if error}
    <p class="status error">{error}</p>
  {:else}
    {#if truncated}
      <p class="status warn">Showing the first part of this file only.</p>
    {/if}
    <pre class="content">{content}</pre>
  {/if}
</Sheet>

<style>
  .status {
    margin: 0 0 10px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .status.error {
    color: var(--msc2-status-error);
  }
  .status.warn {
    color: var(--msc2-status-warn);
  }
  .content {
    margin: 0;
    font-family: var(--msc2-font-mono);
    font-size: 12px;
    line-height: 1.6;
    color: var(--msc2-text-primary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 60vh;
    overflow: auto;
  }
</style>
