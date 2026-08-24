<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import ConfirmDialog from '../../components/ConfirmDialog.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import OperationQueue from '../shared/OperationQueue.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, bytesLabel, dateLabel, errorMessage, mutate } from '../shared/types';
  import { demoBackups, backupPaths } from './model';

  export let api: ScreenProps['api'] = undefined;
  export let operations: readonly Schema['OperationDTO'][] = [];
  let backups = demoBackups;
  let pending: 'restore' | 'delete' | null = null;
  let pendingId = '';
  let autoEnabled = true;
  let interval = 60;
  let maxCount = 10;
  let notice = '';

  onMount(async () => {
    const response = await call<Schema['BackupsResponseDTO']>(api, { backups }, backupPaths.list);
    backups = response.backups;
  });
  async function run(path: string, body?: unknown): Promise<void> {
    try {
      const result = await mutate<Record<string, unknown>>(api, path, body);
      notice = String(result.result ?? result.message ?? 'Backup operation accepted.');
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function confirm(): Promise<void> {
    if (!pending || !pendingId) return;
    await run(pending === 'restore' ? backupPaths.restore : backupPaths.delete, {
      [pending === 'restore' ? 'backupId' : 'backupId']: pendingId,
    });
    pending = null;
  }
  async function saveConfig(): Promise<void> {
    await run(backupPaths.config, {
      autoBackupEnabled: autoEnabled,
      autoBackupIntervalMinutes: interval,
      autoBackupMaxCount: maxCount,
    });
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Protection"
    title="Backups"
    description="Backup creation, configuration, deletion, and restore stay transactional and show operation progress."
    status={`${backups.length} backups`}
    statusTone="positive"
    actionLabel="Back up now"
    onAction={() => run(backupPaths.now)}
  />
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  <section class="screen-card">
    <table class="data-table">
      <thead
        ><tr
          ><th>Backup</th><th>Created</th><th>Size</th><th>Trigger</th><th class="actions"
            >Actions</th
          ></tr
        ></thead
      ><tbody
        >{#each backups as backup (backup.id)}<tr
            ><td
              ><strong>{backup.displayName}</strong><br /><small
                >{backup.slotName ?? 'Server snapshot'}</small
              ></td
            ><td>{dateLabel(backup.modificationDate)}</td><td>{bytesLabel(backup.fileSize)}</td><td
              >{backup.isAutomatic ? 'Automatic' : backup.triggerReason}</td
            ><td class="actions"
              ><ActionButton
                kind="quiet"
                label="Restore backup"
                onclick={() => {
                  pending = 'restore';
                  pendingId = backup.id;
                }}>Restore</ActionButton
              ><ActionButton
                kind="danger"
                label="Delete backup"
                onclick={() => {
                  pending = 'delete';
                  pendingId = backup.id;
                }}>Delete</ActionButton
              ></td
            ></tr
          >{:else}<tr><td colspan="5" class="empty-row">No verified backups yet.</td></tr
          >{/each}</tbody
      >
    </table>
  </section>
  <div class="screen-grid">
    <section class="screen-card">
      <h3>Automatic backups</h3>
      <div class="form-grid" style="margin-top: .7rem">
        <div class="field">
          <label for="backup-enabled">Schedule</label><select
            id="backup-enabled"
            bind:value={autoEnabled}
            ><option value={true}>Enabled</option><option value={false}>Disabled</option></select
          >
        </div>
        <div class="field">
          <label for="backup-interval">Interval (minutes)</label><input
            id="backup-interval"
            type="number"
            min="5"
            bind:value={interval}
          />
        </div>
        <div class="field">
          <label for="backup-count">Maximum count</label><input
            id="backup-count"
            type="number"
            min="1"
            bind:value={maxCount}
          />
        </div>
      </div>
      <ActionButton label="Save backup settings" onclick={saveConfig}>Save</ActionButton>
    </section>
    <section class="screen-card">
      <h3>Recovery progress</h3>
      <OperationQueue {operations} />
    </section>
  </div>
  <CapabilityNotice
    title="Restore stays risk-aware"
    message="A restore may require stopping the server and will remain visible as an operation. Bedrock live restore follows the slot-based Worlds workflow."
  />
  <ConfirmDialog
    open={pending !== null}
    title={pending === 'restore' ? 'Restore this backup?' : 'Delete this backup?'}
    message="The selected host and backup are shown above; confirm only if this is the intended server."
    confirmLabel={pending === 'restore' ? 'Restore backup' : 'Delete backup'}
    onConfirm={confirm}
    onClose={() => (pending = null)}
  />
</div>
