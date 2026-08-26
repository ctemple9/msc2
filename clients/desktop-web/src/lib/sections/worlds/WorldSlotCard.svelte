<script lang="ts">
  // Ports WorldSlotsView.swift's WorldSlotCard (DetailsWorldsTabView.swift's
  // own copy adds a Convert action) -- Realms-style thumbnail, Active badge,
  // size badge, Activate/Convert/Rename/Delete actions. Selecting the card
  // (not an action button) shows its backups below, matching the oracle's
  // tap-to-select behavior. MSC 1 offers this only via a right-click
  // "Set Thumbnail…" context-menu item; this app hasn't established a
  // context-menu pattern anywhere else, so it's a small always-visible
  // overlay button instead -- same capability (P12.4b's real, staged-upload-
  // backed POST /v1/worlds/{slotId}/thumbnail), more discoverable affordance.
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { bytesLabel, dateLabel, mutate } from '../shared/types';
  import { getPlatform } from '../../platform';
  import { placeholderHue, slotThumbnailUrl, worldPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let slot: Schema['WorldSlotDTO'];
  export let selected = false;
  export let serverRunning = false;
  export let busy = false;
  /** Which inline confirmation (P12.3g's expand-in-place pattern, not a
   *  modal) is open for this card, if any. Owned by the parent so only one
   *  card confirms at a time. */
  export let confirming: 'activate' | 'delete' | undefined = undefined;
  export let onSelect: () => void;
  export let onRequestActivate: () => void;
  export let onConfirmActivate: () => void;
  export let onConvert: () => void;
  export let onRename: () => void;
  export let onRequestDelete: () => void;
  export let onConfirmDelete: () => void;
  export let onCancelConfirm: () => void;
  export let onThumbnailUpdated: () => void;

  let thumbnailBusy = false;
  let thumbnailError: string | undefined;
  let fileInput: HTMLInputElement;

  $: thumbnail = slotThumbnailUrl(slot);
  $: hue = placeholderHue(slot.name || slot.id);
  $: seed = slot.worldSeed?.trim();

  async function setThumbnail(): Promise<void> {
    if (!api?.upload) return;
    const picked = await (
      await getPlatform()
    ).pickFile(
      { label: 'Choose an image', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] },
      () => browseBrowserFile(),
    );
    if (!picked) return;
    thumbnailBusy = true;
    thumbnailError = undefined;
    try {
      const staged = await api.upload('world-thumbnail', picked.bytes);
      await mutate(api, worldPaths.thumbnail(slot.id), { stagedUploadId: staged.stagedUploadId });
      onThumbnailUpdated();
    } catch (error) {
      thumbnailError = error instanceof Error ? error.message : 'Failed to set thumbnail.';
    } finally {
      thumbnailBusy = false;
    }
  }

  function browseBrowserFile(): Promise<{ name: string; bytes: Uint8Array } | null> {
    return new Promise((resolve) => {
      fileInput.addEventListener(
        'change',
        async () => {
          const browserFile = fileInput.files?.[0];
          resolve(
            browserFile
              ? { name: browserFile.name, bytes: new Uint8Array(await browserFile.arrayBuffer()) }
              : null,
          );
        },
        { once: true },
      );
      fileInput.click();
    });
  }
</script>

