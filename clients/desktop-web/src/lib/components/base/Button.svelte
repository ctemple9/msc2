<script lang="ts">
  // The button system. 2 shapes only — filled (one per context) and quiet
  // (the default). No gradient, no shadow, no glow. docs/msc2/renderings/buttons-and-type.html
  export let variant: 'primary' | 'start' | 'stop' | 'secondary' | 'destructive' | 'ghost-icon' =
    'secondary';
  export let size: 'md' | 'sm' = 'md';
  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let disabled = false;
  export let label = '';
  export let onclick: ((event: MouseEvent) => void) | undefined = undefined;
</script>

<button
  class="btn {variant} {size}"
  {type}
  {disabled}
  aria-label={variant === 'ghost-icon' ? label : undefined}
  {onclick}
>
  <slot />
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border: none;
    cursor: pointer;
    font-family: inherit;
    transition:
      filter 120ms ease,
      background 120ms ease,
      transform 80ms ease;
  }

  .btn:active:not(:disabled) {
    transform: scale(0.98);
  }

  .btn:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  /* Sizes */
  .md {
    font-size: 13px;
    border-radius: 8px;
    padding: 8px 16px;
  }
  .sm {
    font-size: 12px;
    border-radius: 7px;
    padding: 5px 12px;
  }

  /* Filled — one per context, colored only by meaning */
  .primary,
  .start,
  .stop {
    font-weight: 600;
  }
  .primary:hover:not(:disabled),
  .start:hover:not(:disabled),
  .stop:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .primary {
    color: var(--msc2-neutral-fill-ink);
    background: var(--msc2-neutral-fill);
  }
  .start {
    color: #0d2416;
    background: var(--msc2-status-ok);
  }
  .stop {
    color: #fff;
    background: var(--msc2-status-error);
  }

  /* Quiet — the default */
  .secondary,
  .destructive {
    font-weight: 500;
    background: transparent;
    border: 1px solid var(--msc2-hairline);
  }
  .secondary {
    color: rgba(255, 255, 255, 0.9);
  }
  .secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.06);
  }
  .destructive {
    color: var(--msc2-status-error);
    border-color: rgba(226, 75, 74, 0.4);
  }
  .destructive:hover:not(:disabled) {
    background: rgba(226, 75, 74, 0.08);
  }

  .ghost-icon {
    color: rgba(255, 255, 255, 0.72);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    padding: 0;
  }
  .ghost-icon:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
  }
  .ghost-icon.md {
    width: 32px;
    height: 32px;
  }
  .ghost-icon.sm {
    width: 26px;
    height: 26px;
  }
</style>
