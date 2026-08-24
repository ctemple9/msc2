<script lang="ts">
  import { onMount } from 'svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call } from '../shared/types';
  import { demoPlayers, playerPaths, playerSearch } from './model';

  export let api: ScreenProps['api'] = undefined;
  let players = demoPlayers;
  let search = '';
  let refreshed = false;
  $: filtered = playerSearch(players, search);

  onMount(async () => {
    if (!api) return;
    const response = await call<Schema['PlayersResponseDTO']>(
      api,
      { count: players.length, players },
      playerPaths.players,
    );
    players = response.players;
  });
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Players"
    title="Online roster"
    description="This is the generic online roster only. Player profiles, skins, hidden profiles, and Bedrock allowlist controls remain a separate future capability."
    status={`${players.length} online`}
    statusTone="positive"
    actionLabel="Refresh roster"
    onAction={async () => {
      const response = await call<Schema['PlayersResponseDTO']>(
        api,
        { count: players.length, players },
        playerPaths.players,
      );
      players = response.players;
      refreshed = true;
    }}
  />
  {#if players.some((player) => 'profile' in player)}<CapabilityNotice
      title="Future profile fields ignored"
      message="The roster tolerates additive profile fields without turning them into a profile screen."
    />{/if}
  <section class="screen-card">
    <div class="inline-form">
      <div class="field">
        <label for="player-search">Find a player</label><input
          id="player-search"
          bind:value={search}
          placeholder="Name or display name"
        />
      </div>
      <span class="metric-label">{filtered.length} shown</span>
    </div>
    <table class="data-table">
      <thead><tr><th>Player</th><th>Identity</th><th>Level</th><th>State</th></tr></thead><tbody
        >{#each filtered as player (player.id)}<tr
            ><td><strong>{player.displayName}</strong></td><td>{player.name}</td><td
              >{player.level}</td
            ><td><span class="tag">Online</span></td></tr
          >{:else}<tr><td colspan="4" class="empty-row">No matching players are online.</td></tr
          >{/each}</tbody
      >
    </table>
  </section>
  {#if refreshed}<p class="muted" role="status">
      Roster refreshed without requesting profile data.
    </p>{/if}
</div>