<Card padding="0">
  <div class="slot" class:selected>
    <div class="thumb-wrap">
      <button type="button" class="thumb-area" onclick={onSelect} aria-pressed={selected}>
        <div
          class="thumb"
          style={thumbnail
            ? `background-image: url(${thumbnail});`
            : `background: linear-gradient(160deg, hsl(${hue} 40% 42%), hsl(${(hue + 30) % 360} 45% 22%));`}
        >
          {#if !thumbnail}<Icon name="world" size={26} />{/if}
          {#if slot.zipSizeBytes !== undefined}
            <span class="size-badge">{bytesLabel(slot.zipSizeBytes)}</span>
          {/if}
          {#if slot.isActive}<span class="active-badge">Active</span>{/if}
        </div>
        <div class="info">
          <span class="name">{slot.name}</span>
          <span class="saved">Saved {dateLabel(slot.createdAt)}</span>
          {#if seed}<span class="seed">Seed {seed}</span>{/if}
        </div>
      </button>
      {#if api?.upload}
        <input
          bind:this={fileInput}
          type="file"
          accept="image/*"
          class="hidden-input"
          tabindex="-1"
        />
        <button
          type="button"
          class="thumb-edit"
          disabled={thumbnailBusy}
          onclick={(event) => {
            event.stopPropagation();
            void setThumbnail();
          }}
        >
          {thumbnailBusy ? 'Setting…' : 'Set Thumbnail'}
        </button>
      {/if}
    </div>
    {#if thumbnailError}<p class="thumbnail-error">{thumbnailError}</p>{/if}

    <div class="actions">
      {#if confirming === 'activate'}
        <p class="confirm-message">
          The current world is backed up automatically before the swap. This can't be undone without
          restoring from that backup.
        </p>
        <div class="actions-row">
          <Button size="sm" variant="secondary" onclick={onCancelConfirm}>Cancel</Button>
          <Button size="sm" variant="primary" disabled={busy} onclick={onConfirmActivate}>
            Activate Slot
          </Button>
        </div>
      {:else if confirming === 'delete'}
        <p class="confirm-message">This permanently removes "{slot.name}". It can't be undone.</p>
        <div class="actions-row">
          <Button size="sm" variant="secondary" onclick={onCancelConfirm}>Cancel</Button>
          <Button size="sm" variant="destructive" disabled={busy} onclick={onConfirmDelete}>
            Delete Slot
          </Button>
        </div>
      {:else}
        <div class="actions-row">
          <span
            class="hint"
            title={serverRunning ? 'Stop the server before switching worlds' : 'Load this world'}
          >
            <Button
              size="sm"
              variant="primary"
              disabled={busy || serverRunning || slot.isActive}
              onclick={onRequestActivate}
            >
              Activate
            </Button>
          </span>
          <span
            class="hint"
            title={serverRunning
              ? 'Stop the server before converting'
              : 'Convert this world to another edition'}
          >
            <Button
              size="sm"
              variant="secondary"
              disabled={busy || serverRunning}
              onclick={onConvert}
            >
              Convert
            </Button>
          </span>
        </div>
        <div class="actions-row">
          <Button size="sm" variant="secondary" disabled={busy} onclick={onRename}>Rename</Button>
          <span
            class="hint"
            title={slot.isActive
              ? 'Activate a different slot before deleting this one'
              : 'Delete this slot'}
          >
            <Button
              size="sm"
              variant="destructive"
              disabled={busy || slot.isActive}
              onclick={onRequestDelete}
            >
              Delete
            </Button>
          </span>
        </div>
      {/if}
    </div>
  </div>
</Card>

<style>
  .slot {
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid transparent;
  }
  .slot.selected {
    border-color: rgba(255, 255, 255, 0.24);
  }
  .thumb-wrap {
    position: relative;
  }
  .thumb-area {
    display: flex;
    flex-direction: column;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font: inherit;
    color: inherit;
  }
  .hidden-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    overflow: hidden;
  }
  .thumb-edit {
    position: absolute;
    top: 8px;
    left: 8px;
    font-size: 9px;
    font-weight: 600;
    color: #fff;
    background: rgba(0, 0, 0, 0.45);
    border: none;
    border-radius: 5px;
    padding: 3px 6px;
    cursor: pointer;
  }
  .thumb-edit:hover:not(:disabled) {
    background: rgba(0, 0, 0, 0.6);
  }
  .thumb-edit:disabled {
    cursor: default;
    opacity: 0.7;
  }
  .thumbnail-error {
    margin: 6px 12px 0;
    font-size: 10px;
    color: var(--msc2-status-warn);
  }
  .thumb {
    position: relative;
    height: 100px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.75);
    background-size: cover;
    background-position: center;
  }
  .size-badge {
    position: absolute;
    left: 8px;
    bottom: 8px;
    font-size: 9px;
    font-weight: 600;
    color: #fff;
    background: rgba(0, 0, 0, 0.45);
    padding: 3px 6px;
    border-radius: 5px;
  }
  .active-badge {
    position: absolute;
    right: 8px;
    top: 8px;
    font-size: 9px;
    font-weight: 700;
    color: #0d2416;
    background: var(--msc2-status-ok);
    padding: 3px 7px;
    border-radius: 99px;
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px 8px;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
    color: var(--msc2-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .saved,
  .seed {
    font-size: 10px;
    color: var(--msc2-text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 12px 12px;
  }
  .confirm-message {
    margin: 0 0 4px;
    font-size: 11px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .actions-row {
    display: flex;
    gap: 6px;
  }
  .actions-row > :global(.btn),
  .actions-row .hint {
    flex: 1;
  }
  .hint {
    display: flex;
  }
  .hint :global(.btn) {
    width: 100%;
  }
</style>
