<script lang="ts">
  // Category = neutral (Java/Paper). Status = colored tint (Active, Missing).
  // docs/msc2/renderings/primitives.html
  export let variant: 'category' | 'status' = 'category';
  export let tone: 'ok' | 'warn' | 'error' | 'bedrock' = 'ok';
</script>

{#if variant === 'category'}
  <span class="badge category"><slot /></span>
{:else}
  <span class="badge status {tone}"><slot /></span>
{/if}

<style>
  .badge {
    display: inline-block;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 9px;
    border-radius: 6px;
  }

  .category {
    color: rgba(255, 255, 255, 0.8);
    background: var(--msc2-neutral-elevated);
  }

  .status.ok {
    color: var(--msc2-status-ok);
    background: var(--msc2-status-ok-tint);
  }
  .status.warn {
    color: var(--msc2-status-warn);
    background: var(--msc2-status-warn-tint);
  }
  .status.error {
    color: var(--msc2-status-error);
    background: var(--msc2-status-error-tint);
  }
  .status.bedrock {
    color: var(--msc2-status-bedrock);
    background: var(--msc2-status-bedrock-tint);
  }
</style>
