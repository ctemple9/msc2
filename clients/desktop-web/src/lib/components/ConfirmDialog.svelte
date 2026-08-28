<script lang="ts">
  import Button from './base/Button.svelte';

  export let open = false;
  export let title = 'Confirm action';
  export let message = '';
  export let context = 'Host: selected host';
  export let confirmLabel = 'Confirm';
  export let onConfirm: (() => void) | undefined = undefined;
  export let onClose: (() => void) | undefined = undefined;
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={(event) => event.target === event.currentTarget && onClose?.()}
  >
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
      tabindex="-1"
    >
      <p class="eyebrow">{context}</p>
      <h2 id="dialog-title">{title}</h2>
      <p class="message">{message}</p>
      <div class="dialog-actions">
        <Button variant="secondary" size="sm" onclick={onClose}>Cancel</Button>
        <Button variant="destructive" size="sm" onclick={onConfirm}>{confirmLabel}</Button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    z-index: 200;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.72);
    backdrop-filter: blur(2px);
  }

  .dialog {
    box-sizing: border-box;
    width: min(100%, 28rem);
    margin: 0;
    padding: 20px;
    border: 1px solid var(--msc2-hairline-faint);
    border-radius: 14px;
    color: var(--msc2-text-primary);
    background: var(--msc2-tier-chrome);
    box-shadow: var(--msc2-shadow-float);
  }

  .eyebrow {
    margin: 0 0 7px;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }
  .message {
    margin: 10px 0 0;
    color: var(--msc2-text-secondary);
    font-size: 13px;
    line-height: 1.55;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }
</style>
