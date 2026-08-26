<script lang="ts">
  // Ports PlayerProfileDetailSheet.swift: identity header, skin lookup
  // override, stats, inventory (both editions — Bedrock's are carried
  // through from LevelDB by bedrock_players.rs rather than discarded,
  // P12.3e; still read-only here for both, matching the Swift source),
  // and the 4 approved data-management actions (migrate offline/custom
  // UUID, duplicate, delete — copyPlayerData stays deferred per
  // rolling-plan.md's 2026-08-25 note).
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Field from '../../components/base/Field.svelte';
  import Button from '../../components/base/Button.svelte';
  import InventoryGrid from './InventoryGrid.svelte';
  import type { Schema } from '../shared/types';
  import { call, mutate } from '../shared/types';
  import { bodyUrl, playerPaths, profileDisplayName } from './model';

  export let profile: Schema['PlayerProfileDTO'];
  export let api: import('../shared/types').ScreenApi | undefined = undefined;
  export let onClose: () => void;
  export let onMutated: (profiles: Schema['PlayerProfileDTO'][]) => void;
  export let onDeleted: () => void;

  let skin: Schema['PlayerSkinResponseDTO'] | undefined;
  let lookupInput = profile.skinOverrideIdentifier ?? '';
  let showMigrateInput = false;
  let migrateUuidInput = '';
  let showIdentifyInput = false;
  let identifyInput = '';
  let confirmingDelete = false;
  let busy = false;
  let actionError: string | undefined;
  let actionSuccess: string | undefined;
  let portraitFailed = false;

  $: previewIdentifier = profile.skinOverrideIdentifier || profile.imageIdentifier;
  $: previewHeadUrl = `https://mc-heads.net/avatar/${encodeURIComponent(previewIdentifier)}/64`;
  $: previewBodyUrl = `https://mc-heads.net/body/${encodeURIComponent(previewIdentifier)}/64`;

  onMount(async () => {
    if (profile.isBedrockPlayer) return;
    skin = await call<Schema['PlayerSkinResponseDTO'] | undefined>(
      api,
      undefined,
      playerPaths.skin(profile.id),
    );
  });

  function flashSuccess(message: string): void {
    actionError = undefined;
    actionSuccess = message;
  }
  function flashError(message: string): void {
    actionSuccess = undefined;
    actionError = message;
  }

  async function run(label: string, path: string, body: unknown): Promise<void> {
    busy = true;
    try {
      const result = await mutate<Schema['PlayerMutationResultDTO']>(api, path, body);
      onMutated(result.profiles.profiles);
      flashSuccess(result.message || label);
    } catch (error) {
      flashError(error instanceof Error ? error.message : `Failed to ${label.toLowerCase()}.`);
    } finally {
      busy = false;
    }
  }

  async function migrateOffline(): Promise<void> {
    await run('Migrated to offline UUID', playerPaths.migrateOffline, { profileId: profile.id });
  }
  async function migrateCustom(): Promise<void> {
    const targetUuid = migrateUuidInput.trim();
    if (!targetUuid) return;
    await run('Migrated to custom UUID', playerPaths.migrate, {
      profileId: profile.id,
      targetUuid,
    });
    showMigrateInput = false;
    migrateUuidInput = '';
  }
  async function duplicate(): Promise<void> {
    await run('Duplicate created', playerPaths.duplicate, { profileId: profile.id });
  }
  async function performDelete(): Promise<void> {
    busy = true;
    try {
      const result = await mutate<Schema['PlayerMutationResultDTO']>(api, playerPaths.delete, {
        profileId: profile.id,
      });
      onMutated(result.profiles.profiles);
      onDeleted();
    } catch (error) {
      flashError(error instanceof Error ? error.message : 'Failed to delete player data.');
      confirmingDelete = false;
    } finally {
      busy = false;
    }
  }
  async function identifyPlayer(): Promise<void> {
    const gamertag = identifyInput.trim();
    if (!gamertag) return;
    busy = true;
    try {
      const result = await mutate<Schema['PlayerIdentifyResultDTO']>(api, playerPaths.identify, {
        profileId: profile.id,
        gamertag,
      });
      profile = { ...profile, username: result.username ?? gamertag };
      onMutated([profile]);
      showIdentifyInput = false;
      identifyInput = '';
      flashSuccess(`Identified as ${profile.username}.`);
    } catch (error) {
      flashError(error instanceof Error ? error.message : 'Failed to identify player.');
    } finally {
      busy = false;
    }
  }
  async function toggleHidden(): Promise<void> {
    busy = true;
    try {
      const result = await mutate<Schema['HiddenProfileMutationResultDTO']>(
        api,
        playerPaths.hidden,
        {
          profileId: profile.id,
          hidden: !profile.isHidden,
        },
      );
      profile = { ...profile, isHidden: result.isHidden ?? !profile.isHidden };
      onMutated([profile]);
    } finally {
      busy = false;
    }
  }
  async function saveLookupOverride(): Promise<void> {
    const trimmed = lookupInput.trim();
    const result = await mutate<Schema['PlayerSkinOverrideResultDTO']>(
      api,
      playerPaths.skinOverride,
      {
        profileId: profile.id,
        lookupIdentifier: trimmed || undefined,
      },
    );
    profile = { ...profile, skinOverrideIdentifier: result.lookupIdentifier };
    skin = await call<Schema['PlayerSkinResponseDTO'] | undefined>(
      api,
      skin,
      playerPaths.skin(profile.id),
    );
  }

  function formatLastSeen(value: string | undefined): string {
    if (!value) return 'unknown';
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
  }
