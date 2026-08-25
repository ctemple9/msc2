<script lang="ts">
  // The 8-tab strip. Selected pill = bannerColor — one of the four sanctioned
  // spots for the accent (docs/msc2/renderings/shell.html). Text-only labels:
  // the locked specimen carries icons on none but one tab, so a uniform
  // text-only strip is the faithful reading rather than an invented icon set.
  import type { PrimaryTab } from '../../navigation/primaryTabs';
  import { bannerColorAccent } from '../../styles/bannerColor';

  export let tabs: readonly (PrimaryTab & { available: boolean })[] = [];
  export let activeId: string;
  export let bannerColor: string;
  export let onSelect: (id: string) => void;
</script>

<div class="strip" role="tablist" aria-label="Server sections">
  {#each tabs as tab (tab.id)}
    <button
      type="button"
      role="tab"
      class="tab"
      class:selected={tab.id === activeId}
      aria-selected={tab.id === activeId}
      disabled={!tab.available}
      title={tab.available ? undefined : 'Not yet available'}
      style={tab.id === activeId ? `background: ${bannerColorAccent(bannerColor, 0.24)};` : ''}
      onclick={() => onSelect(tab.id)}
    >
      {tab.label}
    </button>
  {/each}
</div>

<style>
  .strip {
    display: flex;
    gap: 1px;
    padding: 3px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 9px;
    overflow-x: auto;
  }
  .tab {
    flex: 1;
    white-space: nowrap;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--msc2-text-secondary);
    background: transparent;
    border: none;
    border-radius: 6px;
    padding: 6px 10px;
    cursor: pointer;
  }
  .tab.selected {
    color: var(--msc2-text-primary);
  }
  .tab:not(:disabled):not(.selected):hover {
    color: var(--msc2-text-primary);
  }
  .tab:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
    outline-offset: -2px;
  }
  .tab:disabled {
    color: rgba(255, 255, 255, 0.25);
    cursor: not-allowed;
  }
</style>
