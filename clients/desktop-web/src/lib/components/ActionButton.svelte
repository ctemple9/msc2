<script lang="ts">
  export let kind: 'primary' | 'quiet' | 'danger' = 'primary';
  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let disabled = false;
  export let label = '';
  export let onclick: ((event: MouseEvent) => void) | undefined = undefined;
</script>

<button class="action-button {kind}" {type} {disabled} aria-label={label || undefined} {onclick}>
  <slot>{label}</slot>
</button>

<style>
  .action-button {
    border: 0;
    border-radius: var(--msc-radius-sm);
    padding: 0.7rem 0.95rem;
    font: inherit;
    font-weight: 750;
    cursor: pointer;
    transition:
      background 120ms ease,
      transform 120ms ease;
  }

  .action-button:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .action-button:focus-visible,
  .action-button:focus {
    outline: none;
    box-shadow: var(--msc-focus);
  }

  .primary {
    color: var(--msc-bg);
    background: var(--msc-accent);
  }

  .primary:hover:not(:disabled) {
    background: var(--msc-accent-strong);
  }

  .quiet {
    color: var(--msc-text);
    background: transparent;
  }

  .quiet:hover:not(:disabled) {
    background: rgba(232, 238, 242, 0.1);
  }

  .danger {
    color: #271415;
    background: var(--msc-danger);
  }

  .action-button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
</style>
