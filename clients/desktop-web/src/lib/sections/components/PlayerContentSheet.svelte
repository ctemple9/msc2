<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import StagedUploadSheet from './StagedUploadSheet.svelte';
  import PackChangesSheet from './PackChangesSheet.svelte';
  import { pickUploadSource, type UploadSource } from '../shared/upload-source';
  import { errorMessage, bytesLabel, type Schema, type ScreenApi } from '../shared/types';
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
  let fileEvidence: Record<string, string> = {};
  const current = () => !disposed && connection.isCurrent?.() !== false;
  async function refresh(): Promise<void> {
    const result = await connection.get<ContentSnapshot>(`${root}/collection`);
    if (current()) snapshot = result;
  }
  async function load(): Promise<void> {
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
      if (current()) busy = false;
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

<Sheet title="Player content" size="lg" onClose={busy ? undefined : onClose}>
  <div class="content">
    <p>
      Keep a separate collection for players. Server-only content stays on the server; player
      changes here do not install or delete server files.
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
    {#if snapshot}
      <h3>Player requirements</h3>
      {#each snapshot.requirements.filter((item) => !['matched', 'serverOnly', 'serverDisabled', 'optionalMissing'].includes(item.state)) as requirement}
        <p><strong>{requirement.path}</strong> — {requirement.reason}</p>
      {/each}
      <h3>Server content</h3>
      {#each Object.values(snapshot.collection.currentFiles) as entry (entry.path)}
        <div class="row">
          <strong>{entry.path}</strong>
          <p>
            {entry.audience} · {bytesLabel(entry.byteSize)}{entry.optional ? ' · optional' : ''}
          </p>
          {#each entry.unresolvedReasons as reason}<p>{reason}</p>{/each}
          <div class="actions">
            <Button disabled={busy} onclick={() => change(entry, 'copyServerToPlayers')}
              >Players need this</Button
            ><Button
              disabled={busy}
              onclick={() => change(entry, 'setServerAudience', { audience: 'server' })}
              >Server only</Button
            ><Button
              disabled={busy}
              onclick={() =>
                void prepare([
                  {
                    action: 'copyServerToPlayers',
                    path: entry.path,
                    sha512: entry.sha512,
                    optional: true,
                  },
                  {
                    action: 'setServerAudience',
                    path: entry.path,
                    sha512: entry.sha512,
                    audience: 'both',
                    optional: true,
                  },
                ])}>Optional for players</Button
            >
          </div>
        </div>
      {/each}
      <p>
        Changing classification keeps an existing player copy until you explicitly remove it below.
      </p>
      <h3>Player collection</h3>
      {#each Object.values(snapshot.collection.playerFiles) as entry (entry.path)}
        <div class="row">
          <strong>{entry.path}</strong>
          <p>
            {entry.optional ? 'Optional' : 'Required'} · {entry.enabled === false
              ? 'Inactive'
              : 'Selected'} · {bytesLabel(entry.byteSize)}
          </p>
          {#each entry.unresolvedReasons as reason}<p>{reason}</p>{/each}
          <label
            >License or redistribution evidence<input
              disabled={busy}
              bind:value={fileEvidence[entry.path]}
              placeholder={entry.redistributionEvidence ?? 'Unknown'}
            /></label
          >
          <Button
            disabled={busy || !fileEvidence[entry.path]?.trim()}
            onclick={() =>
              change(entry, 'setPlayerOptions', {
                redistributionEvidence: fileEvidence[entry.path].trim(),
              })}>Save evidence for this file</Button
          >
          <div class="actions">
            <Button
              disabled={busy}
              onclick={() => change(entry, 'setPlayerOptions', { optional: !entry.optional })}
              >{entry.optional ? 'Make required' : 'Make optional'}</Button
            ><Button
              disabled={busy}
              onclick={() =>
                change(entry, 'setPlayerOptions', { enabled: entry.enabled === false })}
              >{entry.enabled === false ? 'Select' : 'Leave inactive'}</Button
            ><Button disabled={busy} onclick={() => change(entry, 'removePlayerFile')}
              >Remove from players</Button
            >
            {#if snapshot.collection.upstreamPlayerFiles[entry.path]}<Button
                disabled={busy}
                onclick={() =>
                  change(
                    snapshot!.collection.upstreamPlayerFiles[entry.path],
                    'restorePlayerUpstream',
                  )}>Restore upstream</Button
              >{/if}
          </div>
        </div>
      {/each}
      <h3>Add player content</h3>
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
        ><input type="checkbox" bind:checked={optional} disabled={busy} />Optional content selected
        by the owner</label
      >
      <label
        >Redistribution permission or license evidence (optional)<input
          bind:value={evidence}
          disabled={busy}
          placeholder="Leave unknown until reviewed"
        /></label
      >
      <Button disabled={busy} onclick={() => void chooseFile()}>Choose file…</Button>
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
      {#each review.changes as item}<p>
          <strong>{item.path}</strong> · {item.action}{item.optional === true ? ' · optional' : ''}
        </p>{/each}
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
      void refresh();
      onChanged();
    }}
  />
{/if}

<style>
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
  .row {
    display: grid;
    gap: 6px;
    padding: 10px 0;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
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
