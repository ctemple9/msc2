<script lang="ts">
  // Ports MSC 1 QuickCommandsView.swift in full -- live, in-session shortcuts
  // sent as raw console commands through the existing POST /v1/command
  // route, not persisted server.properties edits (that's SettingsSection's
  // job). Command strings read verbatim from
  // AppViewModel+ServerControls.swift:475-528, not guessed.
  //
  // Bedrock branching is real and manual, not something /v1/command does for
  // us: crates/msc-agent/src/routes/commands.rs forwards whatever string it
  // gets straight to the sidecar with no translation (only a leading-slash
  // strip), and msc-domain/src/commands.rs's own command catalog marks
  // "whitelist"/"save-all"/"reload" as supports_bedrock: false and
  // "allowlist" as supports_java: false -- confirming the openapi contract's
  // x-notes claim ("allowlist/save/operator commands are translated...
  // behind this route") does not hold at this layer. So this component keeps
  // the oracle's own whitelist->allowlist and save-all->save hold/save
  // resume branching client-side, exactly as MSC 1 does.
  import Select from '../../base/Select.svelte';
  import Toggle from '../../base/Toggle.svelte';
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call, errorMessage, mutate } from '../../../sections/shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let activeServerId: string | undefined = undefined;
  export let running = false;
  export let isBedrock = false;
  export let canControl = true;

  const TIME_PRESETS = [
    { value: '1000', label: 'Dawn' },
    { value: '13000', label: 'Dusk' },
    { value: '18000', label: 'Night' },
  ] as const;
  const WEATHER_PRESETS = [
    { value: 'clear', label: 'Clear' },
    { value: 'rain', label: 'Rain' },
    { value: 'thunder', label: 'Storm' },
  ] as const;
  const DIFFICULTY_OPTIONS = [
    { value: 'peaceful', label: 'Peaceful' },
    { value: 'easy', label: 'Easy' },
    { value: 'normal', label: 'Normal' },
    { value: 'hard', label: 'Hard' },
  ];
  const GAMEMODE_OPTIONS = [
    { value: 'survival', label: 'Survival' },
    { value: 'creative', label: 'Creative' },
    { value: 'adventure', label: 'Adventure' },
    { value: 'spectator', label: 'Spectator' },
  ];

  let performance: Schema['PerformanceSnapshotDTO'] | undefined;
  let players: Schema['PlayersResponseDTO'] | undefined;
  let difficulty = 'normal';
  let gamemode = 'survival';
  let whitelistEnabled = false;
  let notice = '';
  let loadedForServerId: string | undefined;

  $: tps = performance?.tps1m?.value;
  $: onlineCount = players?.count ?? 0;
  $: disabled = !running || !canControl;

  $: if (activeServerId !== loadedForServerId) {
    loadedForServerId = activeServerId;
    void load();
  }

  async function load(): Promise<void> {
    performance = await call(api, performance, '/v1/performance');
    players = await call(api, players, '/v1/players');
    const settings = await call<Schema['SettingsResponseDTO'] | undefined>(
      api,
      undefined,
      '/v1/settings',
    );
    const fields = settings?.sections.flatMap((section) => section.fields) ?? [];
    difficulty = fields.find((field) => field.key === 'difficulty')?.value ?? difficulty;
    gamemode = fields.find((field) => field.key === 'gamemode')?.value ?? gamemode;
    whitelistEnabled = fields.find((field) => field.key === 'white-list')?.value === 'true';
  }

  async function sendCommand(command: string): Promise<void> {
    try {
      await mutate(api, '/v1/command', { command });
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  function applyDifficulty(value: string): void {
    difficulty = value;
    void sendCommand(`difficulty ${value}`);
  }

  function applyGamemode(value: string): void {
    gamemode = value;
    void sendCommand(`defaultgamemode ${value}`);
  }

  function setWhitelist(enabled: boolean): void {
    whitelistEnabled = enabled;
    void sendCommand(
      isBedrock
        ? enabled
          ? 'allowlist on'
          : 'allowlist off'
        : enabled
          ? 'whitelist on'
          : 'whitelist off',
    );
  }

  function saveAll(): void {
    if (isBedrock) {
      void sendCommand('save hold');
      setTimeout(() => void sendCommand('save resume'), 1000);
    } else {
      void sendCommand('save-all');
    }
  }

  function reload(): void {
    void sendCommand('reload');
  }
</script>

<div class="quick-commands">
  {#if !running}
    <p class="hint">Start the server to use Quick Commands.</p>
  {:else}
    <div class="stat-strip">
      <span class="stat">{onlineCount} online</span>
      {#if tps !== undefined}<span class="stat">{tps.toFixed(1)} TPS</span>{/if}
    </div>
  {/if}

  <div class="block" class:inactive={disabled}>
    <p class="overline">World</p>
    <p class="sub-label">Time of Day</p>
    <div class="button-row">
      {#each TIME_PRESETS as preset (preset.value)}
        <button
          type="button"
          class="pill"
          {disabled}
          onclick={() => sendCommand(`time set ${preset.value}`)}
        >
          {preset.label}
        </button>
      {/each}
    </div>
    <p class="sub-label">Weather</p>
    <div class="button-row">
      {#each WEATHER_PRESETS as preset (preset.value)}
        <button
          type="button"
          class="pill"
          {disabled}
          onclick={() => sendCommand(`weather ${preset.value}`)}
        >
          {preset.label}
        </button>
      {/each}
    </div>

    <p class="overline">Settings</p>
    <div class="field-row">
      <span class="field-label">Difficulty</span>
      <Select
        options={DIFFICULTY_OPTIONS}
        value={difficulty}
        {disabled}
        onchange={applyDifficulty}
      />
    </div>
    <div class="field-row">
      <span class="field-label">Gamemode</span>
      <Select options={GAMEMODE_OPTIONS} value={gamemode} {disabled} onchange={applyGamemode} />
    </div>
    <div class="field-row toggle-row">
      <Toggle checked={whitelistEnabled} label="Whitelist" {disabled} onchange={setWhitelist} />
      <span class="field-label">Whitelist</span>
    </div>

    <p class="overline">Actions</p>
    <div class="button-row">
      <button type="button" class="pill" {disabled} onclick={saveAll}>Save All</button>
      {#if !isBedrock}
        <button type="button" class="pill" {disabled} onclick={reload}>Reload</button>
      {/if}
    </div>
  </div>

  {#if notice}<p class="notice" role="status">{notice}</p>{/if}
</div>

<style>
  .quick-commands {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    line-height: 1.5;
  }
  .stat-strip {
    display: flex;
    gap: 8px;
  }
  .stat {
    font-size: 10px;
    font-weight: 600;
    color: var(--msc2-text-secondary);
    background: rgba(255, 255, 255, 0.06);
    padding: 3px 7px;
    border-radius: 20px;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .block.inactive {
    opacity: 0.5;
  }
  .overline {
    margin: 6px 0 0;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .overline:first-child {
    margin-top: 0;
  }
  .sub-label {
    margin: 0;
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }
  .button-row {
    display: flex;
    gap: 5px;
  }
  .pill {
    flex: 1;
    padding: 6px 4px;
    font: inherit;
    font-size: 10px;
    font-weight: 500;
    color: var(--msc2-text-secondary);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    cursor: pointer;
  }
  .pill:hover:not(:disabled) {
    color: var(--msc2-text-primary);
    background: rgba(255, 255, 255, 0.07);
  }
  .pill:disabled {
    cursor: default;
  }
  .field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .toggle-row {
    justify-content: flex-start;
  }
  .field-label {
    font-size: 11px;
    color: var(--msc2-text-secondary);
  }
  .notice {
    margin: 0;
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }
</style>
