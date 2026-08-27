<script lang="ts">
  // Ports WorldSlotsView.swift's WorldSlotCard (DetailsWorldsTabView.swift's
  // own copy adds a Convert action) -- Realms-style thumbnail, Active badge,
  // size badge, Activate/Convert/Rename/Delete actions. Selecting the card
  // (not an action) shows its backups below, matching the oracle's
  // tap-to-select behavior. MSC 1 offers thumbnail-setting only via a
  // right-click "Set Thumbnail…" context-menu item; this app hasn't
  // established a context-menu pattern anywhere else, so it's a small
  // always-visible overlay button instead -- same capability (P12.4b's
  // real, staged-upload-backed POST /v1/worlds/{slotId}/thumbnail), more
  // discoverable affordance.
  // P12.4k adds Duplicate (ServerEditorWorldTab.swift, moved here per that
  // step's design reversal): the backend names the copy "{name} copy" and
  // takes no name argument at all, so it's a plain inline confirm like
  // Activate/Delete rather than a name-entry sheet -- rename the copy
  // afterward with the existing Rename action if wanted.
  // Cameron's own follow-up call: a persistent 5-button grid per card read
  // as cluttered. Actions collapse into the same anchored-Menu pattern
  // ComponentsSection.svelte's addon rows and ManageSheet.svelte's server
  // rows already use (a small "more actions" trigger opens a floating list;
  // the destructive item is styled, not separately colored). The Menu
  // itself is owned by the parent (WorldsSection.svelte), same as those two
  // -- one shared overlay instance, not one per card. Selecting a card (for
  // the Backups panel below) is unchanged, just recolored to the same blue
  // `--msc2-selection` token those rows use for their own selected state,
  // instead of a plain brighter-white border.
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
  export let busy = false;
  /** Which inline confirmation (P12.3g's expand-in-place pattern, not a
   *  modal) is open for this card, if any. Owned by the parent so only one
   *  card confirms at a time. */
  export let confirming: 'activate' | 'delete' | 'duplicate' | undefined = undefined;
  export let onSelect: () => void;
  export let onOpenMenu: (event: MouseEvent) => void;
  export let onConfirmActivate: () => void;
  export let onConfirmDuplicate: () => void;
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

    <div class="info-row">
      <button type="button" class="info" onclick={onSelect} aria-pressed={selected}>
        <span class="name">{slot.name}</span>
        <span class="saved">Saved {dateLabel(slot.createdAt)}</span>
        {#if seed}<span class="seed">Seed {seed}</span>{/if}
      </button>
      {#if !confirming}
        <Button variant="ghost-icon" size="sm" label="World actions" onclick={onOpenMenu}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <circle cx="12" cy="5" r="1.6" fill="currentColor" />
            <circle cx="12" cy="12" r="1.6" fill="currentColor" />
            <circle cx="12" cy="19" r="1.6" fill="currentColor" />
          </svg>
        </Button>
      {/if}
    </div>

    {#if confirming === 'activate'}
      <div class="confirm-block">
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
      </div>
    {:else if confirming === 'duplicate'}
      <div class="confirm-block">
        <p class="confirm-message">Creates a copy of "{slot.name}" named "{slot.name} copy".</p>
        <div class="actions-row">
          <Button size="sm" variant="secondary" onclick={onCancelConfirm}>Cancel</Button>
          <Button size="sm" variant="primary" disabled={busy} onclick={onConfirmDuplicate}>
            Duplicate Slot
          </Button>
        </div>
      </div>
    {:else if confirming === 'delete'}
      <div class="confirm-block">
        <p class="confirm-message">This permanently removes "{slot.name}". It can't be undone.</p>
        <div class="actions-row">
          <Button size="sm" variant="secondary" onclick={onCancelConfirm}>Cancel</Button>
          <Button size="sm" variant="destructive" disabled={busy} onclick={onConfirmDelete}>
            Delete Slot
          </Button>
        </div>
      </div>
    {/if}
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
    background: rgba(59, 130, 246, 0.06);
    box-shadow: inset 0 0 0 1.5px var(--msc2-selection);
  }
  .thumb-wrap {
    position: relative;
  }
  .thumb-area {
    display: block;
    box-sizing: border-box;
    width: 100%;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font: inherit;
    color: inherit;
    /* WebKit keeps a native <button> sizing to its own content even with
       display:flex set, so a flex child asking for width:100% (.thumb
       below) can get ignored -- Chromium doesn't have this quirk, which is
       why this only ever showed up in the real Tauri/WKWebView window,
       never in a Chromium-based check. */
    appearance: none;
    -webkit-appearance: none;
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
    width: 100%;
    height: 100px;
    box-sizing: border-box;
    flex-shrink: 0;
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
  .info-row {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 10px 10px 10px 12px;
  }
  .info {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
    gap: 2px;
    text-align: left;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font: inherit;
    color: inherit;
    appearance: none;
    -webkit-appearance: none;
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
  .confirm-block {
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
  .actions-row > :global(.btn) {
    flex: 1;
    min-width: 0;
  }
</style>
