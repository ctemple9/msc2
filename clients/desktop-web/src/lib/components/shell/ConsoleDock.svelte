<script lang="ts">
  // The docked console frame — shape only. Filter chips, buffered log, and
  // command input become real in P12.10; this step wires collapse/expand and
  // manual resize (MSC 1 ContentView.swift's consoleDivider drag).
  // docs/msc2/renderings/shell.html, MSC 1 ConsoleView.
  export let collapsed = false;
  export let onToggle: () => void;
  // Explicit pixel height while expanded — undefined lets the collapsed
  // header-only row size itself naturally.
  export let height: number | undefined = undefined;

  const filters = ['All', 'Server', 'Plugins', 'Warnings', 'Controller', 'Commands', 'Custom'];
  let activeFilter = 'All';
</script>

<div
  class="dock"
  class:expanded={!collapsed}
  style={!collapsed && height !== undefined ? `height: ${height}px;` : ''}
>
  <div class="dock-header">
    <button
      type="button"
      class="collapse"
      aria-label={collapsed ? 'Show console' : 'Hide console'}
      onclick={onToggle}
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <path
          d={collapsed ? 'M6 9l6 6 6-6' : 'M6 15l6-6 6 6'}
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <span class="label">Console</span>
    {#if !collapsed}
      <div class="filters">
        {#each filters as filter (filter)}
          <button
            type="button"
            class="chip"
            class:active={filter === activeFilter}
            onclick={() => (activeFilter = filter)}>{filter}</button
          >
        {/each}
      </div>
    {/if}
  </div>

  {#if !collapsed}
    <div class="body">
      <p class="line">Connect to a running server to see console output here.</p>
    </div>
    <div class="input-row">
      <input type="text" class="command" placeholder="Enter command…" disabled />
      <button type="button" class="send" disabled>Send</button>
    </div>
  {/if}
</div>

<style>
  .dock {
    flex-shrink: 0;
    background: var(--msc2-tier-terminal);
    border-top: 1px solid var(--msc2-hairline-subtle);
    padding: 8px 12px;
    box-sizing: border-box;
  }
  .dock.expanded {
    display: flex;
    flex-direction: column;
  }
  .dock-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .collapse {
    display: inline-flex;
    color: rgba(255, 255, 255, 0.5);
    background: transparent;
    border: none;
    padding: 2px;
    cursor: pointer;
  }
  .collapse:hover {
    color: rgba(255, 255, 255, 0.85);
  }
  .collapse:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
  }
  .label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.8px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .filters {
    display: flex;
    gap: 2px;
    overflow-x: auto;
  }
  .chip {
    font-size: 10px;
    color: var(--msc2-text-secondary);
    background: transparent;
    border: none;
    border-radius: 5px;
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
  }
  .chip.active {
    color: rgba(255, 255, 255, 0.85);
    background: var(--msc2-neutral-elevated);
  }
  .body {
    flex: 1;
    min-height: 0;
    margin-top: 7px;
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    color: var(--msc2-text-secondary);
    line-height: 1.5;
    overflow-y: auto;
  }
  .input-row {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 7px;
  }
  .command {
    flex: 1;
    box-sizing: border-box;
    font-family: inherit;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 7px;
    padding: 5px 9px;
  }
  .command:disabled {
    cursor: not-allowed;
  }
  .send {
    font-size: 11px;
    font-weight: 600;
    color: var(--msc2-neutral-fill-ink);
    background: var(--msc2-neutral-fill);
    border: none;
    border-radius: 7px;
    padding: 5px 12px;
    cursor: pointer;
  }
  .send:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
