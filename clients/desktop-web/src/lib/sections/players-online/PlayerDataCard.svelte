<script lang="ts">
  // Ports PlayerProfilesCard.swift: search, sort, a grid of profile chips,
  // hidden-profile show/hide toggle, click-through to the detail sheet.
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Field from '../../components/base/Field.svelte';
  import Select from '../../components/base/Select.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import type { Schema } from '../shared/types';
  import {
    avatarUrl,
    profileDisplayName,
    profileSearch,
    profileSort,
    type ProfileSortOrder,
  } from './model';

  export let profiles: readonly Schema['PlayerProfileDTO'][] = [];
  export let activeWorldName: string | undefined = undefined;
  export let loading = false;
  export let onSelect: ((profile: Schema['PlayerProfileDTO']) => void) | undefined = undefined;
  export let onReload: (() => void) | undefined = undefined;

  let searchText = '';
  let sortOrder: ProfileSortOrder = 'lastSeen';
  let showHidden = false;

  const sortOptions = [
    { value: 'lastSeen', label: 'Last Seen' },
    { value: 'nameAZ', label: 'Name A–Z' },
  ];

  $: hiddenCount = profiles.filter((profile) => profile.isHidden).length;
  $: visible = showHidden ? profiles : profiles.filter((profile) => !profile.isHidden);
  $: filtered = profileSort(profileSearch(visible, searchText), sortOrder);
</script>

<Card>
  <div class="header">
    <div class="title-block">
      <div class="overline">
        <Icon name="id-card" size={13} />
        <span class="msc2-type-overline">Player Data</span>
      </div>
      {#if activeWorldName}<span class="world">Active world: {activeWorldName}</span>{/if}
    </div>
    <div class="header-actions">
      {#if loading}
        <span class="count">Scanning…</span>
      {:else}
        <span class="count">{profiles.length - hiddenCount} profiles</span>
      {/if}
      {#if hiddenCount > 0}
        <button type="button" class="hidden-toggle" onclick={() => (showHidden = !showHidden)}>
          {showHidden ? 'Hide hidden' : `Show ${hiddenCount} hidden`}
        </button>
      {/if}
      <Select options={sortOptions} bind:value={sortOrder} width="auto" />
      {#if onReload}
        <button
          type="button"
          class="reload"
          aria-label="Reload player profiles from disk"
          onclick={onReload}
        >
          <Icon name="download" size={14} />
        </button>
      {/if}
    </div>
  </div>

  <Field bind:value={searchText} placeholder="Filter by name or UUID" />

  <div class="body">
    {#if loading && profiles.length === 0}
      <p class="loading">Scanning player data…</p>
    {:else if profiles.length === 0}
      <EmptyState
        title="No player data found"
        message="Player profiles appear here after someone has joined the server at least once."
      >
        <Icon name="id-card" size={26} slot="icon" />
      </EmptyState>
    {:else if filtered.length === 0}
      <p class="loading">No profiles match &quot;{searchText}&quot;.</p>
    {:else}
      <div class="grid">
        {#each filtered as profile (profile.id)}
          <button
            type="button"
            class="chip"
            class:hidden={profile.isHidden}
            onclick={() => onSelect?.(profile)}
          >
            {#if avatarUrl(profile)}
              <img class="avatar" src={avatarUrl(profile, 64)} alt="" loading="lazy" />
            {:else}
              <span class="avatar initial"
                >{profileDisplayName(profile).slice(0, 1).toUpperCase()}</span
              >
            {/if}
            <span class="name">{profileDisplayName(profile)}</span>
            {#if profile.isOnline}<span class="online-dot" aria-hidden="true"></span>{/if}
          </button>
        {/each}
      </div>
      <p class="hint">Tap a card to view stats, inventory, and data management options.</p>
    {/if}
  </div>
</Card>

<style>
  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 12px;
  }
  .title-block {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .world {
    font-size: 10px;
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
  .hidden-toggle {
    font-size: 10px;
    color: var(--msc2-text-tertiary);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .reload {
    display: inline-flex;
    color: var(--msc2-text-tertiary);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .body {
    margin-top: 10px;
  }
  .loading {
    margin: 0;
    padding: 12px 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
    text-align: center;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
    gap: 8px;
  }
  .chip {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 10px 6px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 8px;
    cursor: pointer;
  }
  .chip:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .chip.hidden {
    opacity: 0.4;
  }
  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    image-rendering: pixelated;
  }
  .avatar.initial {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-secondary);
    font-size: 13px;
    font-weight: 600;
  }
  .name {
    font-size: 11px;
    color: var(--msc2-text-primary);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .online-dot {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--msc2-status-ok);
  }
  .hint {
    margin: 10px 0 0;
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }
</style>
