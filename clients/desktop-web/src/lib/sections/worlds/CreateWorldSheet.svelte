<script lang="ts">
  // Creation validates and saves the whole profile before activating a fresh slot.
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

  async function submit(confirmationToken?: string): Promise<void> {
    const trimmedName = values.name.trim();
    if (!trimmedName || busy) return;
    busy = true;
    error = undefined;
    confirmation = undefined;
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.create, {
        name: trimmedName,
        seed: values.seed.trim() || undefined,
        changes: worldSettingsChanges({ ...values, name: trimmedName }, serverType),
        ...(confirmationToken ? { confirmation: confirmationToken } : {}),
      });
      if (!result.updated?.activeSlotId) throw new Error('The new world was not activated.');
      onCreated(result.updated);
      onClose();
    } catch (caught) {
      confirmation = safetyPrompt(caught);
      if (!confirmation) {
        error = caught instanceof Error ? caught.message : 'Failed to create the new world.';
      }
    } finally {
      busy = false;
    }
  }
</script>

<Sheet title="Create New World" size="md" onClose={busy ? undefined : onClose}>
  <div class="body">
    <WorldSettingsForm
      {api}
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
    {#if busy}
      <div class="creation-progress" role="status" aria-live="polite">
        <span>Creating and activating world…</span>
        <progress aria-label="Creating and activating world"></progress>
      </div>
    {/if}

    <div class="footer">
      <Button variant="secondary" disabled={busy} onclick={onClose}>Cancel</Button>
      <Button variant="primary" disabled={!values.name.trim() || busy} onclick={() => void submit()}
        >Create World</Button
      >
    </div>
  </div>
</Sheet>

<style>
  .creation-progress {
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .creation-progress progress {
    width: 100%;
    height: 6px;
    accent-color: var(--msc2-text-secondary);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .error,
  .confirmation p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
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
