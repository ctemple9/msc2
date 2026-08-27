<script lang="ts">
  // Ports MSC 1's ServerFilesTabView.swift: Server Root breadcrumb, Folders +
  // Files divided lists, Show in Finder, click-to-preview for a previewable
  // text file. Backed by P12.9's new GET /v1/files + GET /v1/files/read
  // routes -- frozen in the contract since Phase 11 but never given a
  // handler until now (same "frozen contract, no backend" gap P12.3 hit for
  // the Players tab; see rolling-plan.md).
  //
  // Edit/save (the oracle's TextPreviewSheet also lets you edit a file in
  // place) has no route in the contract at all, not even a reserved one --
  // out of scope here, left for a future contract-amendment step.
  //
  // "Reveal in Finder" only means something for a locally-connected agent
  // (the path names a file on *this* machine); a remote host's files have
  // nothing local to reveal, so that action is gated on hostId === the
  // client's own local-agent constant, same string App.svelte's host
  // registry already uses.
  import { onMount } from 'svelte';
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import ListRow from '../../components/base/ListRow.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import FilePreviewSheet from './FilePreviewSheet.svelte';
  import { getPlatform } from '../../platform';
  import type { Schema, ScreenProps } from '../shared/types';
  import { bytesLabel, call, errorMessage } from '../shared/types';
  import { breadcrumbsFor, browseDirectory, browseNoticeFor, relativeTime } from './model';

  export let api: ScreenProps['api'] = undefined;
  export let hostId = 'local-agent';
  export let serverId = 'survival';

  let servers: Schema['ServerDTO'][] = [];
  let currentPath = '';
  let listing: Schema['ServerFilesResponseDTO'] | undefined;
  let loading = true;
  let error: string | undefined;
  let previewingPath: string | undefined;
  let revealNotice: string | undefined;

  $: activeServer = servers.find((server) => server.id === serverId);
  $: isLocalHost = hostId === 'local-agent';
  $: breadcrumbs = breadcrumbsFor(currentPath);
  $: folders = listing?.items.filter((item) => item.isDirectory) ?? [];
  $: files = listing?.items.filter((item) => !item.isDirectory) ?? [];
  $: notice = browseNoticeFor(listing?.note);
  $: revealTitle = isLocalHost
    ? undefined
    : 'Show in Finder only works for a locally-connected agent.';

  async function loadServers(): Promise<void> {
    servers = await call(api, servers, '/v1/servers');
  }

  async function load(path: string): Promise<void> {
    loading = true;
    error = undefined;
    revealNotice = undefined;
    try {
      listing = api ? await browseDirectory(api, path) : undefined;
      currentPath = listing?.path ?? path;
    } catch (err) {
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  let loadedForServerId: string | undefined;
  $: if (serverId !== loadedForServerId) {
    loadedForServerId = serverId;
    void load('');
  }

  function openEntry(entry: Schema['ServerFileItemDTO']): void {
    if (entry.isDirectory) {
      void load(entry.path);
      return;
    }
    if (entry.isPreviewable) {
      previewingPath = entry.path;
      return;
    }
    void revealPath(entry.path);
  }

  async function revealPath(relativePath: string): Promise<void> {
    if (!activeServer) return;
    const absolute = relativePath
      ? `${activeServer.directory}/${relativePath}`
      : activeServer.directory;
    revealNotice = undefined;
    await (
      await getPlatform()
    ).revealInFileManager(absolute, async () => {
      revealNotice = 'Show in Finder needs the desktop app.';
    });
  }

  function fileSubtitle(entry: Schema['ServerFileItemDTO']): string {
    const parts: string[] = [];
    const modified = relativeTime(entry.modifiedAt);
    if (modified) parts.push(modified);
    if (entry.sizeBytes !== undefined) parts.push(bytesLabel(entry.sizeBytes));
    return parts.join(' · ');
  }

  onMount(() => {
    void loadServers();
  });
</script>

<div class="files">
  <Card padding="0">
    <div class="toolbar">
      <div class="crumbs">
        {#each breadcrumbs as crumb, index (crumb.path)}
          {#if index > 0}<span class="sep">/</span>{/if}
          <button
            type="button"
            class="crumb"
            class:current={index === breadcrumbs.length - 1}
            disabled={index === breadcrumbs.length - 1}
            onclick={() => void load(crumb.path)}
          >
            {crumb.label}
          </button>
        {/each}
      </div>
      <Button
        size="sm"
        variant="secondary"
        disabled={!activeServer || !isLocalHost}
        title={revealTitle}
        onclick={() => void revealPath(currentPath)}
      >
        Show in Finder
      </Button>
    </div>

    {#if loading}
      <div class="state">
        <p class="msc2-type-overline">Loading…</p>
      </div>
    {:else if error}
      <div class="state">
        <p class="state-error">{error}</p>
      </div>
    {:else if notice}
      <div class="state">
        <p class="state-error">{notice}</p>
      </div>
    {:else if folders.length === 0 && files.length === 0}
      <EmptyState title="This folder is empty" message="Nothing here yet.">
        <Icon name="folder" size={26} slot="icon" />
      </EmptyState>
    {:else}
      {#if folders.length > 0}
        <p class="msc2-type-overline group-label">Folders</p>
        {#each folders as entry, index (entry.path)}
          <ListRow
            title={entry.name}
            subtitle={fileSubtitle(entry)}
            last={index === folders.length - 1 && files.length === 0}
            onclick={() => openEntry(entry)}
          >
            <Icon slot="icon" name="folder" size={16} />
            <Icon slot="trailing" name="chevron" size={12} />
          </ListRow>
        {/each}
      {/if}
      {#if files.length > 0}
        <p class="msc2-type-overline group-label">Files</p>
        {#each files as entry, index (entry.path)}
          <ListRow
            title={entry.name}
            subtitle={fileSubtitle(entry)}
            last={index === files.length - 1}
            onclick={() => openEntry(entry)}
          >
            <Icon slot="icon" name="note" size={16} />
            <svelte:fragment slot="trailing"></svelte:fragment>
          </ListRow>
        {/each}
      {/if}
    {/if}
  </Card>
  {#if revealNotice}<p class="notice" role="status">{revealNotice}</p>{/if}
</div>

{#if previewingPath !== undefined}
  <FilePreviewSheet {api} path={previewingPath} onClose={() => (previewingPath = undefined)} />
{/if}

<style>
  .files {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--msc2-hairline-faint);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    min-width: 0;
  }
  .crumb {
    background: transparent;
    border: none;
    font: inherit;
    font-size: 12px;
    padding: 2px 2px;
    color: var(--msc2-text-secondary);
    cursor: pointer;
  }
  .crumb:hover:not(:disabled) {
    color: var(--msc2-text-primary);
  }
  .crumb.current {
    color: var(--msc2-text-primary);
    font-weight: 500;
    cursor: default;
  }
  .sep {
    color: var(--msc2-text-tertiary);
    font-size: 12px;
  }
  .group-label {
    padding: 12px 14px 6px;
  }
  .state {
    padding: 26px 16px;
    text-align: center;
  }
  .state-error {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-status-error);
  }
  .notice {
    margin: 0;
    padding: 0 2px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
</style>
