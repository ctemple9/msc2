<script lang="ts">
  import type { SectionDescriptor } from '../navigation/types';
  import ActionButton from './ActionButton.svelte';
  import StatusBadge from './StatusBadge.svelte';

  export let hostLabel = 'No host selected';
  export let serverLabel = 'No server selected';
  export let connectionLabel = 'Disconnected';
  export let sections: readonly SectionDescriptor[] = [];
  export let activeSection = '';
  export let onSection: ((id: string) => void) | undefined = undefined;
  export let onHostSwitcher: (() => void) | undefined = undefined;
  export let onConsole: (() => void) | undefined = undefined;
</script>

<div class="application-shell">
  <aside class="sidebar" aria-label="Main navigation">
    <div class="brand-block">
      <p class="brand-mark">MSC 2</p>
      <p class="brand-subtitle">Minecraft Server Controller</p>
    </div>

    <div class="context-card">
      <p class="context-label">Current host</p>
      <strong>{hostLabel}</strong>
      <p class="context-server">{serverLabel}</p>
      <StatusBadge
        status={connectionLabel}
        tone={connectionLabel === 'Connected' ? 'positive' : 'warning'}
      />
      <ActionButton kind="quiet" label="Switch host" onclick={onHostSwitcher}
        >Switch host</ActionButton
      >
    </div>

    <nav class="section-list" aria-label="Sections">
      {#each sections as section (section.id)}
        <button
          class:active={section.id === activeSection}
          type="button"
          aria-current={section.id === activeSection ? 'page' : undefined}
          onclick={() => onSection?.(section.id)}
        >
          <span>{section.label}</span>
          <span class="route-hint">{section.segment}</span>
        </button>
      {/each}
    </nav>

    <div class="sidebar-footer">
      <ActionButton kind="quiet" label="Open console" onclick={onConsole}>Console</ActionButton>
    </div>
  </aside>

  <main class="main-column">
    <header class="topbar">
      <div>
        <p class="breadcrumb">{hostLabel} <span aria-hidden="true">/</span> {serverLabel}</p>
        <h1>{activeSection || 'Overview'}</h1>
      </div>
      <div class="topbar-actions">
        <span class="mobile-context">{hostLabel} · {serverLabel}</span>
        <ActionButton kind="quiet" label="Open console" onclick={onConsole}>Console</ActionButton>
      </div>
    </header>

    <section class="content" aria-live="polite">
      <slot />
    </section>

    <nav class="bottom-nav" aria-label="Mobile navigation">
      {#each sections as section (section.id)}
        <button
          class:active={section.id === activeSection}
          type="button"
          aria-current={section.id === activeSection ? 'page' : undefined}
          onclick={() => onSection?.(section.id)}>{section.label}</button
        >
      {/each}
      <button type="button" onclick={onConsole}>Console</button>
    </nav>
  </main>
</div>

<style>
  .application-shell {
    display: grid;
    grid-template-columns: 17rem minmax(0, 1fr);
    min-height: 100vh;
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 1.3rem;
    padding: 1.35rem 1rem;
    border-right: 1px solid var(--msc-border);
    background: rgba(16, 24, 32, 0.94);
  }
  .brand-block {
    padding: 0.35rem 0.6rem;
  }
  .brand-mark {
    margin: 0;
    color: var(--msc-accent);
    font-size: 1.45rem;
    font-weight: 850;
    letter-spacing: 0.04em;
  }
  .brand-subtitle {
    margin: 0.3rem 0 0;
    color: var(--msc-subtle);
    font-size: 0.72rem;
    line-height: 1.35;
  }
  .context-card {
    display: grid;
    gap: 0.5rem;
    padding: 0.85rem;
    border: 1px solid var(--msc-border);
    border-radius: var(--msc-radius-md);
    background: var(--msc-surface);
  }
  .context-label,
  .breadcrumb {
    margin: 0;
    color: var(--msc-subtle);
    font-size: 0.72rem;
  }
  .context-server {
    margin: 0;
    color: var(--msc-muted);
    font-size: 0.85rem;
  }
  .context-card :global(.action-button) {
    justify-self: start;
    margin: 0.15rem -0.25rem -0.25rem;
    padding: 0.45rem 0.55rem;
    font-size: 0.78rem;
  }
  .section-list {
    display: grid;
    gap: 0.25rem;
  }
  .section-list button,
  .bottom-nav button {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    border: 0;
    border-radius: var(--msc-radius-sm);
    padding: 0.7rem 0.65rem;
    color: var(--msc-muted);
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .section-list button:hover,
  .section-list button.active,
  .bottom-nav button:hover,
  .bottom-nav button.active {
    color: var(--msc-text);
    background: rgba(143, 227, 207, 0.11);
  }
  .section-list button:focus-visible,
  .bottom-nav button:focus-visible {
    outline: none;
    box-shadow: var(--msc-focus);
  }
  .route-hint {
    color: var(--msc-subtle);
    font-size: 0.7rem;
  }
  .sidebar-footer {
    margin-top: auto;
  }
  .main-column {
    min-width: 0;
    background:
      radial-gradient(circle at 80% 0%, rgba(143, 227, 207, 0.08), transparent 28rem), var(--msc-bg);
  }
  .topbar {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: center;
    padding: 1.5rem clamp(1rem, 4vw, 3.5rem) 1rem;
    border-bottom: 1px solid var(--msc-border);
  }
  .breadcrumb span {
    margin: 0 0.35rem;
    color: var(--msc-subtle);
  }
  h1 {
    margin: 0.35rem 0 0;
    font-size: clamp(1.35rem, 3vw, 2rem);
    text-transform: capitalize;
  }
  .topbar-actions {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .mobile-context {
    display: none;
    color: var(--msc-muted);
    font-size: 0.75rem;
  }
  .content {
    width: min(100%, 76rem);
    margin: 0 auto;
    padding: clamp(1rem, 4vw, 2.5rem) clamp(1rem, 4vw, 3.5rem) 5rem;
  }
  .bottom-nav {
    display: none;
  }

  @media (max-width: 759px) {
    .application-shell {
      display: block;
    }
    .sidebar {
      display: none;
    }
    .topbar {
      align-items: flex-start;
    }
    .topbar-actions :global(.action-button) {
      display: none;
    }
    .mobile-context {
      display: block;
      max-width: 9rem;
      text-align: right;
    }
    .bottom-nav {
      position: fixed;
      z-index: 5;
      right: 0;
      bottom: 0;
      left: 0;
      display: grid;
      grid-auto-columns: minmax(0, 1fr);
      grid-auto-flow: column;
      gap: 0.2rem;
      overflow-x: auto;
      padding: 0.45rem;
      border-top: 1px solid var(--msc-border);
      background: rgba(16, 24, 32, 0.96);
      backdrop-filter: blur(0.8rem);
    }
    .bottom-nav button {
      display: block;
      min-width: 4.7rem;
      padding: 0.55rem 0.35rem;
      color: var(--msc-muted);
      font-size: 0.72rem;
      text-align: center;
    }
  }
</style>
