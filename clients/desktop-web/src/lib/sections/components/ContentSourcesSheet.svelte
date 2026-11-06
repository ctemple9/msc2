<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import ContentManualDownloadSheet from './ContentManualDownloadSheet.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage } from '../shared/types';
  import { pollOperation } from './model';

  export let api: ScreenApi;
  export let serverId: string;
  export let onClose: () => void;
  export let onChanged: () => void;
  const connection = api.snapshot?.() ?? api;
  const root = `/v1/servers/${encodeURIComponent(serverId)}/content`;
  let disposed = false;
  let busy = false;
  let error = '';
  let status = '';
  let projectHints: Record<string, string> = {};
  let identities: Schema['ContentIdentificationDTO'] | undefined;
  let updates: Schema['ContentUpdatesDTO'] | undefined;
  let review: Schema['ContentInstallReviewDTO'] | undefined;
  let selected: Schema['ContentUpdateDTO'] | undefined;
  let manual: { project: Pick<Schema['CatalogItemDTO'], 'projectId' | 'title'> } | undefined;
  const current = () => !disposed && connection.isCurrent?.() !== false;
  async function refresh(): Promise<void> {
    if (busy || !current()) return;
    busy = true;
    error = '';
    review = undefined;
    selected = undefined;
    try {
      const caps = await connection.get<Schema['CapabilitiesDTO']>('/v1/capabilities');
      if (!current()) return;
      if (!caps.contentCatalog?.sourceIdentification?.includes('curseforge') || !caps.contentCatalog?.reviewedUpdates?.includes('curseforge')) throw new Error('Update this agent to identify and update CurseForge content.');
      const found = await connection.get<Schema['ContentIdentificationDTO']>(`${root}/identify`);
      if (!current()) return;
      identities = found;
      const available = await connection.get<Schema['ContentUpdatesDTO']>(`${root}/updates`);
      if (current()) updates = available;
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) busy = false;
    }
  }
  async function link(
    item: Schema['ContentIdentityDTO'],
    candidate?: Schema['ContentSourceDTO'],
  ): Promise<void> {
    if (busy || !item.sha512 || !current()) return;
    busy = true;
    error = '';
    let saved = false;
    try {
      const projectId = candidate ? Number(candidate.projectId) : undefined;
      const versionId = candidate ? Number(candidate.versionId) : undefined;
      if (
        candidate &&
        (![projectId, versionId].every((id) => Number.isSafeInteger(id) && Number(id) > 0) ||
          candidate.provider !== 'curseforge')
      )
        throw new Error(
          'Only exact CurseForge identities can be linked here. Existing Modrinth identities are preserved.',
        );
      await connection.post(`${root}/source`, {
        path: item.path,
        sha512: item.sha512,
        projectId,
        versionId,
      });
      if (current()) {
        saved = true;
        status = candidate ? 'Exact source saved.' : 'Source removed.';
        onChanged();
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        if (saved) await refresh();
      }
    }
  }
  async function saveHint(item: Schema['ContentIdentityDTO']): Promise<void> {
    const projectId = Number(projectHints[item.path]);
    if (busy || !item.sha512 || !current()) return;
    if (!Number.isSafeInteger(projectId) || projectId <= 0) {
      error = 'Enter a positive CurseForge project ID.';
      return;
    }
    busy = true;
    error = '';
    let saved = false;
    try {
      await connection.post(`${root}/source`, { path: item.path, sha512: item.sha512, projectId });
      if (current()) {
        saved = true;
        status = 'Project hint saved. It is not an exact release match.';
        onChanged();
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        if (saved) await refresh();
      }
    }
  }
  async function prepare(item: Schema['ContentUpdateDTO']): Promise<void> {
    const identity = identities?.items.find((value) => value.path === item.path);
    if (busy || !identity?.sha512 || !item.availableVersionId || !current()) return;
    busy = true;
    error = '';
    review = undefined;
    selected = undefined;
    try {
      const projectId = Number(item.projectId),
        versionId = Number(item.availableVersionId);
      if (![projectId, versionId].every((id) => Number.isSafeInteger(id) && id > 0))
        throw new Error('Invalid provider release identity.');
      const result = await connection.post<Schema['ContentInstallReviewDTO']>(
        `${root}/update-review`,
        { path: item.path, sha512: identity.sha512, projectId, versionId },
      );
      if (current()) {
        review = result;
        selected = item;
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) busy = false;
    }
  }
  async function apply(): Promise<void> {
    if (!review || !selected || busy || !current()) return;
    busy = true;
    error = '';
    let applied = false;
    try {
      if (review.files.some((file) => file.manualDownloadRequired)) {
        const project = await connection.get<Schema['ProviderCatalogProjectDTO']>(
          `/v1/catalog/curseforge/projects/${encodeURIComponent(selected.projectId)}`,
        );
        if (current())
          manual = { project: { projectId: selected.projectId, title: project.project.title } };
        return;
      }
      const accepted = await connection.post<{ operationId: string }>(`${root}/install`, {
        reviewId: review.reviewId,
      });
      const operation = await pollOperation(connection, accepted.operationId, (value) => {
        if (!current()) throw new Error('Selected host changed.');
        status = value.statusLine ?? 'Updating…';
      });
      if (operation?.state !== 'succeeded')
        throw new Error(operation?.error?.message ?? 'Update did not complete.');
      if (current()) {
        applied = true;
        status = 'Updated — restart the server to apply.';
        onChanged();
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        if (applied) await refresh();
      }
    }
  }
  onMount(() => void refresh());
  onDestroy(() => {
    disposed = true;
  });
