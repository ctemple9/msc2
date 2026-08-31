<script lang="ts">
  // MSC 1's PlayerAvatarView: Java/Bedrock edition toggle, one saved identity
  // per edition, and the rendered full-body skin. Java is a real minotar.net
  // fetch. Bedrock uses the same GeyserMC join-cache/Xbox lookup and dotted
  // gamertag fallback as MSC 1's BedrockSkinFetcher. No idle-sway: antiAIslop's design law
  // #8 reserves the app's one deliberate flourish for the terrain banner.
  import { onDestroy, onMount } from 'svelte';
  import SegmentedControl from '../base/SegmentedControl.svelte';
  import Field from '../base/Field.svelte';
  import Button from '../base/Button.svelte';
  import {
    getIdentity,
    getStoredEdition,
    setIdentity,
    setStoredEdition,
    type AvatarEdition,
  } from '../../player/avatarIdentity';
  import { fetchBedrockBodyUrl } from '../../player/bedrockSkin';

  type Status = 'prompt' | 'loading' | 'loaded' | 'error';

  const EDITIONS: { value: AvatarEdition; label: string }[] = [
    { value: 'java', label: 'Java' },
    { value: 'bedrock', label: 'Bedrock' },
  ];

  const META: Record<AvatarEdition, { placeholder: string; helper: string; changeLabel: string }> =
    {
      java: {
        placeholder: 'Java username',
        helper: 'Enter your Minecraft Java Edition username to show your skin here.',
        changeLabel: 'Change Username',
      },
      bedrock: {
        placeholder: 'Bedrock gamertag',
        helper: 'Enter your Minecraft Bedrock gamertag to show your skin here.',
        changeLabel: 'Change Gamertag',
      },
    };

  let edition: AvatarEdition = 'java';
  let javaUsername = '';
  let bedrockGamertag = '';
  let inputValue = '';
  let isEditing = false;
  let status: Status = 'prompt';
  let errorMessage = '';
  let imageUrl = '';
  let displayName = '';
  let showEdit = false;
  let avatarRequest = 0;

  $: currentIdentity = edition === 'java' ? javaUsername : bedrockGamertag;
  $: meta = META[edition];

  onMount(() => {
    edition = getStoredEdition();
    javaUsername = getIdentity('java');
    bedrockGamertag = getIdentity('bedrock');
    loadForEdition();
  });

  onDestroy(() => {
    avatarRequest += 1;
  });

  function loadForEdition(): void {
    avatarRequest += 1;
    showEdit = false;
    const identity = currentIdentity.trim();
    if (!identity) {
      status = 'prompt';
      isEditing = false;
      inputValue = '';
      return;
    }
    inputValue = identity;
    isEditing = false;
    if (edition === 'bedrock') {
      void fetchBedrockSkin(identity);
      return;
    }
    fetchJavaSkin(identity);
  }

  function fetchJavaSkin(username: string): void {
    const request = ++avatarRequest;
    status = 'loading';
    const encoded = encodeURIComponent(username);
    const url = `https://minotar.net/body/${encoded}/160`;
    const probe = new Image();
    probe.onload = () => {
      if (request !== avatarRequest || edition !== 'java' || currentIdentity.trim() !== username)
        return;
      imageUrl = url;
      displayName = username;
      status = 'loaded';
    };
    probe.onerror = () => {
      if (request !== avatarRequest || edition !== 'java' || currentIdentity.trim() !== username)
        return;
      errorMessage = `Username "${username}" wasn't found. Check the spelling — this must be a Java Edition username.`;
      status = 'error';
    };
    probe.src = url;
  }

  async function fetchBedrockSkin(gamertag: string): Promise<void> {
    const request = ++avatarRequest;
    status = 'loading';
    const url = await fetchBedrockBodyUrl(gamertag);
    if (request !== avatarRequest || edition !== 'bedrock' || currentIdentity.trim() !== gamertag)
      return;
    if (!url) {
      errorMessage = `Gamertag "${gamertag}" wasn't found. Check the spelling and use the player's Bedrock gamertag.`;
      status = 'error';
      return;
    }
    imageUrl = url;
    displayName = gamertag;
    status = 'loaded';
  }

  function selectEdition(next: string): void {
    showEdit = false;
    edition = next as AvatarEdition;
    setStoredEdition(edition);
    loadForEdition();
  }

  function startEdit(): void {
    avatarRequest += 1;
    showEdit = false;
    status = 'prompt';
    isEditing = true;
    inputValue = currentIdentity;
  }

  function commit(): void {
    const trimmed = inputValue.trim();
    if (!trimmed) return;
    if (edition === 'java') {
      javaUsername = trimmed;
    } else {
      bedrockGamertag = trimmed;
    }
    setIdentity(edition, trimmed);
    isEditing = false;
    loadForEdition();
  }
</script>

<div class="avatar">
  <div class="edition-switcher">
    <SegmentedControl options={EDITIONS} value={edition} onchange={selectEdition} />
  </div>

  {#if status === 'prompt'}
    <div class="prompt">
      {#if !isEditing}
        <p class="helper">{meta.helper}</p>
      {/if}
      <form
        class="prompt-row"
        onsubmit={(event) => {
          event.preventDefault();
          commit();
        }}
      >
        <Field bind:value={inputValue} placeholder={meta.placeholder} />
      </form>
    </div>
  {:else if status === 'loading'}
    <div class="loading">
      <span class="spinner" aria-hidden="true"></span>
      <span>Fetching skin…</span>
    </div>
  {:else if status === 'loaded'}
    <div class="rendered">
      <button
        type="button"
        class="avatar-trigger"
        aria-label="Edit avatar for {displayName}"
        onclick={() => (showEdit = true)}
      >
        <img class="skin" src={imageUrl} alt="{displayName}'s Minecraft skin" />
      </button>
      <p class="name">{displayName}</p>
      {#if showEdit}
        <button type="button" class="link edit-avatar" onclick={startEdit}>Edit</button>
      {/if}
    </div>
  {:else if status === 'error'}
    <div class="error-state">
      <p class="message">{errorMessage}</p>
      <div class="actions">
        <Button
          variant="primary"
          size="sm"
          onclick={() =>
            edition === 'bedrock'
              ? void fetchBedrockSkin(currentIdentity)
              : fetchJavaSkin(currentIdentity)}
        >
          Retry
        </Button>
        <button type="button" class="link" onclick={startEdit}>{meta.changeLabel}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .avatar {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .edition-switcher {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .link {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .link:hover {
    color: var(--msc2-text-secondary);
  }
  .prompt {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .helper {
    margin: 0;
    font-size: 11px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .prompt-row {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
  }
  .prompt-row :global(.field) {
    width: 100%;
    font-size: 12px;
    padding: 6px 8px;
  }
  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 20px 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .spinner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid var(--msc2-hairline);
    border-top-color: var(--msc2-text-secondary);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .rendered {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .avatar-trigger {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    background: transparent;
    border: 0;
    cursor: pointer;
  }
  .avatar-trigger:focus-visible {
    outline: 2px solid var(--msc2-hairline-field-focus);
    outline-offset: 3px;
    border-radius: 4px;
  }
  .edit-avatar {
    align-self: center;
  }
  .skin {
    width: auto;
    height: 160px;
    image-rendering: pixelated;
  }
  .name {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .error-state {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 0;
  }
  .error-state .message {
    margin: 0;
    font-size: 11px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
</style>
