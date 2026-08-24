<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import ConfirmDialog from '../../components/ConfirmDialog.svelte';
  import TransferPanel from '../transfers/TransferPanel.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, bytesLabel, dateLabel, errorMessage, mutate } from '../shared/types';
  import { demoWorlds, worldPaths } from './model';

  export let api: ScreenProps['api'] = undefined;
  let worlds = demoWorlds;
  let activeSlotId = 'world-1';
  let newName = '';
  let seed = '';
  let rename = '';
  let stagedUploadId = '';
  let pendingDelete: string | null = null;
  let notice = '';

  onMount(async () => {
    const result = await call<Schema['WorldSlotsResponseDTO']>(
      api,
      { slots: worlds, activeSlotId, serverRunning: false },
      worldPaths.list,
    );
    worlds = result.slots;
    activeSlotId =
      result.activeSlotId ?? worlds.find((world) => world.isActive)?.id ?? activeSlotId;
  });

  async function action<T>(path: string, body?: unknown): Promise<T | undefined> {
    try {
      const result = await mutate<T>(api, path, body);
      notice = 'World operation accepted.';
      return result;
    } catch (error) {
      notice = errorMessage(error);
      return undefined;
    }
  }
  async function create(): Promise<void> {
    if (!newName.trim()) return;
    await action(worldPaths.create, { name: newName, seed: seed || undefined });
    newName = '';
  }
  async function activate(id: string): Promise<void> {
    await action(worldPaths.activate, { slotId: id });
    activeSlotId = id;
    worlds = worlds.map((world) => ({ ...world, isActive: world.id === id }));
  }
  async function deleteWorld(): Promise<void> {
    if (!pendingDelete) return;
    await action(worldPaths.delete, { slotId: pendingDelete });
    worlds = worlds.filter((world) => world.id !== pendingDelete);
    pendingDelete = null;
  }
  async function renameWorld(): Promise<void> {
    if (!rename.trim()) return;
    await action(worldPaths.rename, { slotId: activeSlotId, name: rename });
  }
  async function exportWorld(id: string): Promise<void> {
    const result = await action<Schema['WorldExportResultDTO']>(worldPaths.export, { slotId: id });
    if (result?.stagedDownloadId) notice = 'Export staged and ready to download.';
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="World slots"
    title="Worlds"
    description="Saved slots make activation, duplication, import, export, and risky live-world changes visible as separate workflows."
    status={`${worlds.length} slots`}
    statusTone="positive"
  />
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Saved worlds</h3>
      <span class="metric-label"
        >Active: {worlds.find((world) => world.id === activeSlotId)?.name ?? 'none'}</span
      >
    </div>
    <table class="data-table">
      <thead
        ><tr><th>Slot</th><th>Created</th><th>Size</th><th class="actions">Actions</th></tr></thead
      ><tbody
        >{#each worlds as world (world.id)}<tr
            ><td
              ><strong>{world.name}</strong>{#if world.isActive}<br /><span class="tag">Active</span
                >{/if}</td
            ><td>{dateLabel(world.createdAt)}</td><td>{bytesLabel(world.zipSizeBytes)}</td><td
              class="actions"
              ><div class="screen-actions">
                <ActionButton
                  label="Activate world"
                  disabled={world.isActive}
                  onclick={() => activate(world.id)}>Activate</ActionButton
                ><ActionButton
                  kind="quiet"
                  label="Export world"
                  onclick={() => exportWorld(world.id)}>Export</ActionButton
                ><ActionButton
                  kind="danger"
                  label="Delete world"
                  onclick={() => (pendingDelete = world.id)}>Delete</ActionButton
                >
              </div></td
            ></tr
          >{:else}<tr><td colspan="4" class="empty-row">No saved world slots.</td></tr
          >{/each}</tbody
      >
    </table>
  </section>
  <div class="screen-grid">
    <section class="screen-card">
      <h3>Create a slot</h3>
      <div class="form-grid" style="margin-top: .7rem">
        <div class="field">
          <label for="world-name">Name</label><input
            id="world-name"
            bind:value={newName}
            placeholder="New world"
          />
        </div>
        <div class="field">
          <label for="world-seed">Seed (optional)</label><input id="world-seed" bind:value={seed} />
        </div>
      </div>
      <div class="screen-actions" style="margin-top: .7rem">
        <ActionButton label="Create world" onclick={create}>Create</ActionButton>
      </div>
    </section>
    <section class="screen-card">
      <h3>Rename active slot</h3>
      <div class="inline-form" style="margin-top: .7rem">
        <div class="field">
          <label for="world-rename">New slot name</label><input
            id="world-rename"
            bind:value={rename}
          />
        </div>
        <ActionButton label="Rename" onclick={renameWorld}>Rename</ActionButton>
      </div>
    </section>
  </div>
  <section class="screen-card">
    <h3>Bounded transfer</h3>
    <p>
      Files are staged through an expiring upload token; the client never sends an arbitrary server
      path.
    </p>
    <TransferPanel
      {api}
      purpose="world-import"
      label="Import a world archive"
      onComplete={(id) => (stagedUploadId = id)}
    />{#if stagedUploadId}<div class="screen-actions" style="margin-top: .7rem">
        <ActionButton
          label="Import staged world"
          onclick={() => action(worldPaths.import, { name: 'Imported world', stagedUploadId })}
          >Import</ActionButton
        >
      </div>{/if}
  </section>
  <section class="screen-card warning">
    <h3>Live-world replacement</h3>
    <p>
      Replacing the active world creates a safety backup and may stop the server. Keep this control
      separate from ordinary slot activation.
    </p>
    <ActionButton
      kind="danger"
      label="Replace active world"
      onclick={() => action(worldPaths.replaceActive, { newLevelName: 'New active world' })}
      >Replace active world</ActionButton
    >
  </section>
  <ConfirmDialog
    open={pendingDelete !== null}
    title="Delete this saved world?"
    message="This removes the selected slot. The active world cannot be deleted while it is active."
    confirmLabel="Delete world"
    onConfirm={deleteWorld}
    onClose={() => (pendingDelete = null)}
  />
</div>