</script>

<Sheet title="Content sources and updates" size="lg" {onClose}>
  <div class="content">
    <p>
      Identify installed files by their exact bytes. Choose a matching source before checking its
      updates. Existing Modrinth sources keep their normal update controls.
    </p>
    <Button disabled={busy} onclick={() => void refresh()}
      >{busy ? 'Checking…' : 'Refresh sources and updates'}</Button
    >
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if status}<p role="status">{status}</p>{/if}
    {#if identities?.note}<p>{identities.note}</p>{/if}
    {#each identities?.items ?? [] as item (item.path)}
      <div class="row">
        <strong>{item.path}</strong>
        <span
          >{item.source
            ? `${item.source.provider} · ${item.source.projectId} · ${item.source.versionId}`
            : item.status}</span
        >
        {#if item.note}<p>{item.note}</p>{/if}
        {#each item.candidates.filter((candidate) => candidate.provider === 'curseforge') as candidate}
          <Button disabled={busy || !item.sha512} onclick={() => void link(item, candidate)}
            >Link CurseForge {candidate.projectId} / {candidate.versionId}</Button
          >
        {/each}
        {#if !item.source}
          <label
            >CurseForge project ID (hint only)
            <input
              inputmode="numeric"
              bind:value={projectHints[item.path]}
              disabled={busy}
              placeholder="Project ID from CurseForge"
            />
          </label>
          <Button
            disabled={busy || !item.sha512 || !projectHints[item.path]}
            onclick={() => void saveHint(item)}>Save project hint</Button
          >
        {/if}
        {#if item.source?.provider === 'curseforge'}<Button
            disabled={busy}
            onclick={() => void link(item)}>Remove source</Button
          >{/if}
      </div>
    {/each}
    <h3>CurseForge updates</h3>
    {#if updates?.note}<p>{updates.note}</p>{/if}
    {#each updates?.items.filter((item) => item.provider === 'curseforge') ?? [] as item (item.path)}
      <div class="row">
        <strong>{item.path}</strong><span
          >{item.label ?? item.availableVersionId ?? 'No compatible update found'}</span
        >
        {#if item.note}<p>{item.note}</p>{/if}
        {#if item.availableVersionId}<Button disabled={busy} onclick={() => void prepare(item)}
            >Review update</Button
          >{/if}
      </div>
    {/each}
    {#if review && selected}
      <h3>Update {selected.path}</h3>
      <p>
        The server must be stopped. This review is bound to the installed checksum; changed files
        require a fresh review.
      </p>
      {#each review.files as file}<p>
          {file.fileName} · {file.versionId}{file.manualDownloadRequired
            ? ' · manual download required'
            : ''}
        </p>{/each}
      {#each review.warnings as warning}<p>{warning}</p>{/each}
      <Button disabled={busy} onclick={() => void apply()}>Apply reviewed update</Button>
    {/if}
  </div>
</Sheet>
{#if manual && review}
  <ContentManualDownloadSheet
    api={connection}
    {serverId}
    project={manual.project}
    {review}
    kind={selected?.path.startsWith('plugins/') ? 'plugin' : 'mod'}
    onClose={() => {
      manual = undefined;
    }}
    onInstalled={() => {
      manual = undefined;
      onChanged();
      void refresh();
    }}
  />
{/if}

<style>
  .content {
    display: grid;
    gap: 12px;
  }
  p {
    margin: 0;
    color: var(--msc2-text-secondary);
    font-size: 13px;
  }
  h3 {
    margin: 8px 0 0;
    font-size: 13px;
  }
  .row {
    display: grid;
    justify-items: start;
    gap: 6px;
    padding: 10px 0;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .row strong {
    overflow-wrap: anywhere;
    font-size: 13px;
  }
  .row span {
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  label {
    display: grid;
    gap: 4px;
    font-size: 12px;
  }
  input {
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