</script>

<Sheet title="Player Profile" size="md" {onClose}>
  <div class="identity">
    {#if skin?.success && skin.imageBase64}
      <img
        class="portrait"
        src={`data:${skin.imageMimeType ?? 'image/png'};base64,${skin.imageBase64}`}
        alt=""
      />
    {:else if !portraitFailed}
      <img
        class="portrait"
        src={bodyUrl(profile, 96)}
        alt=""
        onerror={() => (portraitFailed = true)}
      />
    {:else}
      <span class="portrait initial">{profileDisplayName(profile).slice(0, 1).toUpperCase()}</span>
    {/if}

    <div class="identity-text">
      <span class="display-name">{profileDisplayName(profile)}</span>
      <span class="id">{profile.id}</span>
      <div class="badges">
        <span class="badge" class:on={profile.isOnline}
          >{profile.isOnline ? 'Online' : 'Offline'}</span
        >
        {#if profile.isOp}<span class="badge op">Operator</span>{/if}
        {#if profile.isBedrockPlayer}<span class="badge bedrock">Bedrock</span>{/if}
      </div>
      <span class="last-seen">Last seen {formatLastSeen(profile.lastSeen)}</span>
    </div>
  </div>

  <section class="block">
    <h3><Icon name="id-card" size={12} />Skin Override</h3>
    <div class="skin-preview">
      <div class="skin-preview-item">
        <img class="skin-thumb" src={previewHeadUrl} alt="" />
        <span class="skin-thumb-label">Head</span>
      </div>
      <div class="skin-preview-item">
        <img class="skin-thumb" src={previewBodyUrl} alt="" />
        <span class="skin-thumb-label">Body</span>
      </div>
    </div>
    <form class="lookup-row" onsubmit={(event) => (event.preventDefault(), saveLookupOverride())}>
      <Field bind:value={lookupInput} placeholder="Custom lookup name or UUID" />
      <Button size="sm" type="submit">Save</Button>
    </form>
    <p class="hint">Leave blank to use this profile's own identity for the skin lookup.</p>
  </section>

  {#if actionError}<p class="feedback error">{actionError}</p>{/if}
  {#if actionSuccess}<p class="feedback success">{actionSuccess}</p>{/if}

  <section class="block">
    <h3><Icon name="grid" size={12} />Stats</h3>
    {#if profile.stats}
      {@const stats = profile.stats}
      <div class="stats">
        <div class="stat-row">
          <span class="label">Health</span>
          <span class="value">{stats.health.toFixed(1)} / {stats.maxHealth.toFixed(0)}</span>
        </div>
        <div class="stat-row">
          <span class="label">Food</span>
          <span class="value">{stats.foodLevel} / 20</span>
        </div>
        <div class="stat-row">
          <span class="label">XP</span>
          <span class="value">Level {stats.xpLevel} · {stats.xpTotal} total</span>
        </div>
        <div class="stat-row">
          <span class="label">Mode</span>
          <span class="value">{stats.gameModeDisplay}</span>
        </div>
        <div class="stat-row">
          <span class="label">Position</span>
          <span class="value mono"
            >x {stats.posX.toFixed(0)} y {stats.posY.toFixed(0)} z {stats.posZ.toFixed(0)} · {stats.dimensionDisplay}</span
          >
        </div>
        {#if stats.score > 0}
          <div class="stat-row">
            <span class="label">Score</span>
            <span class="value">{stats.score}</span>
          </div>
        {/if}
      </div>
    {:else}
      <p class="hint">No stats available for this player yet.</p>
    {/if}
  </section>

  <section class="block">
    <h3><Icon name="box" size={12} />Inventory</h3>
    {#if profile.inventory.length === 0}
      <p class="hint">Inventory is empty.</p>
    {:else}
      <InventoryGrid inventory={profile.inventory} />
    {/if}
  </section>

  <section class="block">
    <h3><Icon name="id-card" size={12} />Data Management</h3>
    <div class="actions">
      {#if profile.isBedrockPlayer}
        {#if showIdentifyInput}
          <form
            class="inline-input"
            onsubmit={(event) => (event.preventDefault(), identifyPlayer())}
          >
            <Field bind:value={identifyInput} placeholder="Gamertag" />
            <Button size="sm" type="submit" disabled={busy}>Save</Button>
            <Button size="sm" onclick={() => (showIdentifyInput = false)}>Cancel</Button>
          </form>
        {:else}
          <Button
            disabled={busy}
            onclick={() => {
              identifyInput = profile.username ?? '';
              showIdentifyInput = true;
            }}
          >
            {profile.username ? 'Change Gamertag' : 'Identify Player'}
          </Button>
        {/if}
      {:else}
        <Button disabled={busy || !profile.username} onclick={migrateOffline}>
          Migrate to Offline UUID
        </Button>
        {#if showMigrateInput}
          <form
            class="inline-input"
            onsubmit={(event) => (event.preventDefault(), migrateCustom())}
          >
            <Field bind:value={migrateUuidInput} placeholder="Target UUID" />
            <Button size="sm" type="submit" disabled={busy}>Confirm</Button>
            <Button size="sm" onclick={() => (showMigrateInput = false)}>Cancel</Button>
          </form>
        {:else}
          <Button disabled={busy} onclick={() => (showMigrateInput = true)}
            >Migrate to Custom UUID</Button
          >
        {/if}
        <Button disabled={busy} onclick={duplicate}>Duplicate</Button>
        {#if confirmingDelete}
          <div class="confirm-row">
            <span class="confirm-message"
              >Permanently delete {profileDisplayName(profile)}'s data? This cannot be undone.</span
            >
            <Button size="sm" variant="destructive" disabled={busy} onclick={performDelete}
              >Delete</Button
            >
            <Button size="sm" onclick={() => (confirmingDelete = false)}>Cancel</Button>
          </div>
        {:else}
          <Button variant="destructive" disabled={busy} onclick={() => (confirmingDelete = true)}
            >Delete Player Data</Button
          >
        {/if}
      {/if}
      <Button disabled={busy} onclick={toggleHidden}>
        {profile.isHidden ? 'Unhide Profile' : 'Hide Profile'}
      </Button>
    </div>
  </section>

  {#if profile.isBedrockPlayer}
    <p class="bedrock-note">
      Bedrock player data is stored in a LevelDB database and cannot be edited directly from MSC.
      Stop the server and use a LevelDB editor to modify or remove player data.
    </p>
  {/if}
</Sheet>

<style>
  .identity {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 18px;
  }
  .portrait {
    width: 72px;
    height: 72px;
    border-radius: 10px;
    image-rendering: pixelated;
    flex: none;
  }
  .portrait.initial {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-secondary);
    font-size: 26px;
    font-weight: 600;
  }
  .identity-text {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .display-name {
    font-size: 17px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .id {
    font-family: var(--msc2-font-mono);
    font-size: 10px;
    color: var(--msc2-text-tertiary);
    word-break: break-all;
  }
  .badges {
    display: flex;
    gap: 6px;
  }
  .badge {
    font-size: 10px;
    font-weight: 500;
    padding: 2px 7px;
    border-radius: 5px;
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-tertiary);
  }
  .badge.on {
    background: var(--msc2-status-ok-tint);
    color: var(--msc2-status-ok);
  }
  .badge.op {
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-secondary);
  }
  .badge.bedrock {
    background: var(--msc2-status-bedrock-tint);
    color: var(--msc2-status-bedrock);
  }
  .last-seen {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }

  .block {
    margin-bottom: 18px;
  }
  .block h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-tertiary);
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .feedback {
    margin: 0 0 14px;
    font-size: 12px;
    padding: 8px 10px;
    border-radius: 6px;
  }
  .feedback.error {
    color: var(--msc2-status-warn);
    background: var(--msc2-status-warn-tint);
  }
  .feedback.success {
    color: var(--msc2-status-ok);
    background: var(--msc2-status-ok-tint);
  }

  .lookup-row,
  .inline-input {
    display: flex;
    gap: 8px;
  }

  .skin-preview {
    display: flex;
    gap: 16px;
    margin-bottom: 12px;
  }
  .skin-preview-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  .skin-thumb {
    width: 64px;
    height: 64px;
    object-fit: contain;
    image-rendering: pixelated;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 8px;
  }
  .skin-thumb-label {
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }

  .stats {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 8px;
    padding: 12px;
  }
  .stat-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .stat-row .label {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .stat-row .value {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-primary);
    text-align: right;
  }
  .stat-row .value.mono {
    font-family: var(--msc2-font-mono);
    font-weight: 400;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  .confirm-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .confirm-message {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }

  .bedrock-note {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
    background: var(--msc2-status-bedrock-tint);
    border-radius: 8px;
    padding: 10px 12px;
  }
</style>
