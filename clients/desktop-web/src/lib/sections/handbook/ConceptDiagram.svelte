<script lang="ts">
  // Flat, monochrome node/arrow diagrams standing in for MSC 1's
  // ConceptGuideDiagrams.swift (per-page colored icon tiles + staggered
  // fade-in animation -- exactly antiAIslop.md tells #1/#6/#13). The
  // content pages mark every diagram "client-owned rendering required"
  // (content/guides/concept-guide.json); this renders that gap with plain
  // labeled boxes instead of leaving it blank or inventing prose.
  export let diagram: string;

  const worlds = ['World A', 'World B', 'World C'];
</script>

<div class="diagram" role="presentation" aria-hidden="true">
  {#if diagram === 'server-worlds'}
    <div class="node solid">Server</div>
    <div class="arrow">↓</div>
    <div class="row">
      {#each worlds as world (world)}<div class="node small">{world}</div>{/each}
    </div>
  {:else if diagram === 'connections'}
    <div class="row">
      <div class="node small">Port Forward</div>
      <div class="node small">Playit Tunnel</div>
    </div>
    <div class="arrow">↓</div>
    <div class="node solid">Your IP + Port</div>
    <div class="arrow">↓</div>
    <div class="row">
      <div class="node small">Direct Share</div>
      <div class="node small">Xbox Broadcast</div>
    </div>
  {:else if diagram === 'java-bedrock'}
    <div class="row">
      <div class="node column">
        <span class="node-title">Java Server</span>
        <span class="node-line">Java players</span>
        <span class="node-line">+ Bedrock, with Geyser</span>
      </div>
      <div class="node column">
        <span class="node-title">Bedrock Server</span>
        <span class="node-line">Bedrock players only</span>
      </div>
    </div>
  {:else if diagram === 'world-slots'}
    <div class="node solid">Server</div>
    <div class="arrow">↓</div>
    <div class="stack">
      {#each worlds as world, i (world)}
        <div class="node row-node" class:active={i === 0}>
          {world}{#if i === 0}<span class="tag">Active</span>{/if}
        </div>
      {/each}
    </div>
  {:else if diagram === 'active-world-routing'}
    <div class="node small">Player</div>
    <div class="arrow">↓</div>
    <div class="node solid">Server</div>
    <div class="arrow">↓</div>
    <div class="stack">
      {#each worlds as world, i (world)}
        <div class="node row-node" class:active={i === 0}>
          {world}{#if i === 0}<span class="tag">Active</span>{/if}
        </div>
      {/each}
    </div>
  {:else if diagram === 'settings-separation'}
    <div class="row align-top">
      <div class="node column">
        <span class="node-title">Server Settings</span>
        <span class="node-line">Port</span>
        <span class="node-line">Version</span>
        <span class="node-line">Network</span>
      </div>
      <div class="node column">
        <span class="node-title">World Settings</span>
        <span class="node-line">Game mode</span>
        <span class="node-line">Difficulty</span>
        <span class="node-line">Player data</span>
      </div>
    </div>
  {:else}
    <div class="row">
      <div class="node small">Server</div>
      <div class="arrow inline">→</div>
      <div class="node small">Worlds</div>
      <div class="arrow inline">→</div>
      <div class="node small">Active world</div>
    </div>
  {/if}
</div>

<style>
  .diagram {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 24px 16px;
    background: var(--msc2-tier-chrome);
    border-radius: var(--msc2-radius-3);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .row.align-top {
    align-items: flex-start;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
    max-width: 15rem;
  }
  .node {
    border: 1px solid var(--msc2-hairline);
    border-radius: var(--msc2-radius-2);
    padding: 8px 14px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    background: var(--msc2-tier-content);
    text-align: center;
  }
  .node.solid {
    color: var(--msc2-text-primary);
    font-weight: 500;
    background: var(--msc2-neutral-elevated);
  }
  .node.small {
    font-size: 11px;
  }
  .node.row-node {
    display: flex;
    align-items: center;
    justify-content: space-between;
    text-align: left;
  }
  .node.row-node.active {
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-primary);
  }
  .node.column {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 9rem;
    text-align: left;
  }
  .node-title {
    color: var(--msc2-text-primary);
    font-weight: 500;
    font-size: 12px;
    margin-bottom: 2px;
  }
  .node-line {
    font-size: 11px;
    color: var(--msc2-text-secondary);
  }
  .tag {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .arrow {
    color: var(--msc2-text-tertiary);
    font-size: 15px;
    line-height: 1;
  }
  .arrow.inline {
    font-size: 13px;
  }
</style>
