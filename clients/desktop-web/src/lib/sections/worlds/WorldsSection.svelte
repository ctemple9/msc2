<script lang="ts">
  // Ports MSC 1 DetailsWorldsTabView.swift: World Slots grid (thumbnail,
  // Active badge, Activate/Convert/Rename/Delete) plus the Backups panel for
  // whichever slot is selected. Same shared-component pattern HomeSection/
  // PlayersOnlineSection use (D-003: one component for both Tauri and the
  // browser).
  //
  // "Rename/Replace/Convert/Repair" as the phase plan named them, checked
  // against the oracle rather than built from the names alone: MSC 1's own
  // RenameWorldView.swift and ReplaceWorldView.swift are never instantiated
  // anywhere in minecraft-server-controller (confirmed by grep) -- dead code
  // superseded by DetailsWorldsTabView's own inline RenameSlotSheet. This
  // tab ports exactly what DetailsWorldsTabView itself does: Create, Save
  // Current, Activate, the inline Rename sheet, Delete, Convert, and
  // (Bedrock) Repair -- plus, per P12.4k's design reversal (2026-08-27),
  // ServerEditorWorldTab.swift's Import ZIP / Replace World / Duplicate
  // Slot, moved here so every world action lives in one place instead of
  // splitting across this tab and a World-shaped Server Editor sub-tab.
  // Import ZIP and Replace World act globally (a new slot, the live world),
  // not on a selected card, so they sit in the header actions row next to
  // Save Current/Repair/Create; Duplicate acts on one slot, so it's an
  // inline card confirm like Activate/Delete (WorldSlotCard.svelte).
  import { onDestroy, onMount } from 'svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import WorldSlotCard from './WorldSlotCard.svelte';
  import BackupsPanel from './BackupsPanel.svelte';
  import CreateWorldSheet from './CreateWorldSheet.svelte';
  import RenameWorldSheet from './RenameWorldSheet.svelte';
  import WorldRepairSheet from './WorldRepairSheet.svelte';
  import WorldConversionWizard from './WorldConversionWizard.svelte';
  import ImportWorldZipSheet from './ImportWorldZipSheet.svelte';
  import ReplaceWorldSheet from './ReplaceWorldSheet.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, mutate } from '../shared/types';
  import {
    backupPaths,
    demoBackups,
    demoSlots,
    legacyImportName,
    pollOperation,
    serversPath,
    worldPaths,
  } from './model';

  export let api: ScreenProps['api'] = undefined;
  // Nothing in this screen needs host-scoped local storage (unlike
  // HomeSection's notes or PlayersOnlineSection's Bedrock session-log
  // cutoff) -- kept only so the section registry can pass it uniformly.
  export const hostId = 'local-agent';
  export let serverId = 'survival';

  let worlds: Schema['WorldSlotsResponseDTO'] = { slots: demoSlots, serverRunning: false };
  let backups: Schema['BackupItemDTO'][] = demoBackups;
  let servers: Schema['ServerDTO'][] = [];
  let backupConfig: Schema['BackupConfigResponseDTO'] | undefined;

  let selectedSlotId: string | undefined;
  let confirming: { slotId: string; kind: 'activate' | 'delete' | 'duplicate' } | undefined;
  let confirmingBackupDeleteId: string | undefined;
  let busy = false;
  let notice: string | undefined;

  let showCreate = false;
  let renamingSlot: Schema['WorldSlotDTO'] | undefined;
  let repairOpen = false;
  let convertingSlot: Schema['WorldSlotDTO'] | undefined;
  let importZipOpen = false;
  let replaceOpen = false;

  $: activeServer = servers.find((server) => server.id === serverId);
  $: isBedrock = activeServer?.serverType === 'bedrock';
  $: selectedSlot = worlds.slots.find((slot) => slot.id === selectedSlotId);

  async function loadWorlds(): Promise<void> {
    worlds = await call(api, worlds, worldPaths.list);
  }
  async function loadBackups(): Promise<void> {
    const response = await call<Schema['BackupsResponseDTO']>(api, { backups }, backupPaths.list);
    backups = response.backups;
  }
  async function loadServers(): Promise<void> {
    servers = await call(api, servers, serversPath);
  }
  async function loadBackupConfig(): Promise<void> {
    backupConfig = await call(api, backupConfig, backupPaths.config);
  }
  async function loadAll(): Promise<void> {
    await Promise.all([loadWorlds(), loadBackups(), loadServers(), loadBackupConfig()]);
  }

  function flash(message: string): void {
    notice = message;
  }

  function onWorldsCreatedOrRenamed(updated: Schema['WorldSlotsResponseDTO']): void {
    worlds = updated;
  }

  function requestActivate(slotId: string): void {
    confirming = { slotId, kind: 'activate' };
  }
  function requestDelete(slotId: string): void {
    confirming = { slotId, kind: 'delete' };
  }
  function requestDuplicate(slotId: string): void {
    confirming = { slotId, kind: 'duplicate' };
  }
  function cancelConfirm(): void {
    confirming = undefined;
  }

  async function confirmActivate(): Promise<void> {
    if (!confirming) return;
    const { slotId } = confirming;
    confirming = undefined;
    busy = true;
    flash('Activating world…');
    try {
      const result = await mutate<Schema['WorldActivateResultDTO']>(api, worldPaths.activate, {
        slotId,
      });
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId);
        flash(
          operation?.state === 'succeeded'
            ? 'World activated.'
            : (operation?.error?.message ?? 'Activation did not complete.'),
        );
      }
      await Promise.all([loadWorlds(), loadBackups()]);
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to activate this world.');
    } finally {
      busy = false;
    }
  }

  async function confirmDelete(): Promise<void> {
    if (!confirming) return;
    const { slotId } = confirming;
    confirming = undefined;
    busy = true;
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.delete, {
        slotId,
      });
      if (result.updated) worlds = result.updated;
      if (selectedSlotId === slotId) selectedSlotId = undefined;
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to delete this slot.');
    } finally {
      busy = false;
    }
  }

  async function confirmDuplicate(): Promise<void> {
    if (!confirming) return;
    const { slotId } = confirming;
    confirming = undefined;
    busy = true;
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.duplicate, {
        slotId,
      });
      if (result.updated) worlds = result.updated;
      flash('World slot duplicated.');
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to duplicate this slot.');
    } finally {
      busy = false;
    }
  }

  async function saveCurrentWorld(): Promise<void> {
    busy = true;
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.saveCurrent);
      if (result.updated) worlds = result.updated;
      flash('Active world saved.');
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to save the current world.');
    } finally {
      busy = false;
    }
  }

  function onRepaired(): void {
    flash('World repaired.');
    void Promise.all([loadWorlds(), loadBackups()]);
  }

  async function backUpNow(): Promise<void> {
    busy = true;
    flash('Backing up…');
    try {
      const result = await mutate<Schema['BackupNowResultDTO']>(api, backupPaths.now);
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId);
        flash(
          operation?.state === 'succeeded'
            ? 'Backup complete.'
            : (operation?.error?.message ?? 'Backup did not complete.'),
        );
      }
      await loadBackups();
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to start a backup.');
    } finally {
      busy = false;
    }
  }

  async function restoreBackup(backup: Schema['BackupItemDTO']): Promise<void> {
    busy = true;
    flash('Restoring…');
    try {
      const result = await mutate<Schema['BackupRestoreResultDTO']>(api, backupPaths.restore, {
        backupId: backup.id,
      });
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId);
        flash(
          operation?.state === 'succeeded'
            ? 'Restore complete.'
            : (operation?.error?.message ?? 'Restore did not complete.'),
        );
      }
      await Promise.all([loadWorlds(), loadBackups()]);
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to restore this backup.');
    } finally {
      busy = false;
    }
  }

  async function deleteBackupConfirmed(): Promise<void> {
    if (!confirmingBackupDeleteId) return;
    const backupId = confirmingBackupDeleteId;
    confirmingBackupDeleteId = undefined;
    busy = true;
    try {
      await mutate(api, backupPaths.delete, { backupId });
      await loadBackups();
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to delete this backup.');
    } finally {
      busy = false;
    }
  }

  async function importLegacyBackup(backup: Schema['BackupItemDTO']): Promise<void> {
    busy = true;
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.import, {
        name: legacyImportName(backup),
        backupId: backup.id,
      });
      if (result.updated) worlds = result.updated;
      flash(`Imported "${legacyImportName(backup)}" as a new slot.`);
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to import this backup as a slot.');
    } finally {
      busy = false;
    }
  }

  async function toggleAuto(enabled: boolean): Promise<void> {
    const result = await mutate<Schema['BackupConfigUpdateResultDTO']>(api, backupPaths.config, {
      autoBackupEnabled: enabled,
    });
    if (result.config) backupConfig = result.config;
  }
  async function changeInterval(minutes: number): Promise<void> {
    const result = await mutate<Schema['BackupConfigUpdateResultDTO']>(api, backupPaths.config, {
      autoBackupIntervalMinutes: minutes,
    });
    if (result.config) backupConfig = result.config;
  }

  let refreshTimer: ReturnType<typeof setInterval> | undefined;
  onMount(() => {
    void loadAll();
    refreshTimer = setInterval(() => void loadAll(), 8000);
  });
  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
  });
