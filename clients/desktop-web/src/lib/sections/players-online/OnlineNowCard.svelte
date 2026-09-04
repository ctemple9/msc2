<script lang="ts">
  // Ports DetailsPlayersTabView's onlineNowCard: two columns, live roster on
  // the left, every distinct name seen this session on the right (a dot
  // shows which of those are still online).
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema } from '../shared/types';

  export let players: readonly Schema['PlayerDTO'][] = [];
  export let seenThisSession: readonly string[] = [];
  export let onRefresh: (() => void) | undefined = undefined;

  $: onlineNames = new Set(players.map((player) => player.name));
</script>

<Card>
  <div class="header">
    <div class="overline">
      <span class="msc2-type-overline">Players</span>
    </div>
    <div class="header-actions">
      <span class="count">{players.length} online</span>
      {#if onRefresh}<Button size="sm" onclick={onRefresh}>Refresh</Button>{/if}
    </div>
  </div>

  <div class="columns">
    <div class="column">
      <span class="label">Online Now</span>
      {#if players.length === 0}
        <p class="empty">No players online.</p>
      {:else}
        <ul class="list">
          {#each players as player (player.uuid ?? player.name)}
            <li class="row">
              <span class="dot online" aria-hidden="true"></span>
              <span class="name">{player.displayName || player.name}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="column">
      <span class="label">Seen This Session</span>
      {#if seenThisSession.length === 0}
        <p class="empty">No history yet.</p>
      {:else}
        <ul class="list">
          {#each seenThisSession as name (name)}
            <li class="row">
              <span class="dot" class:online={onlineNames.has(name)} aria-hidden="true"></span>
              <span class="name" class:muted={!onlineNames.has(name)}>{name}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</Card>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .count {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
  }
  .column {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .column + .column {
    padding-left: 20px;
    border-left: 1px solid var(--msc2-hairline-subtle);
  }
  .label {
    font-size: 11px;
    font-weight: 500;
    color: var(--msc2-text-tertiary);
  }
  .empty {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
    background: rgba(255, 255, 255, 0.25);
  }
  .dot.online {
    background: var(--msc2-status-ok);
  }
  .name {
    font-size: 12px;
    color: var(--msc2-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name.muted {
    color: var(--msc2-text-tertiary);
  }
</style>
