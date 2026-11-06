<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import StagedUploadSheet from './StagedUploadSheet.svelte';
  import PackCatalogSheet from './PackCatalogSheet.svelte';
  import { pickUploadSource, type UploadSource } from '../shared/upload-source';
  import { errorMessage, type Schema, type ScreenApi } from '../shared/types';
  import {
    contentOperation,
    contentRoot,
    uploadReviewedFile,
    type ContentSnapshot,
    type ContentReview,
    type PackReview,
    type PathChange,
  } from './content-model';
  export let api: ScreenApi;
  export let serverId: string;
  export let javaFlavor: string;
  export let minecraftVersion: string;
  export let snapshot: ContentSnapshot;
  export let onClose: () => void;
  export let onChanged: () => void;
  const connection = api.snapshot?.() ?? api;
  const root = contentRoot(serverId);
  let mode: 'recovery' | 'update' | 'replacement' = 'recovery';
  let busy = false,
    disposed = false,
    browse = false,
    dirty = false;
  let error = '',
    status = '',
    operationId = '';
  let retained: Schema['RetainedPackStatusDTO'] | undefined;
  let review: PackReview | undefined;
  let recovery: ContentReview | undefined;
  let recoveryPackId: string | undefined;
  let pendingSource: UploadSource | undefined;
  let confirmOrigin = false,
    confirmWorld = false;
  let restoreServer = new Set<string>(),
    restorePlayer = new Set<string>(),
    recoverPaths = new Set<string>(),
    preserve = new Set<string>();
  let audiences: Record<string, string> = {};
  let manualAbort: AbortController | undefined;
  $: basePackId = snapshot.collection.basePackId;
  $: additions = Object.values(snapshot.collection.currentFiles).filter(
    (entry) => !snapshot.collection.upstreamFiles[entry.path],
  );
  $: recoveryUpstream =
    recovery?.collection.upstreamPlayerFiles ?? snapshot.collection.upstreamPlayerFiles;
  $: changeGroups = review
    ? ([
        { title: 'Server', files: review.serverChanges },
        { title: 'Players', files: review.playerChanges },
      ] as { title: string; files: PathChange[] }[])
    : [];
  const current = () => !disposed && connection.isCurrent?.() !== false;
  function toggle(set: Set<string>, path: string): Set<string> {
    const next = new Set(set);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    dirty = true;
    return next;
  }
  async function loadRetained(): Promise<void> {
    if (!basePackId) return;
    const result = await connection.get<Schema['RetainedPackStatusDTO']>(
      `${root}/retained-packs/${encodeURIComponent(basePackId)}`,
    );
    if (current()) retained = result;
  }
  async function initialize(): Promise<void> {
    busy = true;
    try {
      const caps = await connection.get<Schema['CapabilitiesDTO']>('/v1/capabilities');
      if (!caps.contentCatalog?.reviewedPackUpdates || !caps.contentCatalog?.playerContentRecovery)
        throw new Error('Update this agent to review pack changes and recover player content.');
      await loadRetained();
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) busy = false;
    }
  }
  function selectMode(value: typeof mode): void {
    mode = value;
    review = undefined;
    recovery = undefined;
    recoveryPackId = undefined;
    dirty = false;
    error = '';
    status = '';
    restoreServer = new Set();
    restorePlayer = new Set();
    audiences = {};
    preserve = new Set();
  }
  async function chooseArchive(): Promise<void> {
    if (busy) return;
    try {
      pendingSource =
        (await pickUploadSource(
          connection,
          'modpack-archive',
          { label: 'Choose the exact pack archive', extensions: ['zip', 'mrpack'] },
          { targetServerId: serverId },
          current,
        )) ?? undefined;
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    }
  }
  async function submit(uploadId?: string): Promise<void> {
    if (busy || !current()) return;
    busy = true;
    error = '';
    try {
      let route: string, body: unknown;
      if (mode === 'recovery') {
        route = `${root}/recovery-review`;
        body = {
          collectionRevision: snapshot.collection.revision,
          selectedPaths: [...recoverPaths],
          restorePaths: [...restorePlayer],
          ...(uploadId
            ? { stagedUploadId: uploadId }
            : recoveryPackId || basePackId
              ? { retainedPackId: recoveryPackId ?? basePackId }
              : {}),
        };
      } else {
        route = `${root}/pack-${mode === 'replacement' ? 'replacement' : 'update'}-review`;
        body = {
          collectionRevision: snapshot.collection.revision,
          ...(uploadId ? { stagedUploadId: uploadId } : { previousReviewId: review?.reviewId }),
          restoreServer: [...restoreServer],
          restorePlayer: [...restorePlayer],
          audiences,
          confirmUnverifiedOrigin: confirmOrigin,
          confirmWorldCompatibility: confirmWorld,
          preserveAdditions: Object.fromEntries(
            additions
              .filter((entry) => preserve.has(entry.path))
              .map((entry) => [entry.path, entry.sha512]),
          ),
        };
      }
      const accepted = await connection.post<{ operationId: string }>(route, body);
      operationId = accepted.operationId;
      const { pollOperation } = await import('./model');
      const result = await pollOperation(connection, operationId, (value) => {
        if (current()) status = value.statusLine ?? 'Preparing comparison…';
      });
      if (result?.state !== 'succeeded')
        throw new Error(result?.error?.message ?? 'Pack review did not complete.');
      const values = result.result as Record<string, unknown> | undefined;
      if (typeof values?.reviewId !== 'string')
        throw new Error('The agent returned no review identity.');
      if (mode === 'recovery') {
        const response = await connection.get<ContentReview>(
          `${root}/reviews/${encodeURIComponent(values.reviewId)}`,
        );
        if (current()) {
          recovery = response;
          recoveryPackId = typeof values.packId === 'string' ? values.packId : recoveryPackId;
        }
      } else {
        const response = await connection.get<PackReview>(
          `${root}/pack-reviews/${encodeURIComponent(values.reviewId)}`,
        );
        if (current()) review = response;
      }
      if (current()) {
        dirty = false;
        status = 'Comparison ready. Customizations are preserved unless selected for restoration.';
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
      else throw cause;
    } finally {
      if (current()) {
        busy = false;
        operationId = '';
      }
    }
  }
  async function apply(): Promise<void> {
    if (busy || dirty || !current()) return;
    const chosen = mode === 'recovery' ? recovery : review;
    if (!chosen) return;
    busy = true;
    error = '';
    try {
      await contentOperation(
        connection,
        `${root}/${mode === 'recovery' ? 'apply' : mode === 'replacement' ? 'pack-replacement' : 'pack-update'}`,
        { reviewId: chosen.reviewId },
        (line) => {
          if (current()) status = line;
        },
        (id) => (operationId = id),
      );
      if (current()) {
        status =
          mode === 'recovery'
            ? 'Player recovery saved; server files were unchanged.'
            : 'Pack changes applied. Restart the server when ready.';
        review = undefined;
        recovery = undefined;
        onChanged();
        snapshot = await connection.get<ContentSnapshot>(`${root}/collection`);
        await loadRetained();
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
  async function retryRetained(): Promise<void> {
    if (!retained || busy) return;
    busy = true;
    error = '';
    try {
      await contentOperation(
        connection,
        `${root}/retained-packs/${encodeURIComponent(retained.packId)}/retry`,
        {},
        (line) => {
          if (current()) status = line;
        },
        (id) => (operationId = id),
      );
      await loadRetained();
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        operationId = '';
      }
    }
  }
  async function manual(path: string, packReview = false): Promise<void> {
    if (busy) return;
    const prefix =
      packReview && review
        ? `${root}/pack-reviews/${encodeURIComponent(review.reviewId)}/manual`
        : retained
          ? `${root}/retained-packs/${encodeURIComponent(retained.packId)}/manual`
          : undefined;
    if (!prefix) return;
    busy = true;
    error = '';
    manualAbort = new AbortController();
    try {
      await uploadReviewedFile(connection, prefix, path, manualAbort.signal, (bytes) => {
        if (current()) status = `${bytes.toLocaleString()} bytes transferred`;
      });
      if (current()) {
        status = 'File checked. Refresh the comparison before applying.';
        dirty = true;
        await loadRetained();
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        manualAbort = undefined;
      }
    }
  }
  async function cancel(): Promise<void> {
    if (operationId) {
      try {
        await connection.post(`/v1/operations/${encodeURIComponent(operationId)}/cancel`, {});
      } catch (cause) {
        if (current()) error = errorMessage(cause);
      }
    }
    manualAbort?.abort();
  }
  async function uploaded(value: Schema['StagedUploadCompleteResultDTO']): Promise<void> {
    await submit(value.stagedUploadId);
    if (error) throw new Error(error);
  }
  async function catalogSelected(id: string): Promise<void> {
    browse = false;
    try {
      await submit(id);
    } finally {
      try {
        await connection.delete?.(`/v1/staged-uploads/${encodeURIComponent(id)}`);
      } catch (cause) {
        if (current()) error += ` Archive cleanup unconfirmed: ${errorMessage(cause)}`;
      }
    }
  }
  onMount(() => void initialize());
  onDestroy(() => {
    disposed = true;
    manualAbort?.abort();
  });
</script>

<Sheet title="Pack content" size="lg" onClose={busy ? undefined : onClose}>
  <div class="content">
    <div class="actions">
      {#each ['recovery', 'update', 'replacement'] as value}<Button
          disabled={busy}
          variant={mode === value ? 'primary' : 'secondary'}
          onclick={() => selectMode(value as typeof mode)}
          >{value === 'recovery'
            ? 'Recover players'
            : value === 'update'
              ? 'Update base pack'
              : 'Replace base pack'}</Button
        >{/each}
    </div>
    <p>
      {mode === 'recovery'
        ? 'Recover the client collection from the original pack or explicitly selected server files. This does not reinstall the server or reconstruct absent client-only files from an installed mod list.'
        : 'Keep owner customizations by default. Review each conflict and choose individual upstream restorations. Server changes require a stopped server; worlds are excluded.'}
    </p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if status}<p role="status">{status}</p>{/if}
    {#if busy && (operationId || manualAbort)}<Button onclick={() => void cancel()}
        >Cancel current work</Button
      >{/if}
    {#if mode === 'replacement'}
      <label
        ><input
          type="checkbox"
          bind:checked={confirmWorld}
          onchange={() => (dirty = true)}
          disabled={busy}
        />I understand switching packs may make existing worlds incompatible.</label
      >
      <h3>Owner additions to preserve</h3>
      <p>
        Check an item only after confirming it works with the replacement pack. MSC binds this
        choice to its current checksum.
      </p>
      {#each additions as entry (entry.path)}<label
          ><input
            type="checkbox"
            checked={preserve.has(entry.path)}
            onchange={() => (preserve = toggle(preserve, entry.path))}
            disabled={busy}
          />{entry.path}</label
        >{/each}
    {/if}
    {#if mode !== 'recovery'}<label
        ><input
          type="checkbox"
          bind:checked={confirmOrigin}
          onchange={() => (dirty = true)}
          disabled={busy}
        />I reviewed the identity of a local archive whose provider origin cannot be verified.</label
      >{/if}
    <div class="actions">
      <Button
        disabled={busy || (mode === 'replacement' && !confirmWorld)}
        onclick={() => void chooseArchive()}
        >Choose {mode === 'recovery'
          ? 'original client'
          : mode === 'update'
            ? 'updated'
            : 'replacement'} archive…</Button
      ><Button
        disabled={busy || (mode === 'replacement' && !confirmWorld)}
        onclick={() => (browse = true)}>Browse packs…</Button
      >
    </div>
    {#if mode === 'recovery'}
      {#if retained}
        <h3>Retained original: {retained.name} · {retained.version}</h3>
        {#if retained.playerRecoveryRequired}<p>
            A server pack does not prove that the missing client collection has been recovered.
            Choose the corresponding original client pack.
          </p>{/if}
        <div class="actions">
          <Button disabled={busy} onclick={() => void submit()}
            >Review recovery from retained original</Button
          ><Button disabled={busy} onclick={() => void retryRetained()}
            >Retry missing downloads</Button
          >
        </div>
        {#each retained.missing as file (file.path)}<div class="row">
            <strong>{file.path}</strong>
            <p>{file.reason}</p>
            {#if file.manualRecoveryAvailable}<Button
                disabled={busy}
                onclick={() => void manual(file.path)}>Choose exact file…</Button
              >{/if}
          </div>{/each}
      {/if}
      <h3>Recover selected server files</h3>
      {#each Object.values(snapshot.collection.currentFiles).filter((entry) => !entry.path.startsWith('plugins/')) as entry (entry.path)}<label
          ><input
            type="checkbox"
            checked={recoverPaths.has(entry.path)}
            onchange={() => (recoverPaths = toggle(recoverPaths, entry.path))}
            disabled={busy}
          />{entry.path}</label
        >{/each}
      <Button disabled={busy || recoverPaths.size === 0} onclick={() => void submit()}
        >Review selected server files</Button
      >
      {#if recovery}
        <h3>Recovery comparison</h3>
        {#each recovery.warnings as warning}<p>{warning}</p>{/each}
        {#each Object.values(recovery.collection.playerFiles) as entry (entry.path)}<p>
            {entry.path} · {entry.optional ? 'Optional' : 'Required'}
          </p>{/each}
        <h3>Explicit upstream restorations</h3>
        {#each Object.keys(recoveryUpstream) as path (path)}<label
            ><input
              type="checkbox"
              checked={restorePlayer.has(path)}
              onchange={() => (restorePlayer = toggle(restorePlayer, path))}
              disabled={busy}
            />Restore {path}</label
          >{/each}
        <Button disabled={busy} onclick={() => void submit()}>Refresh recovery comparison</Button
        ><Button disabled={busy || dirty} onclick={() => void apply()}
          >Apply reviewed player recovery</Button
        >
      {/if}
    {:else if review}
      <h3>{review.name} · {review.version}</h3>
      {#each review.warnings as warning}<p>{warning}</p>{/each}
      {#each changeGroups as group}
        <h3>{group.title} changes</h3>
        {#each group.files as item (item.path)}<div class="row">
            <strong>{item.path}</strong>
            <p>{item.action}{item.conflict ? ' · owner conflict' : ''} — {item.reason}</p>
            <label
              ><input
                type="checkbox"
                disabled={busy}
                checked={group.title === 'Server'
                  ? restoreServer.has(item.path)
                  : restorePlayer.has(item.path)}
                onchange={() => {
                  if (group.title === 'Server') restoreServer = toggle(restoreServer, item.path);
                  else restorePlayer = toggle(restorePlayer, item.path);
                }}
              />Restore this upstream item</label
            >
          </div>{/each}
      {/each}
      {#if review.missing.length}<h3>Resolve before applying</h3>
        {#each review.missing as item}<p>{item}</p>{/each}{/if}
      {#each review.missingFiles ?? [] as file (file.path)}<div class="row">
          <strong>{file.path}</strong>
          <p>{file.reason}</p>
          {#if file.manualRecoveryAvailable}<Button
              disabled={busy}
              onclick={() => void manual(file.path, true)}>Choose exact file…</Button
            >{/if}
        </div>{/each}
      <h3>Review uncertain audience</h3>
      {#each Object.values(review.collection.currentFiles).filter((entry) => entry.unresolvedReasons.length || !entry.classificationEvidence.length) as entry (entry.path)}<label
          >{entry.path}<select
            disabled={busy}
            bind:value={audiences[entry.path]}
            onchange={() => (dirty = true)}
            ><option value="">Unreviewed</option><option value="server">Server only</option><option
              value="both">Server and players</option
            ><option value="players">Players only</option></select
          ></label
        >{/each}
      <Button disabled={busy} onclick={() => void submit()}>Refresh comparison</Button><Button
        disabled={busy || dirty || review.missing.length > 0}
        onclick={() => void apply()}>Apply reviewed {mode}</Button
      >
    {/if}
  </div>
</Sheet>
{#if pendingSource}<StagedUploadSheet
    api={connection}
    purpose="modpack-archive"
    source={pendingSource}
    completionStage="installing"
    onComplete={uploaded}
    onClose={() => (pendingSource = undefined)}
  />{/if}
{#if browse}<PackCatalogSheet
    api={connection}
    {javaFlavor}
    {minecraftVersion}
    onClose={() => (browse = false)}
    onSelected={(id) => void catalogSelected(id)}
  />{/if}

<style>
  .content {
    display: grid;
    gap: 12px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
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
    font-weight: 500;
    font-size: 13px;
  }
  strong {
    font-weight: 500;
    font-size: 13px;
    overflow-wrap: anywhere;
  }
  .row {
    display: grid;
    gap: 6px;
    padding: 10px 0;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  select {
    color: var(--msc2-text-primary);
    background: var(--msc2-tier-content);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 6px;
    padding: 6px 8px;
  }
  .error {
    color: var(--msc2-status-error);
  }
</style>
