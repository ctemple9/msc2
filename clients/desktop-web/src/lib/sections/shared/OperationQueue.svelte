<script lang="ts">
  import ActionButton from '../../components/ActionButton.svelte';
  import type { Schema } from './types';
  import { operationLabel } from './types';

  export let operations: readonly Schema['OperationDTO'][] = [];
  export let onCancel: ((id: string) => void) | undefined = undefined;
</script>

<div class="operation-list" aria-live="polite">
  {#if operations.length === 0}
    <p class="muted">
      No active operations. Long-running work will remain visible here after reconnect.
    </p>
  {:else}
    {#each operations as operation (operation.id)}
      <div class="operation-row">
        <div>
          <strong>{operationLabel(operation)}</strong>
          <p>{operation.statusLine || operation.error?.message || 'Working…'}</p>
        </div>
        {#if (operation.state === 'running' || operation.state === 'queued') && operation.cancelable !== false}
          <ActionButton
            kind="quiet"
            label="Cancel operation"
            onclick={() => onCancel?.(operation.id)}>Cancel</ActionButton
          >
        {/if}
      </div>
    {/each}
  {/if}
</div>
