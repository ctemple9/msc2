<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import ConfirmDialog from '../../components/ConfirmDialog.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, dateLabel, errorMessage, mutate } from '../shared/types';

  export let api: ScreenProps['api'] = undefined;
  let users: Schema['UserSummaryDTO'][] = [];
  let label = '';
  let role = 'operator';
  let permissionText = 'serverControl';
  let oneTimeSecret = '';
  let notice = '';
  let pendingRevoke: string | null = null;
  onMount(async () => {
    users = (await call<Schema['UserListResponseDTO']>(api, { users: [] }, '/v1/users')).users;
  });
  async function create(): Promise<void> {
    try {
      const result = await mutate<Schema['UserCreateResultDTO']>(api, '/v1/users', {
        label,
        role,
        permissions: permissionText
          .split(',')
          .map((item) => item.trim())
          .filter(Boolean),
      });
      notice = result.message;
      oneTimeSecret = result.token ?? '';
      if (result.user) users = [...users, result.user];
      label = '';
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function revoke(): Promise<void> {
    if (!pendingRevoke) return;
    try {
      const result = await mutate<Schema['UserRevokeResultDTO']>(api, '/v1/users/revoke', {
        userId: pendingRevoke,
      });
      notice = result.message;
      users = users.filter((user) => user.id !== pendingRevoke);
    } catch (error) {
      notice = errorMessage(error);
    }
    pendingRevoke = null;
  }
  async function extend(user: Schema['UserSummaryDTO']): Promise<void> {
    try {
      const result = await mutate<Schema['UserUpdateResultDTO']>(api, '/v1/users/update', {
        userId: user.id,
        expiresInDays: 30,
      });
      notice = result.message;
      if (result.user)
        users = users.map((current) => (current.id === user.id ? result.user! : current));
    } catch (error) {
      notice = errorMessage(error);
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Named tokens"
    title="Access administration"
    description="Create, update, and revoke named tokens. A newly created secret is shown once and is never copied into a URL or Svelte-accessible storage."
    status={`${users.length} tokens`}
    statusTone="positive"
  />
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  {#if oneTimeSecret}<section class="capability-notice" role="alert">
      <strong>Copy this secret now</strong>
      <p class="one-time-secret">{oneTimeSecret}</p>
      <small>It will not be returned again. Store it in the platform credential store.</small>
    </section>{/if}
  <section class="screen-card">
    <h3>Create named token</h3>
    <div class="form-grid" style="margin-top: .7rem">
      <div class="field">
        <label for="token-label">Label</label><input
          id="token-label"
          bind:value={label}
          placeholder="Cameron's desktop"
        />
      </div>
      <div class="field">
        <label for="token-role">Role</label><select id="token-role" bind:value={role}
          ><option value="viewer">Viewer</option><option value="operator">Operator</option><option
            value="admin">Admin</option
          ></select
        >
      </div>
      <div class="field full">
        <label for="token-permissions">Permission categories (comma-separated)</label><input
          id="token-permissions"
          bind:value={permissionText}
          placeholder="serverControl, worlds"
        />
      </div>
    </div>
    <ActionButton label="Create token" onclick={create}>Create token</ActionButton>
  </section>
  <section class="screen-card">
    <table class="data-table">
      <thead
        ><tr
          ><th>Label</th><th>Role</th><th>Permissions</th><th>Expiry</th><th class="actions"
            >Actions</th
          ></tr
        ></thead
      ><tbody
        >{#each users as user (user.id)}<tr
            ><td><strong>{user.label}</strong></td><td>{user.role}</td><td
              ><div class="tag-list">
                {#each user.permissions ?? [] as permission}<span class="tag">{permission}</span
                  >{/each}
              </div></td
            ><td>{user.isExpired ? 'Expired' : dateLabel(user.expiresAtISO8601)}</td><td
              class="actions"
              ><ActionButton kind="quiet" label="Extend token" onclick={() => extend(user)}
                >Extend 30 days</ActionButton
              ><ActionButton
                kind="danger"
                label="Revoke token"
                onclick={() => (pendingRevoke = user.id)}>Revoke</ActionButton
              ></td
            ></tr
          >{:else}<tr
            ><td colspan="5" class="empty-row">No named tokens are visible for this credential.</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </section>
  <CapabilityNotice
    title="Agent-Planned files remain out of this screen"
    message="File browsing, watchdog controls, and player-profile administration are not represented by a decorative disabled control."
  />
  <ConfirmDialog
    open={pendingRevoke !== null}
    title="Revoke this token?"
    message="Existing clients using this named token will lose access immediately."
    confirmLabel="Revoke token"
    onConfirm={revoke}
    onClose={() => (pendingRevoke = null)}
  />
</div>
