<script lang="ts">
  // Ports DetailsPlayersTabView's bedrockAllowlistCard. Bedrock-only.
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Field from '../../components/base/Field.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema } from '../shared/types';

  export let entries: readonly Schema['AllowlistEntryDTO'][] = [];
  export let onAdd: ((gamertag: string) => void) | undefined = undefined;
  export let onRemove: ((gamertag: string) => void) | undefined = undefined;
  export let onReload: (() => void) | undefined = undefined;

  let newEntry = '';

  function commitAdd(): void {
    const trimmed = newEntry.trim();
    if (!trimmed) return;
    onAdd?.(trimmed);
    newEntry = '';
  }
</script>

<Card>
  <div class="header">
    <div class="overline">
      <span class="msc2-type-overline">Allowlist</span>
    </div>
    <div class="header-actions">
      <span class="count">{entries.length} entries</span>
      {#if onReload}<Button size="sm" onclick={onReload}>Reload</Button>{/if}
    </div>
  </div>

  <form class="add-row" onsubmit={(event) => (event.preventDefault(), commitAdd())}>
    <Field bind:value={newEntry} placeholder="Gamertag" width="auto" />
    <Button size="sm" type="submit" disabled={newEntry.trim().length === 0}>Add</Button>
  </form>

  {#if entries.length === 0}
    <p class="empty">Allowlist is empty. All players can join.</p>
  {:else}
    <ul class="list">
      {#each entries as entry (entry.name)}
        <li class="row">
          <Icon name="seal-check" size={12} />
          <span class="name">{entry.name}</span>
          <span class="spacer"></span>
          <button
            type="button"
            class="remove"
            aria-label={`Remove ${entry.name} from allowlist`}
            onclick={() => onRemove?.(entry.name)}>Remove</button
          >
        </li>
      {/each}
    </ul>
  {/if}

  <p class="note">
    Note: allowlist is only enforced when online-mode is enabled in server properties.
  </p>
</Card>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
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
    gap: 10px;
  }
  .count {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .add-row {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }
  .add-row :global(.field) {
    flex: 1;
  }
  .empty {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-size: 12px;
    color: var(--msc2-text-primary);
  }
  .spacer {
    flex: 1;
  }
  .remove {
    font-size: 11px;
    color: var(--msc2-status-error);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .note {
    margin: 10px 0 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
</style>
