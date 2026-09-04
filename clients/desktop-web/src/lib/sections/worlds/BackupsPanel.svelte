<script lang="ts">
  // Ports DetailsWorldsTabView.swift's backupsSection: day-grouped backups
  // for the selected slot, auto-backup toggle, Back Up Now, Restore/Delete
  // per row, and the Legacy/Unmatched Backups sub-list. Two honest,
  // documented divergences from the oracle, both forced by the real (frozen)
  // contract rather than chosen for convenience:
  //   1. POST /v1/backups/now has no slotId param -- it always backs up
  //      whichever slot is active, unlike MSC 1's per-selected-slot manual
  //      backup. Labeled plainly below rather than implying slot targeting.
  //   2. POST /v1/backups/restore 409s if the backup's slot isn't the
  //      currently active one (crates/msc-agent/src/routes/backups.rs) --
  //      MSC 1 could restore a backup into its owning slot even when that
  //      slot wasn't active. Restore is disabled with an explanation for a
  //      backup whose slot isn't active, rather than calling a route that
  //      would just reject it.
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import Select from '../../components/base/Select.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import type { Schema } from '../shared/types';
  import { bytesLabel, dateLabel } from '../shared/types';
  import {
    backupsForSlot,
    formatBackupDay,
    groupBackupsByDay,
    legacyBackupReason,
    legacyOrUnmatchedBackups,
  } from './model';

  export let backups: readonly Schema['BackupItemDTO'][] = [];
  export let slots: readonly Schema['WorldSlotDTO'][] = [];
  export let selectedSlot: Schema['WorldSlotDTO'] | undefined = undefined;
  export let activeSlotId: string | undefined = undefined;
  export let serverRunning = false;
  export let isBedrock = false;
  export let config: Schema['BackupConfigResponseDTO'] | undefined = undefined;
  export let busy = false;
  export let confirmingDeleteId: string | undefined = undefined;
  export let onBackUpNow: () => void;
  export let onRestore: (backup: Schema['BackupItemDTO']) => void;
  export let onDeleteRequest: (backupId: string) => void;
  export let onDeleteConfirm: () => void;
  export let onDeleteCancel: () => void;
  export let onToggleAuto: (enabled: boolean) => void;
  export let onIntervalChange: (minutes: number) => void;
  export let onImportLegacy: (backup: Schema['BackupItemDTO']) => void;

  $: slotBackups = backupsForSlot(backups, selectedSlot?.id);
  $: groups = groupBackupsByDay(slotBackups);
  $: legacy = legacyOrUnmatchedBackups(backups, slots);
  $: totalBytes = backups.reduce((sum, backup) => sum + (backup.fileSize ?? 0), 0);
  $: intervalOptions = (config?.intervalOptions ?? []).map((minutes) => ({
    value: String(minutes),
    label: minutes < 60 ? `Every ${minutes} min` : `Every ${minutes / 60} hr`,
  }));
</script>

