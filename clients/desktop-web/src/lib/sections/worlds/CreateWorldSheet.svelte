<script lang="ts">
  // A new world is created first because the profile route is slot-scoped.
  // Once the slot exists, the same WorldSettingsForm values are saved through
  // that route, including the central safety confirmation when needed.
  import { ApiError } from '../../api/client';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import WorldSettingsForm from './WorldSettingsForm.svelte';
  import type { ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import {
    defaultWorldSettingsValues,
    worldPaths,
    worldSettingsChanges,
    type WorldProfileUpdateResult,
    type WorldServerType,
    type WorldSettingsValues,
  } from './model';
  import type { Schema } from '../shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: WorldServerType = 'java';
  export let onClose: () => void;
  export let onCreated: (updated: Schema['WorldSlotsResponseDTO']) => void;

  let values: WorldSettingsValues = defaultWorldSettingsValues(serverType);
  let busy = false;
  let error: string | undefined;
  let confirmation: SafetyPrompt | undefined;
  let createdWorlds: Schema['WorldSlotsResponseDTO'] | undefined;
  let createdSlotId: string | undefined;
  let status: string | undefined;

  type SafetyPrompt = {
    token: string;
    title: string;
    message: string;
  };

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

  function updateValues(next: WorldSettingsValues): void {
    values = next;
  }

  function slotCreatedByRequest(
    before: ReadonlySet<string>,
    updated: Schema['WorldSlotsResponseDTO'],
  ): Schema['WorldSlotDTO'] | undefined {
    return (
      updated.slots.find((slot) => !before.has(slot.id)) ??
      updated.slots
        .filter((slot) => slot.name === values.name.trim())
        .sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0]
    );
  }

  function replaceUpdatedSlot(
    worlds: Schema['WorldSlotsResponseDTO'],
    slot: Schema['WorldSlotDTO'],
  ): Schema['WorldSlotsResponseDTO'] {
    return { ...worlds, slots: worlds.slots.map((item) => (item.id === slot.id ? slot : item)) };
  }

  async function saveProfile(confirmationToken?: string): Promise<void> {
    if (!createdSlotId) return;
    const result = await mutate<WorldProfileUpdateResult>(api, worldPaths.profile(createdSlotId), {
      changes: worldSettingsChanges({ ...values, name: values.name.trim() }, serverType),
      ...(confirmationToken ? { confirmation: confirmationToken } : {}),
    });
    status = result.status;
    values = {
      ...values,
      name: result.slot.profile.identity.name ?? result.slot.slot.name,
      levelName: result.slot.profile.identity.levelName ?? '',
      seed: result.slot.profile.identity.seed ?? values.seed,
    };
    if (createdWorlds) {
      createdWorlds = replaceUpdatedSlot(createdWorlds, result.slot.slot);
      onCreated(createdWorlds);
    }
  }

  async function submit(confirmationToken?: string): Promise<void> {
    const trimmedName = values.name.trim();
    if (!trimmedName || busy) return;
    busy = true;
    error = undefined;
    confirmation = undefined;
    try {
      if (!createdSlotId) {
        const before = new Set(createdWorlds?.slots.map((slot) => slot.id) ?? []);
        const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.create, {
          name: trimmedName,
          seed: values.seed.trim() || undefined,
        });
        if (!result.updated) throw new Error('The agent created no world slot to configure.');
        createdWorlds = result.updated;
        onCreated(result.updated);
        const slot = slotCreatedByRequest(before, result.updated);
        if (!slot) throw new Error('The new world slot could not be identified.');
        createdSlotId = slot.id;
      }
      await saveProfile(confirmationToken);
    } catch (caught) {
      confirmation = safetyPrompt(caught);
      if (!confirmation) {
        error = createdSlotId
          ? `World created, but its settings were not saved: ${caught instanceof Error ? caught.message : 'unknown error'}`
          : caught instanceof Error
            ? caught.message
            : 'Failed to create the new world.';
      }
    } finally {
      busy = false;
    }
  }

  function statusLabel(value: string): string {
    if (value === 'pending_restart') return 'Saved. Restart the server to apply these settings.';
    if (value === 'blocked') return 'Saved, but the active runtime could not apply these settings.';
    return 'World settings saved and applied.';
  }
</script>

<Sheet title="Create New World" size="md" onClose={busy ? undefined : onClose}>
  <div class="body">
    {#if createdSlotId}
      <p class="created-note">The world slot exists. Its profile is being saved now.</p>
    {/if}

    <WorldSettingsForm
      mode="create"
      {serverType}
      {values}
      serverSettingsHref="../settings"
      onChange={updateValues}
    />

    {#if confirmation}
      <div class="confirmation" role="alert">
        <p class="confirmation-title">{confirmation.title}</p>
        <p>{confirmation.message}</p>
        <div class="confirmation-actions">
          <Button variant="secondary" onclick={() => (confirmation = undefined)}>Cancel</Button>
          <Button variant="primary" onclick={() => void submit(confirmation?.token)}>
            Continue
          </Button>
        </div>
      </div>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if status}<p class="status" role="status">{statusLabel(status)}</p>{/if}

    <div class="footer">
      {#if createdSlotId}
        <Button variant="primary" onclick={onClose}>Done</Button>
      {:else}
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button
          variant="primary"
          disabled={!values.name.trim() || busy}
          onclick={() => void submit()}>Create World</Button
        >
      {/if}
    </div>
  </div>
</Sheet>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .created-note,
  .status,
  .error,
  .confirmation p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
  }
  .created-note,
  .status {
    color: var(--msc2-text-secondary);
  }
  .error {
    color: var(--msc2-status-warn);
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
  .confirmation-title {
    color: var(--msc2-text-primary);
    font-weight: 600;
  }
  .confirmation-actions,
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
