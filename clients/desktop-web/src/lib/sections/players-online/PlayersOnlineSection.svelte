<script lang="ts">
  // Ports DetailsPlayersTabView.swift: Online Now / Seen This Session,
  // Session Log, the Bedrock Allowlist card (Bedrock only), and Player Data
  // (profiles -> detail sheet). Like HomeSection, this keeps screen behavior
  // independent of the Tauri shell.
  import { onDestroy, onMount } from 'svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, mutate } from '../shared/types';
  import OnlineNowCard from './OnlineNowCard.svelte';
  import SessionLogCard from './SessionLogCard.svelte';
  import BedrockAllowlistCard from './BedrockAllowlistCard.svelte';
  import PlayerDataCard from './PlayerDataCard.svelte';
  import PlayerDetailSheet from './PlayerDetailSheet.svelte';
  import PlayerActionsSheet from './PlayerActionsSheet.svelte';
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
  export let active = true;
  export let permissions: readonly string[] = [];

  let online: Schema['PlayersResponseDTO'] = { count: 0, players: [] };
  let profiles: Schema['PlayerProfileDTO'][] = [];
  let profilesLoading = true;
  let sessionEvents: SessionEvent[] = [];
  let allowlist: Schema['AllowlistResponseDTO'] = { serverType: 'bedrock', entries: [] };
  let selectedProfile: Schema['PlayerProfileDTO'] | undefined;
  let selectedOnlinePlayer: Schema['PlayerDTO'] | undefined;
  let selectedAction: 'message' | undefined;
  let sheetHostId = hostId;
  let sheetServerId = serverId;
  let capabilities: Schema['CapabilitiesDTO'] | undefined;
  let capabilitiesKey = '';
  let capabilitiesApi: ScreenProps['api'];
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
  $: actionsAvailable = capabilities?.playerActions === true;
  $: canSendCommands =
    permissions.length === 0 ||
    permissions.includes('serverControl') ||
    permissions.includes('admin');
  $: canEditAccess =
    permissions.length === 0 || permissions.includes('players') || permissions.includes('admin');
  $: if (selectedOnlinePlayer && (sheetHostId !== hostId || sheetServerId !== serverId))
    selectedOnlinePlayer = undefined;
  $: if (selectedOnlinePlayer && !active) selectedOnlinePlayer = undefined;

  function openPlayer(player: Schema['PlayerDTO'], action?: 'message'): void {
    if (!actionsAvailable) return;
    sheetHostId = hostId;
    sheetServerId = serverId;
    selectedAction = action;
    selectedOnlinePlayer = player;
  }

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
  async function loadCapabilities(): Promise<void> {
    const targetApi = api;
    if (!targetApi) return;
    const key = `${hostId}:${serverId}`;
    if (capabilitiesKey === key && capabilitiesApi === targetApi) return;
    capabilitiesKey = key;
    capabilitiesApi = targetApi;
    capabilities = undefined;
    const result = await call<Schema['CapabilitiesDTO'] | undefined>(
      targetApi,
      undefined,
      '/v1/capabilities',
    );
    if (capabilitiesKey === key && capabilitiesApi === targetApi) {
      capabilities = result;
      if (!result) capabilitiesKey = '';
    }
  }
  async function loadAllowlist(): Promise<void> {
    if (!isBedrock) return;
    allowlist = await call(api, allowlist, playerPaths.allowlist);
  }

  async function loadAll(): Promise<void> {
    await Promise.all([loadServers(), loadWorlds(), loadCapabilities()]);
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
  let mounted = false;

  function startPolling(): void {
    if (refreshTimer) return;
    void loadAll();
    refreshTimer = setInterval(() => void loadAll(), 8000);
  }

  function stopPolling(): void {
    if (refreshTimer) clearInterval(refreshTimer);
    refreshTimer = undefined;
  }

  onMount(() => {
    mounted = true;
    if (active) startPolling();
  });
  onDestroy(() => {
    mounted = false;
    stopPolling();
  });

  $: if (mounted && active) startPolling();
  $: if (mounted && !active) stopPolling();
</script>

<div class="players">
  <OnlineNowCard
    players={online.players}
    seenThisSession={seenThisSession(visibleSessionEvents)}
    onRefresh={() => void loadOnline()}
    onPlayer={(player) => openPlayer(player)}
    onMessage={(player) => openPlayer(player, 'message')}
    {actionsAvailable}
    actionsUnsupported={capabilities !== undefined && !actionsAvailable}
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
  />
</div>

{#if selectedOnlinePlayer && activeServer && actionsAvailable}
  <PlayerActionsSheet
    player={selectedOnlinePlayer}
    onlinePlayers={online.players}
    {serverId}
    serverType={activeServer.serverType}
    minecraftVersion={capabilities?.worldSettings?.context.minecraftVersion ?? undefined}
    {api}
    initialAction={selectedAction}
    {canSendCommands}
    {canEditAccess}
    online={online.players.some((player) => player.name === selectedOnlinePlayer?.name)}
    onClose={() => (selectedOnlinePlayer = undefined)}
    onAllowlistChanged={(entries) => (allowlist = { ...allowlist, entries })}
  />
{/if}

{#if selectedProfile}
  <PlayerDetailSheet
    profile={selectedProfile}
    {api}
    serverRunning={worlds.serverRunning}
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
