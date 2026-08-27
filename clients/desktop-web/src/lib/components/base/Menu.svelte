<script lang="ts">
  // A small action menu anchored at a click point -- e.g. clicking a plugin
  // row instead of exposing an always-visible Remove button. Flat opaque
  // surface, no blur (docs/msc2/antiAIslop.md #4 reserves blur for a true
  // modal scrim; this is a lightweight popover, not one). Clamped to stay
  // inside the viewport since the anchor point can be anywhere on screen.
  export let x: number;
  export let y: number;
  export let onClose: () => void;
  export let items: {
    label: string;
    onSelect: () => void;
    tone?: 'default' | 'destructive';
    disabled?: boolean;
  }[];

  let menuEl: HTMLDivElement | undefined;
  let left = x;
  let top = y;

  function clamp(): void {
    if (!menuEl) return;
    const rect = menuEl.getBoundingClientRect();
    left = Math.min(x, window.innerWidth - rect.width - 8);
    top = Math.min(y, window.innerHeight - rect.height - 8);
  }

  function bind(el: HTMLDivElement): { destroy(): void } {
    menuEl = el;
    clamp();
    return { destroy: () => (menuEl = undefined) };
  }

  function select(item: (typeof items)[number]): void {
    if (item.disabled) return;
    item.onSelect();
    onClose();
  }

  function dismissOnEscape(event: KeyboardEvent): void {
    if (event.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={dismissOnEscape} />

<div class="scrim" role="presentation" onclick={onClose} oncontextmenu={onClose}></div>
<div class="menu" use:bind style="left: {left}px; top: {top}px;" role="menu">
  {#each items as item (item.label)}
    <button
      type="button"
      role="menuitem"
      class="item"
      class:destructive={item.tone === 'destructive'}
      disabled={item.disabled}
      onclick={() => select(item)}
    >
      {item.label}
    </button>
  {/each}
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 100;
  }
  .menu {
    position: fixed;
    z-index: 101;
    min-width: 150px;
    padding: 4px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline);
    border-radius: 10px;
    box-shadow: var(--msc2-shadow-float);
    display: flex;
    flex-direction: column;
  }
  .item {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-radius: 6px;
    padding: 7px 10px;
    font: inherit;
    font-size: 13px;
    color: var(--msc2-text-primary);
    cursor: pointer;
  }
  .item:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
  }
  .item:disabled {
    color: var(--msc2-text-tertiary);
    cursor: not-allowed;
  }
  .item.destructive {
    color: var(--msc2-status-error);
  }
</style>
