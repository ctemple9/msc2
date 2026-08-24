<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, dateLabel } from '../shared/types';
  import { demoPlayers, playerPaths, playerSearch } from './model';

  export let api: ScreenProps['api'] = undefined;
  let players = demoPlayers;
  let search = '';
  let session: Schema['SessionLogResponseDTO'] = { events: [] };
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
    session = await call(api, session, playerPaths.sessions);
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
  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Recent join and leave events</h3>
      <ActionButton kind="quiet" label="Open session history" onclick={() => (refreshed = true)}
        >Refresh</ActionButton
      >
    </div>
    {#if session.events.length}<div class="notification-feed">
        {#each session.events.slice(-8).reverse() as event (event.id)}<div class="notification-row">
            <strong>{event.playerName}</strong><span>{dateLabel(event.timestamp)}</span>
            <p>{event.eventType}</p>
          </div>{/each}
      </div>{:else}<p class="muted">No session events loaded yet.</p>{/if}
  </section>
  {#if refreshed}<p class="muted" role="status">
      Roster refreshed without requesting profile data.
    </p>{/if}
</div>