<Card>
  <div class="header">
    <div class="overline">
      <span class="msc2-type-overline">Backups</span>
      {#if selectedSlot}<span class="slot-name">· {selectedSlot.name}</span>{/if}
    </div>
    <div class="header-actions">
      {#if config}
        <Toggle
          checked={config.autoBackupEnabled}
          label="Automatic backups"
          onchange={onToggleAuto}
        />
        <span class="auto-label">Auto</span>
        {#if config.autoBackupEnabled && intervalOptions.length > 0}
          <Select
            options={intervalOptions}
            value={String(config.autoBackupIntervalMinutes)}
            width="auto"
            onchange={(value) => onIntervalChange(Number(value))}
          />
        {/if}
      {/if}
      {#if totalBytes > 0}<span class="size">{bytesLabel(totalBytes)} total</span>{/if}
      <Button size="sm" variant="secondary" disabled={busy} onclick={onBackUpNow}>
        Back Up Now
      </Button>
    </div>
  </div>

  {#if !selectedSlot}
    <EmptyState
      title="Select a world slot to view its backups"
      message="Backups shown here stay attached to the selected slot."
    >
      <Icon name="folder" size={26} slot="icon" />
    </EmptyState>
  {:else if slotBackups.length === 0}
    <EmptyState
      title="No backups for this slot yet"
      message={'Use "Back Up Now" to create one for the active world.'}
    >
      <Icon name="folder" size={26} slot="icon" />
    </EmptyState>
  {:else}
    <div class="days">
      {#each groups as group (group.day)}
        <div class="day-header">{formatBackupDay(group.day)}</div>
        {#each group.items as backup (backup.id)}
          {@const canRestore = !isBedrock && backup.slotId === activeSlotId && !serverRunning}
          <div class="row">
            <span class="badge" class:auto={backup.isAutomatic}>
              {backup.isAutomatic ? 'Auto' : 'Manual'}
            </span>
            <span class="time">{dateLabel(backup.modificationDate)}</span>
            {#if backup.fileSize !== undefined}<span class="size-col"
                >{bytesLabel(backup.fileSize)}</span
              >{/if}
            <span class="spacer"></span>
            {#if confirmingDeleteId === backup.id}
              <span class="confirm">Delete this backup?</span>
              <Button size="sm" variant="destructive" disabled={busy} onclick={onDeleteConfirm}>
                Delete
              </Button>
              <Button size="sm" variant="secondary" onclick={onDeleteCancel}>Cancel</Button>
            {:else}
              <span
                title={isBedrock
                  ? 'Restoring a backup is not available for Bedrock servers'
                  : serverRunning
                    ? 'Stop the server before restoring a backup'
                    : backup.slotId !== activeSlotId
                      ? 'Restore is only available for the active world’s backups'
                      : 'Restore this backup'}
              >
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy || !canRestore}
                  onclick={() => onRestore(backup)}
                >
                  Restore
                </Button>
              </span>
              <Button
                size="sm"
                variant="destructive"
                disabled={busy}
                onclick={() => onDeleteRequest(backup.id)}
              >
                Delete
              </Button>
            {/if}
          </div>
        {/each}
      {/each}
    </div>
  {/if}

  {#if legacy.length > 0}
    <div class="legacy">
      <p class="msc2-type-overline">Legacy / unmatched backups</p>
      <p class="legacy-explain">
        These backups are still on disk but aren't attached to any current world slot.
      </p>
      {#each legacy as backup (backup.id)}
        <div class="row">
          <span class="badge" class:auto={backup.isAutomatic}>
            {backup.isAutomatic ? 'Auto' : 'Manual'}
          </span>
          <span class="reason">{legacyBackupReason(backup)}</span>
          <span class="spacer"></span>
          {#if backup.fileSize !== undefined}<span class="size-col"
              >{bytesLabel(backup.fileSize)}</span
            >{/if}
          <Button
            size="sm"
            variant="secondary"
            disabled={busy}
            onclick={() => onImportLegacy(backup)}
          >
            Import as Slot
          </Button>
          {#if confirmingDeleteId === backup.id}
            <Button size="sm" variant="destructive" disabled={busy} onclick={onDeleteConfirm}>
              Delete
            </Button>
            <Button size="sm" variant="secondary" onclick={onDeleteCancel}>Cancel</Button>
          {:else}
            <Button
              size="sm"
              variant="destructive"
              disabled={busy}
              onclick={() => onDeleteRequest(backup.id)}
            >
              Delete
            </Button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</Card>

<style>
  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .slot-name {
    font-size: 11px;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .auto-label {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .size {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .days {
    display: flex;
    flex-direction: column;
  }
  .day-header {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
    padding: 10px 0 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
    flex-wrap: wrap;
  }
  .badge {
    font-size: 9px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-secondary);
    flex-shrink: 0;
  }
  .badge.auto {
    background: var(--msc2-status-bedrock-tint);
    color: var(--msc2-status-bedrock);
  }
  .time {
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    color: var(--msc2-text-secondary);
  }
  .size-col {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .reason {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .spacer {
    flex: 1;
  }
  .confirm {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .legacy {
    margin-top: 16px;
    padding-top: 14px;
    border-top: 1px solid var(--msc2-hairline-faint);
  }
  .legacy-explain {
    margin: 4px 0 8px;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
</style>
