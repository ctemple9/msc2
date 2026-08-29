<script lang="ts">
  // Ports CreateWorldSlotSheet.swift: name + optional seed, a note that
  // difficulty/gamemode are server-wide (set from the Settings tab, not here).
  import Sheet from '../../components/base/Sheet.svelte';
  import Field from '../../components/base/Field.svelte';
  import Button from '../../components/base/Button.svelte';
  import { ApiError } from '../../api/client';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { worldPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let onClose: () => void;
  export let onCreated: (updated: Schema['WorldSlotsResponseDTO']) => void;

  let name = '';
  let seed = '';
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

  $: trimmedName = name.trim();

  async function submit(confirmationToken?: string): Promise<void> {
    if (!trimmedName || busy) return;
    busy = true;
    error = undefined;
    confirmation = undefined;
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.create, {
        name: trimmedName,
        seed: seed.trim() || undefined,
        ...(confirmationToken ? { confirmation: confirmationToken } : {}),
      });
      if (result.updated) onCreated(result.updated);
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

<Sheet title="Create New World" size="sm" {onClose}>
  <form class="body" onsubmit={(event) => (event.preventDefault(), submit())}>
    <div class="field-group">
      <span class="msc2-type-overline">World Name</span>
      <Field bind:value={name} placeholder="e.g. Survival World" />
      <p class="hint">
        This is the display name for the world slot, separate from the server name.
      </p>
    </div>

    <div class="field-group">
      <span class="msc2-type-overline">Seed</span>
      <Field bind:value={seed} placeholder="Optional — leave blank for a random world" />
      <p class="hint">
        Only used the first time this slot generates terrain; has no effect once a world exists.
      </p>
    </div>

    <p class="note">
      Difficulty and game mode are server-wide settings that apply to every world slot. Change them
      from the Settings tab.
    </p>

    {#if confirmation}
      <div class="confirmation" role="alert">
        <p class="confirmation-title">{confirmation.title}</p>
        <p>{confirmation.message}</p>
        <div class="confirmation-actions">
          <Button variant="secondary" type="button" onclick={() => (confirmation = undefined)}>
            Cancel
          </Button>
          <Button variant="primary" type="button" onclick={() => void submit(confirmation?.token)}>
            Continue
          </Button>
        </div>
      </div>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}

    <div class="footer">
      <Button variant="secondary" type="button" onclick={onClose}>Cancel</Button>
      <Button variant="primary" type="submit" disabled={!trimmedName || busy}>Create World</Button>
    </div>
  </form>
</Sheet>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .field-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field-group span {
    color: var(--msc2-text-tertiary);
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .note {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    background: var(--msc2-tier-chrome);
    border-radius: 8px;
    padding: 10px 12px;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-status-warn);
    background: var(--msc2-status-warn-tint);
    border-radius: 8px;
    padding: 8px 10px;
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
  .confirmation-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
