<script lang="ts">
  // The button system. 2 shapes only — filled (one per context) and quiet
  // (the default). No gradient, no shadow, no glow. docs/msc2/renderings/buttons-and-type.html
  import { onboardingAnchor } from '../../help/tourAnchors';

  export let variant: 'primary' | 'start' | 'stop' | 'secondary' | 'destructive' | 'ghost-icon' =
    'secondary';
  export let size: 'md' | 'sm' = 'md';
  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let disabled = false;
  export let label = '';
  /** A native tooltip, set directly on the <button> -- not a wrapping
   *  element. A <span> wrapper used only to carry `title` is both a flex
   *  item and its own flex container at once, which is exactly the nested
   *  shape that turned out unreliable for equal-width sizing across
   *  browsers (see WorldSlotCard.svelte's history) -- this keeps every
   *  Button a single element, however a parent's flex layout treats it. */
  export let title: string | undefined = undefined;
  export let onclick: ((event: MouseEvent) => void) | undefined = undefined;
  /** Reports this button's rect to the guided tour under this id, when set.
   *  See src/lib/help/tourAnchors.ts. Additive -- most callers leave it unset. */
  export let anchorId: string | undefined = undefined;
</script>

<button
  class="btn {variant} {size}"
  {type}
  {disabled}
  {title}
  aria-label={variant === 'ghost-icon' ? label : undefined}
  {onclick}
  use:onboardingAnchor={anchorId}
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
    box-sizing: border-box;
    /* A flex item's default min-width is `auto` (its own content size), not
       0 -- so a parent applying `flex: 1` to size two buttons evenly (e.g.
       WorldSlotCard.svelte's action rows) still lets a WebKit button keep
       its native minimum-content floor, which can end up wider or narrower
       than a same-`flex:1` sibling whose text measures differently. Safari
       is the browser that actually keeps that native floor in practice
       (Chromium doesn't), which is why uneven button widths only ever
       showed up in the real Tauri/WKWebView app, never in a Chromium check.
       `min-width: 0` plus `appearance: none` remove both the floor and
       whatever native sizing behavior WebKit was applying to the button. */
    min-width: 0;
    appearance: none;
    -webkit-appearance: none;
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
