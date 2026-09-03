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
  //
  // Per-card actions (Set as Active/Convert/Rename/Duplicate/Delete) were a
  // persistent 5-button grid; Cameron's follow-up call collapsed that into
  // the same anchored `Menu` list ComponentsSection.svelte's addon rows and
  // ManageSheet.svelte's server rows already use, one shared overlay owned
  // here (`actionMenu`) rather than one per card.
  import { ApiError } from '../../api/client';
  import { onDestroy, onMount } from 'svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Menu from '../../components/base/Menu.svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import WorldSlotCard from './WorldSlotCard.svelte';
  import WorldSettingsForm from './WorldSettingsForm.svelte';
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
    diffWorldSettings,
    profileToWorldSettings,
    type WorldServerType,
    type WorldSettingsValues,
    type WorldSlotWithProfile,
    type WorldProfileUpdateResult,
  } from './model';

  export let api: ScreenProps['api'] = undefined;
  // Nothing in this screen needs host-scoped local storage (unlike
  // HomeSection's notes or PlayersOnlineSection's Bedrock session-log
  // cutoff) -- kept only so the section registry can pass it uniformly.
  export const hostId = 'local-agent';
  export let serverId = 'survival';
  export let active = true;

  let worlds: Schema['WorldSlotsResponseDTO'] = { slots: demoSlots, serverRunning: false };
  let backups: Schema['BackupItemDTO'][] = demoBackups;
  let servers: Schema['ServerDTO'][] = [];
  let backupConfig: Schema['BackupConfigResponseDTO'] | undefined;
  let profiles: Record<string, WorldSlotWithProfile> = {};

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
  let editingSlot: Schema['WorldSlotDTO'] | undefined;
  let editingProfile: WorldSlotWithProfile | undefined;
  let editingValues: WorldSettingsValues | undefined;
  let editingLoading = false;
  let editingBusy = false;
  let editingError: string | undefined;
  let editingNotice: string | undefined;
  let editingConfirmation: SafetyPrompt | undefined;
  /** The floating per-card action menu (Set as Active/Convert/Rename/
   *  Duplicate/Delete) -- one shared overlay instance owned here, same
   *  pattern as ComponentsSection.svelte's `addonMenu`/ManageSheet.svelte's
   *  `openMenuFor`, not one Menu per card. */
  let actionMenu: { slot: Schema['WorldSlotDTO']; x: number; y: number } | undefined;
  let headerMenu: { x: number; y: number } | undefined;

  $: activeServer = servers.find((server) => server.id === serverId);
  $: isBedrock = activeServer?.serverType === 'bedrock';
  $: selectedSlot = worlds.slots.find((slot) => slot.id === selectedSlotId);

  type SafetyPrompt = {
    token: string;
    title: string;
    message: string;
  };

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

  async function loadProfiles(): Promise<void> {
    if (!api) return;
    const entries = await Promise.all(
      worlds.slots.map(async (slot) => {
        try {
          return [
            slot.id,
            await api.get<WorldSlotWithProfile>(worldPaths.profile(slot.id)),
          ] as const;
        } catch {
          return undefined;
        }
      }),
    );
    profiles = Object.fromEntries(
      entries.filter((entry): entry is [string, WorldSlotWithProfile] => Boolean(entry)),
    );
  }
  async function loadBackupConfig(): Promise<void> {
    backupConfig = await call(api, backupConfig, backupPaths.config);
  }
  async function loadAll(): Promise<void> {
    await Promise.all([loadWorlds(), loadBackups(), loadServers(), loadBackupConfig()]);
    await loadProfiles();
  }

  function flash(message: string): void {
    notice = message;
  }

  function onWorldsCreatedOrRenamed(updated: Schema['WorldSlotsResponseDTO']): void {
    worlds = updated;
    void loadProfiles();
  }

  function safetyPrompt(caught: unknown): SafetyPrompt | undefined {
    if (!(caught instanceof ApiError) || caught.error.code !== 'confirmation_required') return;
    const raw = (caught.error.details as Record<string, unknown> | null | undefined)?.confirmation;
    if (!raw || typeof raw !== 'object') return;
    const prompt = raw as Record<string, unknown>;
    if (
      typeof prompt.acknowledgement !== 'string' ||
      typeof prompt.title !== 'string' ||
      typeof prompt.message !== 'string'
    ) {
      return;
    }
    return {
      token: prompt.acknowledgement,
      title: prompt.title,
      message: prompt.message,
    };
  }

  function worldServerType(): WorldServerType {
    return activeServer?.serverType === 'bedrock' ? 'bedrock' : 'java';
  }

  async function openWorldSettings(slot: Schema['WorldSlotDTO']): Promise<void> {
    editingSlot = slot;
    editingProfile = undefined;
    editingValues = undefined;
    editingLoading = true;
    editingBusy = false;
    editingError = undefined;
    editingNotice = undefined;
    editingConfirmation = undefined;
    try {
      const detail =
        profiles[slot.id] ?? (await api?.get<WorldSlotWithProfile>(worldPaths.profile(slot.id)));
      if (!detail) throw new Error('Connect to an agent to load world settings.');
      if (editingSlot?.id !== slot.id) return;
      editingProfile = detail;
      editingValues = profileToWorldSettings(detail.profile, detail.slot);
    } catch (error) {
      if (editingSlot?.id === slot.id) {
        editingError = error instanceof Error ? error.message : 'Failed to load world settings.';
      }
    } finally {
      if (editingSlot?.id === slot.id) editingLoading = false;
    }
  }

  function closeWorldSettings(): void {
    if (editingBusy) return;
    editingSlot = undefined;
    editingProfile = undefined;
    editingValues = undefined;
    editingError = undefined;
    editingNotice = undefined;
    editingConfirmation = undefined;
  }

  function updateEditingValues(next: WorldSettingsValues): void {
    editingValues = next;
  }

  function profileStatusLabel(value: string): string {
    if (value === 'pending_restart') return 'Saved. Restart the server to apply these settings.';
    if (value === 'blocked') return 'Saved, but the active runtime could not apply these settings.';
    return 'World settings saved and applied.';
  }

  async function saveWorldSettings(confirmationToken?: string): Promise<void> {
    if (!editingSlot || !editingProfile || !editingValues || editingBusy) return;
    const changes = diffWorldSettings(
      profileToWorldSettings(editingProfile.profile, editingProfile.slot),
      editingValues,
      worldServerType(),
    );
    if (Object.keys(changes).length === 0) {
      editingNotice = 'No changes to save.';
      return;
    }
    editingBusy = true;
    editingError = undefined;
    editingNotice = undefined;
    editingConfirmation = undefined;
    try {
      const result = await mutate<WorldProfileUpdateResult>(
        api,
        worldPaths.profile(editingSlot.id),
        { changes, ...(confirmationToken ? { confirmation: confirmationToken } : {}) },
      );
      editingProfile = result.slot;
      editingValues = profileToWorldSettings(result.slot.profile, result.slot.slot);
      profiles = { ...profiles, [editingSlot.id]: result.slot };
      worlds = {
        ...worlds,
        slots: worlds.slots.map((slot) =>
          slot.id === result.slot.slot.id ? result.slot.slot : slot,
        ),
      };
      editingNotice = profileStatusLabel(result.status);
    } catch (caught) {
      editingConfirmation = safetyPrompt(caught);
      if (!editingConfirmation) {
        editingError = caught instanceof Error ? caught.message : 'Failed to save world settings.';
      }
    } finally {
      editingBusy = false;
    }
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

  function openActionMenu(event: MouseEvent, slot: Schema['WorldSlotDTO']): void {
    actionMenu = { slot, x: event.clientX, y: event.clientY };
  }

  function openHeaderMenu(event: MouseEvent): void {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    headerMenu = { x: rect.left, y: rect.bottom + 4 };
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
  let mounted = false;
  onMount(() => {
    mounted = true;
    if (active) {
      void loadAll();
      refreshTimer = setInterval(() => void loadAll(), 8000);
    }
  });
  onDestroy(() => {
    mounted = false;
    if (refreshTimer) clearInterval(refreshTimer);
    refreshTimer = undefined;
  });

  $: if (mounted && active && refreshTimer === undefined) {
    void loadAll();
    refreshTimer = setInterval(() => void loadAll(), 8000);
  }
  $: if (mounted && !active && refreshTimer !== undefined) {
    clearInterval(refreshTimer);
    refreshTimer = undefined;
  }
</script>

<div class="worlds">
  <section class="zone">
    <div class="section-header">
      <div class="overline">
        <Icon name="world" size={13} />
        <span class="msc2-type-overline">World Slots</span>
      </div>
      <div class="header-actions">
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
        <Button size="sm" variant="secondary" disabled={busy} onclick={openHeaderMenu}>…</Button>
      </div>
    </div>

    <p class="ownership">
      World settings travel with each slot. Server settings—ports, player limits, access, MOTD,
      runtime, and network helpers—apply to every world and stay in Settings.
    </p>

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
            profile={profiles[slot.id]?.profile}
            selected={selectedSlotId === slot.id}
            {busy}
            confirming={confirming?.slotId === slot.id ? confirming.kind : undefined}
            onSelect={() => (selectedSlotId = selectedSlotId === slot.id ? undefined : slot.id)}
            onOpenMenu={(event) => openActionMenu(event, slot)}
            onConfirmActivate={() => void confirmActivate()}
            onConfirmDuplicate={() => void confirmDuplicate()}
            onConfirmDelete={() => void confirmDelete()}
            onCancelConfirm={cancelConfirm}
            onThumbnailUpdated={() => void loadWorlds()}
          />
        {/each}
      </div>
    {/if}
  </section>

  {#if headerMenu}
    <Menu
      x={headerMenu.x}
      y={headerMenu.y}
      onClose={() => (headerMenu = undefined)}
      items={[
        { label: 'Save Current World', disabled: busy, onSelect: () => void saveCurrentWorld() },
        { label: 'Import ZIP…', disabled: busy, onSelect: () => (importZipOpen = true) },
        { label: 'Replace World…', disabled: busy, onSelect: () => (replaceOpen = true) },
      ]}
    />
  {/if}

  {#if actionMenu}
    {@const menuSlot = actionMenu.slot}
    <Menu
      x={actionMenu.x}
      y={actionMenu.y}
      onClose={() => (actionMenu = undefined)}
      items={[
        ...(menuSlot.isActive
          ? []
          : [
              {
                label: 'Set as Active',
                disabled: worlds.serverRunning,
                onSelect: () => requestActivate(menuSlot.id),
              },
            ]),
        {
          label: 'Convert World…',
          disabled: worlds.serverRunning,
          onSelect: () => (convertingSlot = menuSlot),
        },
        { label: 'World Settings…', onSelect: () => void openWorldSettings(menuSlot) },
        { label: 'Rename…', onSelect: () => (renamingSlot = menuSlot) },
        { label: 'Duplicate…', onSelect: () => requestDuplicate(menuSlot.id) },
        {
          label: 'Delete This World Slot',
          tone: 'destructive',
          disabled: menuSlot.isActive,
          onSelect: () => requestDelete(menuSlot.id),
        },
      ]}
    />
  {/if}

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
    serverType={worldServerType()}
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

{#if editingSlot}
  <Sheet
    title={`World Settings — ${editingSlot.name}`}
    size="lg"
    onClose={editingBusy ? undefined : closeWorldSettings}
  >
    {#if editingLoading}
      <p class="edit-status">Loading the saved world profile…</p>
    {:else if editingError && !editingProfile}
      <div class="edit-message">
        <p class="edit-error" role="alert">{editingError}</p>
        <Button size="sm" variant="secondary" onclick={closeWorldSettings}>Close</Button>
      </div>
    {:else if editingProfile && editingValues}
      <div class="edit-body">
        <WorldSettingsForm
          mode="edit"
          serverType={worldServerType()}
          metadata={editingProfile.profile.fieldMetadata}
          values={editingValues}
          serverSettingsHref="../settings"
          onChange={updateEditingValues}
        />

        {#if editingConfirmation}
          <div class="confirmation" role="alert">
            <p class="confirmation-title">{editingConfirmation.title}</p>
            <p>{editingConfirmation.message}</p>
            <div class="confirmation-actions">
              <Button variant="secondary" onclick={() => (editingConfirmation = undefined)}>
                Cancel
              </Button>
              <Button
                variant="primary"
                onclick={() => void saveWorldSettings(editingConfirmation?.token)}
              >
                Continue
              </Button>
            </div>
          </div>
        {/if}
        {#if editingError}<p class="edit-error" role="alert">{editingError}</p>{/if}
        {#if editingNotice}<p class="edit-status" role="status">{editingNotice}</p>{/if}
        {#if editingNotice && editingProfile}
          <p class="edit-detail">
            The agent read back the saved profile. Creation-only values stay locked after
            generation; use Replace World or create a new slot when the terrain itself must change.
          </p>
        {/if}
        <div class="edit-footer">
          <Button variant="secondary" onclick={closeWorldSettings} disabled={editingBusy}>
            Close
          </Button>
          <Button variant="primary" disabled={editingBusy} onclick={() => void saveWorldSettings()}>
            {editingBusy ? 'Saving…' : 'Save World Settings'}
          </Button>
        </div>
      </div>
    {/if}
  </Sheet>
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
  .ownership {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .edit-body,
  .edit-message {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .edit-status,
  .edit-detail,
  .edit-error {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
  }
  .edit-status,
  .edit-detail {
    color: var(--msc2-text-secondary);
  }
  .edit-error {
    color: var(--msc2-status-warn);
  }
  .edit-footer,
  .confirmation-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .confirmation {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 10px 12px;
    border: 1px solid var(--msc2-hairline-strong);
    border-radius: 8px;
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }
  .confirmation p {
    margin: 0;
  }
  .confirmation-title {
    color: var(--msc2-text-primary);
    font-weight: 600;
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
