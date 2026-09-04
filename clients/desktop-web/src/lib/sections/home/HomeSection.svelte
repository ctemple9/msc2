<script lang="ts">
  // MSC 1 DetailsOverviewTabView, rebuilt to the S0 disciplined system
  // (docs/msc2/antiAIslop.md). Zones: Status (connection + live stats),
  // Server Health, Activity (players / active world / chat), Notes — the
  // same order as MSC 1's Overview tab. This is the shared component both
  // Tauri and the browser load (D-003); no desktop-only branch exists here.
  import { onDestroy, onMount } from 'svelte';
  import Card from '../../components/base/Card.svelte';
  import ConnectionCard from './ConnectionCard.svelte';
  import LiveStatsCard from './LiveStatsCard.svelte';
  import HealthGrid from './HealthGrid.svelte';
  import PlayersCard from './PlayersCard.svelte';
  import ActiveWorldCard from './ActiveWorldCard.svelte';
  import ChatCard from './ChatCard.svelte';
  import { parseChatFeed, type ChatFeedMessage } from './chatFeed';
  import { readNotes, writeNotes } from './notes';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, mutate } from '../shared/types';

  export let api: ScreenProps['api'] = undefined;
  export let hostId = 'local-agent';
  export let serverId = 'survival';
  export let active = true;
  export let healthRefreshVersion = 0;
  export let onWorlds: (() => void) | undefined = undefined;
  export let addressesVisible = false;
  export let onToggleAddresses: () => void = () => undefined;

  let health: Schema['HealthResponseDTO'] = {
    cards: [],
    overallSeverity: 'gray',
    serverName: '',
    serverRunning: false,
    serverType: '',
  };
  let connectivity: Schema['ConnectivityResponseDTO'] | undefined;
  let performance: Schema['PerformanceSnapshotDTO'] | undefined;
  let servers: Schema['ServerDTO'][] = [];
  let geyser: Schema['GeyserConfigResponseDTO'] | undefined;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let players: Schema['PlayersResponseDTO'] = { count: 0, players: [] };
  let worlds: Schema['WorldSlotsResponseDTO'] = { serverRunning: false, slots: [] };
  let settings: Schema['SettingsResponseDTO'] | undefined;
  let chatMessages: ChatFeedMessage[] = [];
  let notesText = '';
  let notesTimer: ReturnType<typeof setTimeout> | undefined;

  $: activeServer = servers.find((s) => s.id === serverId);
  $: activeSlot = worlds.slots.find((s) => s.id === worlds.activeSlotId);
  $: settingField = (key: string): string | undefined =>
    settings?.sections.flatMap((s) => s.fields).find((f) => f.key === key)?.value;

  async function loadHealth(): Promise<void> {
    health = await call(api, health, '/v1/health');
  }

  async function loadAll(): Promise<void> {
    await loadHealth();
    connectivity = await call(api, connectivity, '/v1/connectivity');
    performance = await call(api, performance, '/v1/performance');
    servers = await call(api, servers, '/v1/servers');
    geyser = await call(api, geyser, '/v1/config/geyser');
    playit = await call(api, playit, '/v1/playit');
    players = await call(api, players, '/v1/players');
    worlds = await call(api, worlds, '/v1/worlds');
    settings = await call(api, settings, '/v1/settings');
    const lines = await call<Schema['ConsoleLineDTO'][]>(api, [], '/v1/console/tail?n=200');
    chatMessages = parseChatFeed(lines);
  }

  async function createBackup(): Promise<void> {
    try {
      await mutate(api, '/v1/backups/now');
    } catch {
      // Surfaced fully by the Worlds/Backups tab; this button is a shortcut.
    }
  }

  function onNotesInput(): void {
    if (notesTimer) clearTimeout(notesTimer);
    notesTimer = setTimeout(() => writeNotes(hostId, serverId, notesText), 400);
  }

  let refreshTimer: ReturnType<typeof setInterval> | undefined;
  let mounted = false;
  let appliedHealthRefreshVersion = 0;

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
    notesText = readNotes(hostId, serverId);
    if (active) startPolling();
  });

  onDestroy(() => {
    mounted = false;
    stopPolling();
    if (notesTimer) clearTimeout(notesTimer);
  });

  $: if (mounted && active) startPolling();
  $: if (mounted && !active) stopPolling();
  $: if (mounted && active && healthRefreshVersion !== appliedHealthRefreshVersion) {
    appliedHealthRefreshVersion = healthRefreshVersion;
    void loadHealth();
  }

  $: isBedrock = activeServer?.serverType === 'bedrock';
</script>

<div class="overview">
  <section class="zone">
    <div class="status-row">
      <ConnectionCard
        serverType={activeServer?.serverType}
        gamePort={activeServer?.gamePort}
        bedrockPort={activeServer?.bedrockPort}
        hostAddress={activeServer?.hostAddress}
        {geyser}
        {connectivity}
        {playit}
        {addressesVisible}
        {onToggleAddresses}
      />
      <LiveStatsCard snapshot={performance} />
    </div>
  </section>

  <section class="zone">
    <HealthGrid cards={health.cards} />
  </section>

  <section class="zone">
    <div class="overline standalone">
      <span class="msc2-type-overline">Activity</span>
    </div>
    <div class="activity-row">
      <PlayersCard
        players={players.players}
        maxPlayers={settingField('max-players') ? Number(settingField('max-players')) : undefined}
        {isBedrock}
      />
      <div class="world-col">
        <ActiveWorldCard
          {api}
          slot={activeSlot}
          {isBedrock}
          difficulty={settingField('difficulty')}
          gamemode={settingField('gamemode')}
          onSwitch={() => onWorlds?.()}
          onBackup={() => void createBackup()}
        />
      </div>
      <ChatCard messages={chatMessages} serverRunning={health.serverRunning} />
    </div>
  </section>

  <section class="zone">
    <div class="overline standalone">
      <span class="msc2-type-overline">Notes</span>
    </div>
    <Card padding="14px 16px">
      <div class="notes-header">
        <span class="notes-label">For this server</span>
      </div>
      <textarea
        class="notes"
        bind:value={notesText}
        oninput={onNotesInput}
        placeholder="Notes for this server…"
      ></textarea>
      <p class="notes-hint">Auto-saved as you type. Visible only in this app.</p>
    </Card>
  </section>
</div>

<style>
  .overview {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .overline.standalone {
    color: var(--msc2-text-tertiary);
  }
  .status-row {
    display: grid;
    grid-template-columns: 1.4fr 1fr;
    gap: 10px;
    align-items: stretch;
  }
  .activity-row {
    display: grid;
    grid-template-columns: 1fr 240px 1fr;
    gap: 10px;
    align-items: stretch;
  }
  .world-col {
    display: flex;
  }
  .world-col :global(> *) {
    flex: 1;
  }
  .notes-header {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
    margin-bottom: 8px;
  }
  .notes-label {
    font-size: 11px;
  }
  .notes {
    width: 100%;
    box-sizing: border-box;
    min-height: 80px;
    resize: vertical;
    font-family: var(--msc2-font-mono);
    font-size: 12px;
    color: var(--msc2-text-primary);
    background: var(--msc2-tier-terminal);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    padding: 10px;
    outline: none;
  }
  .notes:focus-visible {
    border-color: var(--msc2-hairline-field-focus);
  }
  .notes-hint {
    margin: 6px 0 0;
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }

  @media (max-width: 900px) {
    .status-row,
    .activity-row {
      grid-template-columns: 1fr;
    }
  }
</style>
