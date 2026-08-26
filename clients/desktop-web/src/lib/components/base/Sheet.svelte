<script lang="ts">
  // The sheet frame. Three fixed widths only — 480 / 640 / 820. Not locked in
  // a rendering specimen; built to the same law as the rest of S0 (flat tier
  // surfaces, no glass, shadow only because this genuinely floats above the
  // shell — the one place a blur scrim is sanctioned, docs/msc2/antiAIslop.md #4).
  export let title: string;
  export let size: 'sm' | 'md' | 'lg' = 'md';
  export let onClose: (() => void) | undefined = undefined;

  const widths = { sm: '480px', md: '640px', lg: '820px' } as const;

  function dismissOnBackdrop(event: MouseEvent) {
    if (event.target === event.currentTarget) onClose?.();
  }

  function dismissOnEscape(event: KeyboardEvent) {
    if (event.key === 'Escape') onClose?.();
  }
</script>

<svelte:window onkeydown={onClose ? dismissOnEscape : undefined} />

<div class="scrim" role="presentation" onclick={dismissOnBackdrop}>
  <div
    class="sheet"
    style="width: {widths[size]};"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
  >
    <div class="header">
      <span class="title">{title}</span>
      {#if onClose}
        <button type="button" class="close" aria-label="Close" onclick={onClose}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M6 6l12 12M18 6L6 18"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
          </svg>
        </button>
      {/if}
    </div>
    <div class="body">
      <slot />
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .sheet {
    box-sizing: border-box;
    max-width: calc(100vw - 32px);
    max-height: calc(100vh - 32px);
    overflow: auto;
    background: var(--msc2-tier-chrome);
    border-radius: 14px;
    box-shadow: var(--msc2-shadow-float);
    scrollbar-width: none;
  }
  .sheet::-webkit-scrollbar {
    display: none;
    width: 0;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 16px 16px 20px;
    border-bottom: 1px solid var(--msc2-hairline-faint);
  }

  .title {
    font-size: 15px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }

  .close {
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.6);
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }
  .close:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .body {
    padding: 20px;
  }
</style>
