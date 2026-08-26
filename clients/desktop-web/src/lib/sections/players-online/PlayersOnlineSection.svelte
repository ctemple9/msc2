<script lang="ts">
  // Ports DetailsPlayersTabView.swift: Online Now / Seen This Session,
  // Session Log, the Bedrock Allowlist card (Bedrock only), and Player Data
  // (profiles -> detail sheet). Same shared-component pattern HomeSection
  // uses (D-003: one component for both Tauri and the browser).
  import { onDestroy, onMount } from 'svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, mutate } from '../shared/types';
  import OnlineNowCard from './OnlineNowCard.svelte';
  import SessionLogCard from './SessionLogCard.svelte';
  import BedrockAllowlistCard from './BedrockAllowlistCard.svelte';
  import PlayerDataCard from './PlayerDataCard.svelte';
  import PlayerDetailSheet from './PlayerDetailSheet.svelte';
  import {
    clearSessionLog,
    playerPaths,
    readSessionLogClearedAt,
    seenThisSession,
    sessionEventsFromConsole,
    sessionEventsFromLog,
    type SessionEvent,
  } from './model';

  export let api: ScreenProps['api'] = undefined;
  export let hostId = 'local-agent';
  export let serverId = 'survival';

  let online: Schema['PlayersResponseDTO'] = { count: 0, players: [] };
  let profiles: Schema['PlayerProfileDTO'][] = [];
  let profilesLoading = true;
  let sessionEvents: SessionEvent[] = [];
  let allowlist: Schema['AllowlistResponseDTO'] = { serverType: 'bedrock', entries: [] };
  let selectedProfile: Schema['PlayerProfileDTO'] | undefined;
  let servers: Schema['ServerDTO'][] = [];
  let worlds: Schema['WorldSlotsResponseDTO'] = { serverRunning: false, slots: [] };

  $: activeServer = servers.find((server) => server.id === serverId);
  $: isBedrock = activeServer?.serverType === 'bedrock';
  $: worldName = worlds.slots.find((slot) => slot.id === worlds.activeSlotId)?.name;

  $: clearedAt = readSessionLogClearedAt(hostId, serverId);
  // Java's session log is cleared for real on the backend, so its events
  // are already fresh; the clearedAt cutoff only applies to Bedrock's
  // client-derived console-tail fallback (see model.ts).
  $: visibleSessionEvents = isBedrock
    ? sessionEvents.filter((event) => new Date(event.ts).getTime() > clearedAt)
    : sessionEvents;
  $: onlineNames = new Set(online.players.map((player) => player.name));

  async function loadOnline(): Promise<void> {
    online = await call(api, online, playerPaths.players);
  }
  async function loadProfiles(): Promise<void> {
    const response = await call<Schema['PlayerProfilesResponseDTO']>(
      api,
      { profiles, isLoadingStats: false },
      playerPaths.profiles,
    );
    profiles = response.profiles;
    profilesLoading = false;
  }
  async function loadSessionEvents(): Promise<void> {
    if (isBedrock) {
      const lines = await call<Schema['ConsoleLineDTO'][]>(api, [], '/v1/console/tail?n=200');
      sessionEvents = sessionEventsFromConsole(lines);
      return;
    }
    const response = await call<Schema['SessionLogResponseDTO']>(
      api,
      { events: [] },
      playerPaths.sessionLog,
    );
    sessionEvents = sessionEventsFromLog(response.events);
  }
  async function loadServers(): Promise<void> {
    servers = await call(api, servers, '/v1/servers');
  }
  async function loadWorlds(): Promise<void> {
    worlds = await call(api, worlds, '/v1/worlds');
  }
  async function loadAllowlist(): Promise<void> {
    if (!isBedrock) return;
    allowlist = await call(api, allowlist, playerPaths.allowlist);
  }

  async function loadAll(): Promise<void> {
    await Promise.all([loadServers(), loadWorlds()]);
    await Promise.all([loadOnline(), loadProfiles(), loadSessionEvents(), loadAllowlist()]);
  }

  async function onClearSessionLog(): Promise<void> {
    if (isBedrock) {
      clearSessionLog(hostId, serverId);
      clearedAt = readSessionLogClearedAt(hostId, serverId);
      return;
    }
    await mutate<Schema['SessionLogResponseDTO']>(api, playerPaths.sessionLogClear);
    await loadSessionEvents();
  }

  async function onAddAllowlistEntry(name: string): Promise<void> {
    const result = await mutate<Schema['AllowlistMutationResultDTO']>(api, playerPaths.allowlist, {
      action: 'add',
      name,
    });
    allowlist = { ...allowlist, entries: result.entries };
  }
  async function onRemoveAllowlistEntry(name: string): Promise<void> {
    const result = await mutate<Schema['AllowlistMutationResultDTO']>(api, playerPaths.allowlist, {
      action: 'remove',
      name,
    });
    allowlist = { ...allowlist, entries: result.entries };
  }

  function onProfilesMutated(updated: readonly Schema['PlayerProfileDTO'][]): void {
    const byId = new Map(updated.map((profile) => [profile.id, profile]));
    profiles = profiles.map((profile) => byId.get(profile.id) ?? profile);
    for (const profile of updated) {
      if (!profiles.some((existing) => existing.id === profile.id))
        profiles = [...profiles, profile];
    }
    if (selectedProfile) selectedProfile = byId.get(selectedProfile.id) ?? selectedProfile;
  }

  let refreshTimer: ReturnType<typeof setInterval> | undefined;

  onMount(() => {
    void loadAll();
    refreshTimer = setInterval(() => void loadAll(), 8000);
  });
  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
  });
</script>

<div class="players">
  <OnlineNowCard
    players={online.players}
    seenThisSession={seenThisSession(visibleSessionEvents)}
    onRefresh={() => void loadOnline()}
  />

  {#if isBedrock}
    <BedrockAllowlistCard
      entries={allowlist.entries}
      onAdd={(name) => void onAddAllowlistEntry(name)}
      onRemove={(name) => void onRemoveAllowlistEntry(name)}
      onReload={() => void loadAllowlist()}
    />
  {/if}

  <SessionLogCard
    events={visibleSessionEvents}
    {onlineNames}
    onClear={() => void onClearSessionLog()}
  />

  <PlayerDataCard
    {profiles}
    loading={profilesLoading}
    activeWorldName={worldName}
    onSelect={(profile) => (selectedProfile = profile)}
    onReload={() => void loadProfiles()}
  />
</div>

{#if selectedProfile}
  <PlayerDetailSheet
    profile={selectedProfile}
    {api}
    onClose={() => (selectedProfile = undefined)}
    onMutated={onProfilesMutated}
    onDeleted={() => {
      if (selectedProfile)
        profiles = profiles.filter((profile) => profile.id !== selectedProfile?.id);
      selectedProfile = undefined;
    }}
  />
{/if}

<style>
  .players {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
</style>