</script>

<div class="worlds">
  <section class="zone">
    <div class="section-header">
      <div class="overline">
        <Icon name="world" size={13} />
        <span class="msc2-type-overline">World Slots</span>
      </div>
      <div class="header-actions">
        <Button
          size="sm"
          variant="secondary"
          disabled={busy}
          onclick={() => void saveCurrentWorld()}
        >
          Save Current World
        </Button>
        <Button
          size="sm"
          variant="secondary"
          disabled={busy}
          onclick={() => (importZipOpen = true)}
        >
          Import ZIP…
        </Button>
        <Button size="sm" variant="secondary" disabled={busy} onclick={() => (replaceOpen = true)}>
          Replace World…
        </Button>
        {#if isBedrock}
          <Button
            size="sm"
            variant="secondary"
            disabled={busy || worlds.serverRunning}
            onclick={() => (repairOpen = true)}
          >
            Repair World
          </Button>
        {/if}
        <Button size="sm" variant="primary" disabled={busy} onclick={() => (showCreate = true)}>
          Create New World
        </Button>
      </div>
    </div>

    {#if notice}<p class="notice" role="status">{notice}</p>{/if}

    {#if worlds.slots.length === 0}
      <EmptyState
        title="No world slots yet"
        message="Create a new world slot, or save the current active world back into its slot."
      >
        <Icon name="world" size={26} slot="icon" />
        <div slot="action" class="empty-action">
          <Button size="sm" variant="primary" onclick={() => (showCreate = true)}>
            Create New World
          </Button>
        </div>
      </EmptyState>
    {:else}
      <div class="grid">
        {#each worlds.slots as slot (slot.id)}
          <WorldSlotCard
            {api}
            {slot}
            selected={selectedSlotId === slot.id}
            serverRunning={worlds.serverRunning}
            {busy}
            confirming={confirming?.slotId === slot.id ? confirming.kind : undefined}
            onSelect={() => (selectedSlotId = selectedSlotId === slot.id ? undefined : slot.id)}
            onRequestActivate={() => requestActivate(slot.id)}
            onConfirmActivate={() => void confirmActivate()}
            onConvert={() => (convertingSlot = slot)}
            onRename={() => (renamingSlot = slot)}
            onRequestDuplicate={() => requestDuplicate(slot.id)}
            onConfirmDuplicate={() => void confirmDuplicate()}
            onRequestDelete={() => requestDelete(slot.id)}
            onConfirmDelete={() => void confirmDelete()}
            onCancelConfirm={cancelConfirm}
            onThumbnailUpdated={() => void loadWorlds()}
          />
        {/each}
      </div>
    {/if}
  </section>

  <BackupsPanel
    {backups}
    slots={worlds.slots}
    {selectedSlot}
    activeSlotId={worlds.activeSlotId}
    serverRunning={worlds.serverRunning}
    {isBedrock}
    config={backupConfig}
    {busy}
    confirmingDeleteId={confirmingBackupDeleteId}
    onBackUpNow={() => void backUpNow()}
    onRestore={(backup) => void restoreBackup(backup)}
    onDeleteRequest={(id) => (confirmingBackupDeleteId = id)}
    onDeleteConfirm={() => void deleteBackupConfirmed()}
    onDeleteCancel={() => (confirmingBackupDeleteId = undefined)}
    onToggleAuto={(enabled) => void toggleAuto(enabled)}
    onIntervalChange={(minutes) => void changeInterval(minutes)}
    onImportLegacy={(backup) => void importLegacyBackup(backup)}
  />
</div>

{#if showCreate}
  <CreateWorldSheet
    {api}
    onClose={() => (showCreate = false)}
    onCreated={onWorldsCreatedOrRenamed}
  />
{/if}

{#if renamingSlot}
  <RenameWorldSheet
    {api}
    slotId={renamingSlot.id}
    currentName={renamingSlot.name}
    onClose={() => (renamingSlot = undefined)}
    onRenamed={onWorldsCreatedOrRenamed}
  />
{/if}

{#if repairOpen}
  <WorldRepairSheet
    {api}
    activeSlotId={worlds.activeSlotId}
    onClose={() => (repairOpen = false)}
    {onRepaired}
  />
{/if}

{#if importZipOpen}
  <ImportWorldZipSheet
    {api}
    onClose={() => (importZipOpen = false)}
    onImported={(updated) => {
      worlds = updated;
      flash('Imported ZIP as a new world slot.');
    }}
  />
{/if}

{#if replaceOpen}
  <ReplaceWorldSheet
    {api}
    serverRunning={worlds.serverRunning}
    onClose={() => (replaceOpen = false)}
    onReplaced={() => {
      flash('World replacement started.');
      void Promise.all([loadWorlds(), loadBackups()]);
    }}
  />
{/if}

{#if convertingSlot}
  <WorldConversionWizard
    {api}
    sourceSlot={convertingSlot}
    sourceServer={activeServer}
    sourceServerRunning={worlds.serverRunning}
    {servers}
    onClose={() => (convertingSlot = undefined)}
    onConverted={() => {
      flash('World conversion started.');
      void Promise.all([loadWorlds(), loadBackups()]);
    }}
  />
{/if}

<style>
  .worlds {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .notice {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 10px;
  }
  .empty-action {
    margin-top: 10px;
  }
</style>
