<script lang="ts">
  export let kind: 'empty' | 'loading' | 'error' = 'empty';
  export let title = '';
  export let message = '';
</script>

<div class="state-panel {kind}" role={kind === 'error' ? 'alert' : 'status'}>
  <strong>{title}</strong>
  <p>{message}</p>
  <slot />
</div>

<style>
  .state-panel {
    padding: 1rem;
    border: 1px dashed var(--msc-border);
    border-radius: var(--msc-radius-sm);
    color: var(--msc-muted);
  }

  strong {
    color: var(--msc-text);
  }
  p {
    margin: 0.35rem 0 0;
    line-height: 1.5;
  }
  .error {
    border-color: rgba(239, 154, 154, 0.42);
  }
  .error strong {
    color: var(--msc-danger);
  }
  .loading strong {
    color: var(--msc-warning);
  }
</style>
