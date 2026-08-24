<script lang="ts">
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
    <dialog open class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="dialog-title">
      <p class="eyebrow">{context}</p>
      <h2 id="dialog-title">{title}</h2>
      <p class="message">{message}</p>
      <div class="dialog-actions">
        <button type="button" class="quiet" onclick={onClose}>Cancel</button>
        <button type="button" class="danger" onclick={onConfirm}>{confirmLabel}</button>
      </div>
    </dialog>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    z-index: 10;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(7, 12, 16, 0.72);
  }

  .dialog {
    width: min(100%, 28rem);
    padding: 1.4rem;
    border: 1px solid var(--msc-border);
    border-radius: var(--msc-radius-lg);
    background: var(--msc-surface-raised);
    box-shadow: var(--msc-shadow);
  }

  .eyebrow {
    margin: 0 0 0.45rem;
    color: var(--msc-warning);
    font-size: 0.72rem;
    font-weight: 800;
    text-transform: uppercase;
  }
  h2 {
    margin: 0;
    font-size: 1.25rem;
  }
  .message {
    color: var(--msc-muted);
    line-height: 1.55;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1.2rem;
  }
  button {
    border: 0;
    border-radius: var(--msc-radius-sm);
    padding: 0.7rem 0.9rem;
    font: inherit;
    font-weight: 750;
    cursor: pointer;
  }
  button:focus-visible {
    outline: none;
    box-shadow: var(--msc-focus);
  }
  .quiet {
    color: var(--msc-text);
    background: transparent;
  }
  .danger {
    color: #271415;
    background: var(--msc-danger);
  }
</style>
