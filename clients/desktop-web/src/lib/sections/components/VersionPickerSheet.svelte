<script lang="ts">
  // Shared by the Server JAR row, the Modded Loader row, and the Bedrock
  // Runtime card -- all three are one route pair in the real contract:
  // GET /v1/versions lists what's available for the active server's flavor
  // (VersionsResponseDTO.isBedrock/runtime cover Bedrock the same way),
  // POST /v1/components/version applies the choice. Ports the shape of
  // MSC 1's VersionPickerSheet.swift without a separate Bedrock variant.
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { componentPaths, pollOperation } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let title = 'Change Version';
  export let serverRunning = false;
  export let onClose: () => void;
  export let onChanged: (result: Schema['VersionChangeResultDTO']) => void;

  type State =
    | { kind: 'loading' }
    | { kind: 'ready'; response: Schema['VersionsResponseDTO'] }
    | { kind: 'unavailable'; message: string };
  let state: State = { kind: 'loading' };
  let selectedId: string | undefined;
  let applying = false;
  let statusLine = '';
  let failureMessage = '';

  onMount(async () => {
    if (!api) {
      state = { kind: 'unavailable', message: 'Not connected to an agent.' };
      return;
    }
    try {
      const response = await api.get<Schema['VersionsResponseDTO']>(componentPaths.versions);
      state = { kind: 'ready', response };
      selectedId =
        response.versions.find((entry) => entry.isLatest)?.id ?? response.versions[0]?.id;
    } catch (error) {
      state = {
        kind: 'unavailable',
        message: error instanceof Error ? error.message : 'Could not load available versions.',
      };
    }
  });

  async function apply(): Promise<void> {
    if (!selectedId) return;
    applying = true;
    failureMessage = '';
    statusLine = 'Applying version…';
    try {
      const result = await mutate<Schema['VersionChangeResultDTO']>(api, componentPaths.version, {
        versionId: selectedId,
      });
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId, (tick) => {
          statusLine = tick.statusLine ?? statusLine;
        });
        if (operation && operation.state !== 'succeeded') {
          failureMessage = operation.error?.message ?? 'Version change did not complete.';
          applying = false;
          return;
        }
      }
      onChanged(result);
      onClose();
    } catch (error) {
      failureMessage = error instanceof Error ? error.message : 'Failed to change version.';
      applying = false;
    }
  }
</script>

<Sheet {title} size="sm" onClose={applying ? undefined : onClose}>
  {#if state.kind === 'loading'}
    <p class="explain">Loading available versions…</p>
  {:else if state.kind === 'unavailable'}
    <p class="explain">{state.message}</p>
    <div class="footer">
      <Button variant="secondary" onclick={onClose}>Close</Button>
    </div>
  {:else if applying}
    <p class="explain">{statusLine}</p>
  {:else}
    {#if serverRunning}
      <p class="explain warn">Stop the server before changing its version.</p>
    {/if}
    {#if state.response.versions.length === 0}
      <p class="explain">No versions were reported for {state.response.flavorName}.</p>
    {:else}
      <div class="list" role="listbox" aria-label="Available versions">
        {#each state.response.versions as entry (entry.id)}
          <button
            type="button"
            class="row"
            class:selected={selectedId === entry.id}
            onclick={() => (selectedId = entry.id)}
          >
            <span class="label">{entry.displayLabel}</span>
            {#if entry.isLatest}<span class="tag">Latest</span>{/if}
          </button>
        {/each}
      </div>
    {/if}
    {#if failureMessage}<p class="explain warn">{failureMessage}</p>{/if}
    <div class="footer">
      <Button variant="secondary" onclick={onClose}>Cancel</Button>
      <Button
        variant="primary"
        disabled={!selectedId || serverRunning || state.response.versions.length === 0}
        onclick={() => void apply()}
      >
        Apply
      </Button>
    </div>
  {/if}
</Sheet>

<style>
  .explain {
    margin: 0 0 12px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .explain.warn {
    color: var(--msc2-status-warn);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 260px;
    overflow-y: auto;
    margin-bottom: 12px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 9px 12px;
    background: var(--msc2-tier-chrome);
    border: 1px solid transparent;
    border-radius: 8px;
    color: var(--msc2-text-primary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }
  .row.selected {
    border-color: rgba(255, 255, 255, 0.28);
  }
  .tag {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
