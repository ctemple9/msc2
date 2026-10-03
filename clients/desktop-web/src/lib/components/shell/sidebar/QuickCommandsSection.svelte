<script lang="ts">
  // Sidebar difficulty and default gamemode follow the active world profile.
  // Time, weather, and other shortcuts remain in-session commands. Command strings read verbatim from
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
  import { onMount } from 'svelte';
  import Select from '../../base/Select.svelte';
  import Toggle from '../../base/Toggle.svelte';
  import { ApiError } from '../../../api/client';
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call, errorMessage, mutate } from '../../../sections/shared/types';
  import { worldPaths } from '../../../sections/worlds/model';

  export let api: ScreenApi | undefined = undefined;
  export let activeServerId: string | undefined = undefined;
  export let running = false;
  export let isBedrock = false;
  export let capabilities: Schema['CapabilitiesDTO'] | null = null;
  export let canControl = true;

  const TIME_PRESETS = [
    { value: 'dawn', label: 'Dawn' },
    { value: 'dusk', label: 'Dusk' },
    { value: 'night', label: 'Night' },
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
  let confirmation: SafetyPrompt | undefined;
  let loadedForServerId: string | undefined;
  let activeWorldSlotId: string | undefined;
  let settingBusy = false;
  let loadGeneration = 0;

  type SafetyPrompt = {
    token: string;
    title: string;
    message: string;
    command: string;
    restoreGamemode?: string;
    worldSetting?: {
      key: 'gameplay.difficulty' | 'gameplay.default-game-mode';
      value: string;
    };
  };

  function safetyPrompt(
    error: unknown,
  ): Omit<SafetyPrompt, 'command' | 'restoreGamemode'> | undefined {
    if (!(error instanceof ApiError) || error.error.code !== 'confirmation_required') return;
    const raw = (error.error.details as Record<string, unknown> | null | undefined)?.confirmation;
    if (!raw || typeof raw !== 'object') return;
    const prompt = raw as Record<string, unknown>;
    if (
      typeof prompt.acknowledgement !== 'string' ||
      typeof prompt.title !== 'string' ||
      typeof prompt.message !== 'string'
    ) {
      return;
    }
    return {
      token: prompt.acknowledgement,
      title: prompt.title,
      message: prompt.message,
    };
  }

  $: tps = performance?.tps1m?.value;
  $: onlineCount = players?.count ?? 0;
  $: disabled = !running || !canControl;
  $: worldSettingDisabled = disabled || !activeWorldSlotId || settingBusy;
  $: relativeTime = capabilities?.worldSettings?.relativeTime;
  $: relativeTimeAvailable = relativeTime?.available === true;

  function supportsRelativeTimePreset(preset: (typeof TIME_PRESETS)[number]['value']): boolean {
    return relativeTimeAvailable && relativeTime?.presets.includes(preset) === true;
  }

  $: if (activeServerId !== loadedForServerId) {
    loadedForServerId = activeServerId;
    void load();
  }

  onMount(() => {
    const refresh = () => void load();
    window.addEventListener('msc2:active-world-profile-changed', refresh);
    return () => window.removeEventListener('msc2:active-world-profile-changed', refresh);
  });

  async function load(): Promise<void> {
    const generation = ++loadGeneration;
    if (!api) return;
    performance = await call(api, performance, '/v1/performance');
    players = await call(api, players, '/v1/players');
    const settings = await call<Schema['SettingsResponseDTO'] | undefined>(
      api,
      undefined,
      '/v1/settings',
    );
    const fields = settings?.sections.flatMap((section) => section.fields) ?? [];
    whitelistEnabled = fields.find((field) => field.key === 'white-list')?.value === 'true';
    activeWorldSlotId = undefined;
    difficulty = 'normal';
    gamemode = 'survival';
    try {
      const worlds = await api.get<Schema['WorldSlotsResponseDTO']>(worldPaths.list);
      if (generation !== loadGeneration) return;
      activeWorldSlotId = worlds.activeSlotId;
      if (activeWorldSlotId) {
        const slot = await api.get<Schema['WorldSlotWithProfileDTO']>(worldPaths.profile(activeWorldSlotId));
        if (generation !== loadGeneration) return;
        difficulty = slot.profile.gameplay.difficulty ?? 'normal';
        gamemode = slot.profile.gameplay.defaultGameMode ?? 'survival';
      }
    } catch (error) {
      if (generation !== loadGeneration) return;
      activeWorldSlotId = undefined;
      notice = errorMessage(error);
    }
  }

  async function sendCommand(
    command: string,
    confirmationToken?: string,
    restoreGamemode?: string,
  ): Promise<void> {
    confirmation = undefined;
    try {
      await mutate(api, '/v1/command', {
        command,
        ...(confirmationToken ? { confirmation: confirmationToken } : {}),
      });
    } catch (error) {
      const prompt = safetyPrompt(error);
      if (prompt) {
        confirmation = { ...prompt, command, restoreGamemode };
      } else {
        notice = errorMessage(error);
      }
    }
  }

  async function sendRelativeTime(preset: (typeof TIME_PRESETS)[number]['value']): Promise<void> {
    if (!supportsRelativeTimePreset(preset)) return;
    notice = '';
    try {
      await mutate(api, '/v1/time/relative', { preset });
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function saveWorldSetting(
    key: 'gameplay.difficulty' | 'gameplay.default-game-mode',
    value: string,
    token?: string,
  ): Promise<void> {
    if (!api || !activeWorldSlotId || settingBusy) return;
    const previous = key === 'gameplay.difficulty' ? difficulty : gamemode;
    settingBusy = true;
    notice = '';
    confirmation = undefined;
    try {
      const result = await mutate<Schema['WorldProfileUpdateResultDTO']>(api, worldPaths.profile(activeWorldSlotId), {
        changes: { [key]: value },
        ...(token ? { confirmation: token } : {}),
      });
      if (key === 'gameplay.difficulty') difficulty = value;
      else gamemode = value;
      if (
        result.status === 'pending_restart' ||
        result.status === 'pending_activation' ||
        result.status === 'blocked'
      ) {
        notice = result.changes.find((change) => change.key === key)?.reason ?? 'Saved; it will apply after the server restarts.';
      }
      window.dispatchEvent(new Event('msc2:active-world-profile-changed'));
    } catch (error) {
      const prompt = safetyPrompt(error);
      if (prompt) {
        confirmation = {
          ...prompt,
          command: key === 'gameplay.difficulty' ? `difficulty ${value}` : `defaultgamemode ${value}`,
          restoreGamemode: key === 'gameplay.default-game-mode' ? previous : undefined,
          worldSetting: { key, value },
        };
      } else {
        if (key === 'gameplay.difficulty') difficulty = previous;
        else gamemode = previous;
        notice = errorMessage(error);
      }
    } finally {
      settingBusy = false;
    }
  }

  function applyDifficulty(value: string): void {
    void saveWorldSetting('gameplay.difficulty', value);
  }

  function applyGamemode(value: string): void {
    void saveWorldSetting('gameplay.default-game-mode', value);
  }

  function cancelConfirmation(): void {
    if (confirmation?.restoreGamemode) gamemode = confirmation.restoreGamemode;
    confirmation = undefined;
  }

  function confirmCommand(): void {
    if (!confirmation) return;
    const pending = confirmation;
    if (pending.worldSetting) {
      void saveWorldSetting(pending.worldSetting.key, pending.worldSetting.value, pending.token);
    } else {
      void sendCommand(pending.command, pending.token, pending.restoreGamemode);
    }
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
  {#if running}
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
          disabled={disabled || !supportsRelativeTimePreset(preset.value)}
          title={supportsRelativeTimePreset(preset.value)
            ? 'Keep the current Minecraft day'
            : (relativeTime?.reason ?? 'This runtime does not advertise same-day time shortcuts.')}
          onclick={() => void sendRelativeTime(preset.value)}
        >
          {preset.label}
        </button>
      {/each}
    </div>
    {#if !activeWorldSlotId}
      <p class="subtle-note" role="status">Select an active world slot to change its difficulty or default game mode.</p>
    {/if}
    {#if !relativeTimeAvailable}
      <p class="subtle-note" role="status">
        {relativeTime?.reason ??
          'Same-day shortcuts are unavailable until the selected runtime advertises time support.'}
      </p>
    {:else}
      <p class="subtle-note">Dawn, dusk, and night keep the current Minecraft day.</p>
    {/if}
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
        disabled={worldSettingDisabled}
        onchange={applyDifficulty}
      />
    </div>
    <div class="field-row">
      <span class="field-label">Gamemode</span>
      <Select options={GAMEMODE_OPTIONS} value={gamemode} disabled={worldSettingDisabled} onchange={applyGamemode} />
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

  {#if confirmation}
    <div class="confirmation" role="alert">
      <p class="confirmation-title">{confirmation.title}</p>
      <p>{confirmation.message}</p>
      <div class="confirmation-actions">
        <button type="button" class="pill" onclick={cancelConfirmation}>Cancel</button>
        <button type="button" class="pill" onclick={confirmCommand}>Continue</button>
      </div>
    </div>
  {/if}
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}
</div>

<style>
  .quick-commands {
    display: flex;
    flex-direction: column;
    gap: 8px;
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
  .subtle-note {
    margin: 0;
    font-size: 10px;
    line-height: 1.4;
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
  .confirmation {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 9px 10px;
    border: 1px solid var(--msc2-hairline-strong);
    border-radius: 7px;
    color: var(--msc2-text-secondary);
    font-size: 10px;
    line-height: 1.45;
  }
  .confirmation p {
    margin: 0;
  }
  .confirmation-title {
    color: var(--msc2-text-primary);
    font-weight: 600;
  }
  .confirmation-actions {
    display: flex;
    gap: 5px;
  }
</style>
