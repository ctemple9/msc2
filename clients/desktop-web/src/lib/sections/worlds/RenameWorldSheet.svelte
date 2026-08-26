<script lang="ts">
  // Ports the RenameSlotSheet private struct DetailsWorldsTabView.swift
  // defines inline -- metadata-only rename (WorldRenameRequestDTO), no files
  // touched. Not to be confused with the dead, never-wired RenameWorldView.swift
  // (renames the live world's on-disk folders) -- see the P12.4 rolling-plan
  // note on why that file and ReplaceWorldView.swift aren't ported here.
  import Sheet from '../../components/base/Sheet.svelte';
  import Field from '../../components/base/Field.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { worldPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let slotId: string;
  export let currentName: string;
  export let onClose: () => void;
  export let onRenamed: (updated: Schema['WorldSlotsResponseDTO']) => void;

  let name = currentName;
  let busy = false;
  let error: string | undefined;

  $: trimmed = name.trim();
  $: canRename = !busy && trimmed.length > 0 && trimmed !== currentName;

  async function submit(): Promise<void> {
    if (!canRename) return;
    busy = true;
    error = undefined;
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.rename, {
        slotId,
        name: trimmed,
      });
      if (result.updated) onRenamed(result.updated);
      onClose();
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'Failed to rename this slot.';
    } finally {
      busy = false;
    }
  }
</script>

<Sheet title="Rename Slot" size="sm" {onClose}>
  <form class="body" onsubmit={(event) => (event.preventDefault(), submit())}>
    <Field bind:value={name} placeholder="New name" />
    {#if error}<p class="error">{error}</p>{/if}
    <div class="footer">
      <Button variant="secondary" type="button" onclick={onClose}>Cancel</Button>
      <Button variant="primary" type="submit" disabled={!canRename}>Rename</Button>
    </div>
  </form>
</Sheet>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-status-warn);
    background: var(--msc2-status-warn-tint);
    border-radius: 8px;
    padding: 8px 10px;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
