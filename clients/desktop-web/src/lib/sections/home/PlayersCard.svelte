<script lang="ts">
  // MSC 1 OverviewPlayersStripView, simplified to the generic online roster
  // the agent actually reports (Schema['PlayerDTO']: name, uuid — P11.11's
  // scope already excludes the profile/quick-actions/hidden-player system
  // from this client; the Players tab, P12.3, is where that lands). Java
  // heads render from the same minotar.net probe P12.1a's sidebar avatar
  // uses; Bedrock names show as initials since there is no public skin API.
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { Schema } from '../shared/types';

  export let players: readonly Schema['PlayerDTO'][] = [];
  export let maxPlayers: number | undefined = undefined;
  export let isBedrock = false;
</script>

<Card padding="14px 16px">
  <div class="header">
    <div class="overline">
      <span class="msc2-type-overline">Players</span>
    </div>
    <span class="count"
      >{players.length}{maxPlayers !== undefined ? ` / ${maxPlayers}` : ''} online</span
    >
  </div>

  {#if players.length === 0}
    <div class="empty">
      <Icon name="people" size={18} />
      <span>No players online.</span>
    </div>
  {:else}
    <div class="roster">
      {#each players as player (player.uuid ?? player.name)}
        <div class="chip">
          {#if isBedrock}
            <span class="initial">{player.name.slice(0, 1).toUpperCase()}</span>
          {:else}
            <img
              class="head"
              src={`https://minotar.net/avatar/${encodeURIComponent(player.name)}/28`}
              alt=""
              loading="lazy"
            />
          {/if}
          <span class="name">{player.name}</span>
        </div>
      {/each}
    </div>
  {/if}
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
  .count {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .empty {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    padding: 8px 0;
  }
  .roster {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    padding: 3px 10px 3px 3px;
  }
  .head {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    image-rendering: pixelated;
  }
  .initial {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-secondary);
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .name {
    font-size: 12px;
    color: var(--msc2-text-primary);
  }
</style>
