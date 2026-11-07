<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import PlayerContentList from './PlayerContentList.svelte';
  import {
    actionAllowed,
    playerContentChanges,
    contentItems,
    primary,
    needsAttention,
    itemMatches,
    supportingFolder,
    primaryFolders,
    categoryNames,
    changeNames,
    type PlayerContentItem,
  } from './player-content-view';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import StagedUploadSheet from './StagedUploadSheet.svelte';
  import PackChangesSheet from './PackChangesSheet.svelte';
  import { pickUploadSource, type UploadSource } from '../shared/upload-source';
  import { errorMessage, type Schema, type ScreenApi } from '../shared/types';
  import {
    contentOperation,
    contentRoot,
    type ContentEntry,
    type ContentSnapshot,
    type ContentReview,
  } from './content-model';
  export let api: ScreenApi;
  export let serverId: string;
  export let javaFlavor = '';
  export let minecraftVersion = '';
  export let onClose: () => void;
  export let onChanged: () => void;
  const connection = api.snapshot?.() ?? api;
  const root = contentRoot(serverId);
  let snapshot: ContentSnapshot | undefined;
  let review: ContentReview | undefined;
  let busy = false,
    disposed = false,
    showPacks = false;
  let error = '',
    status = '',
    operationId = '';
  let folder = 'mods',
    relativeName = '',
    evidence = '';
  let optional = false;
  let pendingSource: UploadSource | undefined;
  let uploadPath = '';
  let loading = true;
  let query = '';
  let filter = 'all';
  let selected = new Set<string>();
  let bulkAction = 'required';
  let expandedGroups = new Set<string>();
  $: items = contentItems(snapshot);
  $: attention = items.filter((item) => primary(item) && needsAttention(item));
  $: supportingAttention = items.filter((item) => !primary(item) && needsAttention(item) && itemMatches(item, query, filter)).length;
  $: categories = primaryFolders
    .map((name) => ({
      name,
      items: items.filter(
        (item) =>
          item.path.startsWith(`${name}/`) &&
          (item.player || needsAttention(item)) &&
          itemMatches(item, query, filter),
      ),
    }))
    .filter((group) => group.items.length);
  $: serverOnly = items.filter(
    (item) =>
      primary(item) && !item.player && !needsAttention(item) && itemMatches(item, query, filter),
  );
  $: supportGroups = groupSupporting(
    items.filter((item) => !primary(item) && itemMatches(item, query, filter)),
  );
  $: selectedItems = items.filter((item) => selected.has(item.path));
  $: bulkAllowed =
    selectedItems.length > 0 && selectedItems.every((item) => actionAllowed(item, bulkAction));
  function groupSupporting(
    entries: PlayerContentItem[],
  ): { name: string; items: PlayerContentItem[] }[] {
    const groups = new Map<string, PlayerContentItem[]>();
    for (const item of entries) {
      const name = supportingFolder(item);
      const group = groups.get(name) ?? [];
      group.push(item);
      groups.set(name, group);
    }
    return [...groups].map(([name, items]) => ({ name, items }));
  }
  function toggleGroup(name: string, open: boolean): void {
    const next = new Set(expandedGroups);
    if (open) next.add(name);
    else next.delete(name);
    expandedGroups = next;
  }
  function selectItems(paths: string[], checked: boolean): void {
    const next = new Set(selected);
    for (const path of paths) {
      if (checked) next.add(path);
      else next.delete(path);
    }
    selected = next;
  }
  function reviewSelection(): void {
    if (!bulkAllowed || busy) return;
    try {
      void prepare(playerContentChanges(selectedItems, bulkAction));
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
  function reviewEvidence(entry: ContentEntry, evidence: string): void {
    change(entry, 'setPlayerOptions', { redistributionEvidence: evidence });
  }
  const current = () => !disposed && connection.isCurrent?.() !== false;
  async function refresh(): Promise<void> {
    const result = await connection.get<ContentSnapshot>(`${root}/collection`);
    if (current()) {
      snapshot = result;
      selected = new Set();
    }
  }
  async function load(): Promise<void> {
    if (busy || !current()) return;
    loading = true;
    busy = true;
    error = '';
    try {
      const caps = await connection.get<Schema['CapabilitiesDTO']>('/v1/capabilities');
      if (!caps.contentCatalog?.playerContentManagement)
        throw new Error('Update this agent to manage player content.');
      await refresh();
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        loading = false;
      }
    }
  }
  async function prepare(changes: Partial<Schema['ContentChangeDTO']>[]): Promise<void> {
    if (!snapshot || busy || !current()) return;
    busy = true;
    error = '';
    review = undefined;
    try {
      const result = await connection.post<ContentReview>(`${root}/review`, {
        collectionRevision: snapshot.collection.revision,
        changes,
      });
      if (current()) review = result;
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) busy = false;
    }
  }
  async function apply(): Promise<void> {
    if (!review || busy || !current()) return;
    busy = true;
    error = '';
    try {
      await contentOperation(
        connection,
        `${root}/apply`,
        { reviewId: review.reviewId },
        (line) => {
          if (current()) status = line;
        },
        (id) => (operationId = id),
      );
      if (current()) {
        review = undefined;
        status = 'Player content saved. Server files were not changed.';
        await refresh();
        onChanged();
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        operationId = '';
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
  async function chooseFile(): Promise<void> {
    if (busy || !current()) return;
    error = '';
    try {
      const caps = await connection.get<Schema['CapabilitiesDTO']>('/v1/capabilities');
      if (!caps.contentCatalog?.playerFilePreparation)
        throw new Error('Update this agent to prepare player files.');
      const source = await pickUploadSource(
        connection,
        'addon-local-file',
        { label: 'Choose player content', extensions: [] },
        { targetServerId: serverId },
        current,
      );
      if (!source) return;
      uploadPath = `${folder}/${relativeName.trim() || source.name}`;
      pendingSource = source;
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    }
  }
  async function uploaded(
    value: Schema['StagedUploadCompleteResultDTO'],
    scoped: ScreenApi,
  ): Promise<void> {
    if (!snapshot) throw new Error('Refresh the content collection first.');
    let boundId: string | undefined;
    try {
      const prepared = await scoped.post<Schema['PlayerFilePreparedDTO']>(
        `${root}/player-files/prepare`,
        { path: uploadPath, stagedUploadId: value.stagedUploadId },
      );
      boundId = prepared.stagedUploadId;
      if (!current()) throw new Error('The selected host or server changed.');
      const changes: Partial<Schema['ContentChangeDTO']>[] = [
        {
          action: 'addPlayerFile',
          path: prepared.path,
          sha512: prepared.sha512,
          stagedUploadId: boundId,
          optional,
          ...(evidence.trim() ? { redistributionEvidence: evidence.trim() } : {}),
        },
      ];
      if (optional && (folder === 'resourcepacks' || folder === 'shaderpacks'))
        changes.push({
          action: 'setPlayerOptions',
          path: prepared.path,
          sha512: prepared.sha512,
          enabled: false,
        });
      const result = await scoped.post<ContentReview>(`${root}/review`, {
        collectionRevision: snapshot.collection.revision,
        changes,
      });
      if (current()) review = result;
    } finally {
      if (boundId) {
        if (!scoped.delete) throw new Error('Prepared upload cleanup is unavailable.');
        await scoped.delete(`/v1/staged-uploads/${encodeURIComponent(boundId)}`);
      }
    }
  }
  function change(
    entry: ContentEntry,
    action: Schema['ContentChangeDTO']['action'],
    values: Partial<Schema['ContentChangeDTO']> = {},
  ): void {
    void prepare([{ action, path: entry.path, sha512: entry.sha512, ...values }]);
  }
  onMount(() => void load());
  onDestroy(() => {
    disposed = true;
  });
</script>

<Sheet title="Player content" size="lg" onClose={busy && !loading ? undefined : onClose}>
  <div class="content">
    <p>
      Choose what players receive: required mods, optional content and the files that support your
      pack. Server files stay unchanged.
    </p>
    <div class="actions">
      <Button disabled={busy} onclick={() => void load()}>Refresh</Button><Button
        disabled={busy || !snapshot}
        onclick={() => (showPacks = true)}>Recover, update or replace pack…</Button
      >
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if status}<p role="status">{status}</p>{/if}
    {#if operationId}<Button onclick={() => void cancel()}>Cancel save</Button>{/if}
    {#if loading}<p class="loading" role="status">
        {snapshot ? 'Refreshing player content…' : 'Loading player content…'}
      </p>{/if}
    {#if busy && !loading && !operationId}<p role="status">Preparing content changes…</p>{/if}
    {#if snapshot}
      <p>
        {Object.keys(snapshot.collection.playerFiles).length} player files · {items.filter(
          (item) => item.player && primary(item),
        ).length} mods and packs · {items.filter((item) => !primary(item)).length} supporting files
      </p>
      {#if attention.length}
        <section class="attention-summary">
          <h3>Needs attention · {attention.length}</h3>
          <p>
            Missing player content, uncertain audience or different versions. Review these items
            before sharing.
          </p>
          <Button
            size="sm"
            variant="secondary"
            disabled={busy}
            onclick={() => { query = ''; filter = 'attention'; }}>Show items needing attention</Button
          >
        </section>
      {/if}
      <div class="actions filters">
        <label class="search"
          >Search content<input
            bind:value={query}
            placeholder="Name or folder…"
            disabled={busy}
          /></label
        >
        <label
          >Show<select bind:value={filter} disabled={busy}>
            <option value="all">All content</option><option value="required">Required</option
            ><option value="optional">Optional</option><option value="inactive">Inactive</option
            ><option value="attention">Needs attention</option>
          </select></label
        >
      </div>
      <div class="bulk-actions">
        <span>{selected.size} selected</span>
        <select
          aria-label="Action for selected files"
          bind:value={bulkAction}
          disabled={busy || !selected.size}
        >
          <option value="required">Required for players</option><option value="optional"
            >Optional for players</option
          >
          <option value="inactive" disabled={!selectedItems.every((item) => !!item.player)}
            >Leave inactive</option
          >
          <option value="active" disabled={!selectedItems.every((item) => !!item.player)}
            >Select for players</option
          >
          <option value="serverOnly" disabled={!selectedItems.every((item) => !!item.server)}
            >Server only</option
          >
          <option value="remove" disabled={!selectedItems.every((item) => !!item.player)}
            >Remove from players</option
          >
          <option value="restore" disabled={!selectedItems.every((item) => !!item.upstream)}
            >Restore original pack files</option
          >
        </select>
        <Button size="sm" disabled={busy || !bulkAllowed} onclick={reviewSelection}
          >Review changes…</Button
        >
        {#if selected.size}<Button
            size="sm"
            variant="secondary"
            disabled={busy}
            onclick={() => (selected = new Set())}>Clear selection</Button
          >{/if}
      </div>
      <p class="selection-note">
        Changes are reviewed before saving. Existing player customizations are kept unless you
        choose Restore original pack files. Server-only classification does not remove an existing
        player copy.
      </p>
      {#each categories as category (category.name)}
        <section>
          <h3>{categoryNames[category.name]}</h3>
          <PlayerContentList
            items={category.items}
            {selected}
            {busy}
            onSelect={selectItems}
            onEvidence={reviewEvidence}
          />
        </section>
      {:else}<p>
          {query || filter !== 'all'
            ? 'No mods or packs match this view.'
            : 'No player mods or packs selected yet. Add files below, or recover the original client pack.'}
        </p>{/each}
      {#if serverOnly.length}
        <details class="folder-group">
          <summary>Server-only or disabled content · {serverOnly.length}</summary>
          <p>These files stay on the server. Select files to review including them for players.</p>
          <PlayerContentList
            items={serverOnly}
            {selected}
            {busy}
            onSelect={selectItems}
            onEvidence={reviewEvidence}
          />
        </details>
      {/if}
      {#if supportGroups.length}
        <details class="supporting">
          <summary
            >Supporting files · {supportGroups.reduce(
              (count, group) => count + group.items.length,
              0,
            )}{supportingAttention ? ` · ${supportingAttention} need review` : ''}</summary
          >
          <p>
            Configuration, scripts and other pack files are grouped by folder. Keep the pack’s
            player copies, or select files together to review their audience. Permissions and
            private-file checks still apply at export.
          </p>
          {#each supportGroups as group (group.name)}
            <details
              class="folder-group"
              ontoggle={(event) => toggleGroup(group.name, event.currentTarget.open)}
            >
              <summary
                >{group.name} · {group.items.length} files{group.items.some(needsAttention)
                  ? ' · needs review'
                  : ''}</summary
              >
              {#if expandedGroups.has(group.name)}
                <PlayerContentList
                  items={group.items}
                  {selected}
                  {busy}
                  onSelect={selectItems}
                  onEvidence={reviewEvidence}
                />
              {/if}
            </details>
          {/each}
        </details>
      {/if}
      <details class="add-content">
        <summary>Add player content</summary>

        <p>
          Select one file and its Minecraft folder. Config/scripts require individual review;
          credentials, account files and worlds are excluded.
        </p>
        <div class="actions">
          <label
            >Folder<select bind:value={folder} disabled={busy}
              >{#each ['mods', 'resourcepacks', 'shaderpacks', 'datapacks', 'config', 'scripts', 'kubejs', 'defaultconfigs'] as name}<option
                  value={name}>{name}</option
                >{/each}</select
            ></label
          ><label
            >Relative filename (optional)<input
              bind:value={relativeName}
              placeholder="Use the selected file’s name"
              disabled={busy}
            /></label
          >
        </div>
        <label
          ><input type="checkbox" bind:checked={optional} disabled={busy} />Optional content
          selected by the owner</label
        >
        <label
          >Redistribution permission or license evidence (optional)<input
            bind:value={evidence}
            disabled={busy}
            placeholder="Leave unknown until reviewed"
          /></label
        >
        <Button disabled={busy} onclick={() => void chooseFile()}>Choose file…</Button>
      </details>
    {/if}
  </div>
</Sheet>
{#if review}
  <Sheet
    title="Review player changes"
    size="md"
    onClose={busy ? undefined : () => (review = undefined)}
  >
    <div class="content">
      <p>
        These changes apply to the player collection at revision {review.collectionRevision}.
        Changed server files require a fresh review.
      </p>
      <div class="review-list">
        {#each review.changes as item}<p>
            <strong>{item.path}</strong> · {changeNames[item.action] ?? item.action}
            {#if typeof item.optional === 'boolean'}
              · {item.optional ? 'Optional' : 'Required'}{/if}
            {#if item.audience}
              · {item.audience === 'server'
                ? 'Server only'
                : item.audience === 'both'
                  ? 'Server and players'
                  : 'Players only'}{/if}
            {#if typeof item.enabled === 'boolean'}
              · {item.enabled ? 'Selected' : 'Inactive'}{/if}
          </p>{/each}
      </div>
      {#each review.warnings as warning}<p>{warning}</p>{/each}
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      {#if operationId}<Button onclick={() => void cancel()}>Cancel save</Button>{/if}
      <Button disabled={busy} onclick={() => void apply()}
        >{busy ? 'Saving…' : 'Apply player changes'}</Button
      >
    </div>
  </Sheet>
{/if}
{#if pendingSource}
  <StagedUploadSheet
    api={connection}
    source={pendingSource}
    purpose="addon-local-file"
    completionStage="installing"
    onComplete={uploaded}
    onClose={() => (pendingSource = undefined)}
  />
{/if}
{#if showPacks && snapshot}
  <PackChangesSheet
    api={connection}
    {serverId}
    {javaFlavor}
    {minecraftVersion}
    {snapshot}
    onClose={() => (showPacks = false)}
    onChanged={() => {
      void load();
      onChanged();
    }}
  />
{/if}

<style>
  .review-list {
    max-height: 360px;
    overflow-y: auto;
    display: grid;
    gap: 8px;
  }
  section {
    min-width: 0;
  }
  .search {
    flex: 1;
    min-width: 180px;
  }
  .bulk-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 10px 0;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
    font-size: 12px;
  }
  .selection-note {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .loading {
    padding: 32px 0;
  }
  .attention-summary {
    display: grid;
    gap: 8px;
  }
  .attention-summary h3 {
    color: var(--msc2-status-warn);
  }
  .supporting,
  .folder-group,
  .add-content {
    border-top: 1px solid var(--msc2-hairline-subtle);
    padding-top: 12px;
  }
  summary {
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    color: var(--msc2-text-primary);
    overflow-wrap: anywhere;
  }
  details > p,
  details > .actions,
  details > label {
    margin-top: 10px;
  }
  details[open] > summary {
    margin-bottom: 10px;
  }
  .supporting .folder-group {
    margin-top: 12px;
  }

  .content {
    display: grid;
    gap: 12px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 8px;
  }
  p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-secondary);
  }
  h3 {
    margin: 8px 0 0;
    font-size: 13px;
    font-weight: 500;
  }
  strong {
    overflow-wrap: anywhere;
    font-size: 13px;
    font-weight: 500;
  }
  label {
    display: grid;
    gap: 4px;
    font-size: 12px;
  }
  label:has(input[type='checkbox']) {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  input:not([type='checkbox']),
  select {
    background: var(--msc2-tier-content);
    color: var(--msc2-text-primary);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 6px;
    padding: 7px 9px;
  }
  .error {
    color: var(--msc2-status-error);
  }
</style>
