<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../components/ActionButton.svelte';
  import {
    ACCENT_PRESETS,
    applyAccent,
    saveAccent,
    storedAccent,
    type AccentChoice,
  } from '../styles/accent';

  export let compact = false;
  export let headingId = 'first-launch-title';
  export let onComplete: () => void = () => {};

  let selected = 'green';
  let customColor = '#22c85a';

  onMount(() => {
    selected = storedAccent();
    if (selected.startsWith('#')) customColor = selected;
    applyAccent(selected);
  });

  function chooseAccent(choice: AccentChoice): void {
    selected = choice.id;
    saveAccent(choice);
  }

  function chooseCustom(): void {
    selected = customColor;
    saveAccent(customColor);
  }
</script>

<div class="setup-intro" class:compact>
  {#if !compact}
    <header class="setup-heading">
      <div class="setup-heading-icon" aria-hidden="true">▤</div>
      <div>
        <p class="eyebrow">First-time Setup</p>
        <h2 id={headingId}>First-time Setup</h2>
        <p class="setup-subtitle">Let’s get Minecraft Server Controller configured.</p>
      </div>
    </header>
  {:else}
    <h3 id={headingId}>First-time Setup</h3>
    <p class="setup-subtitle">Let’s get Minecraft Server Controller configured.</p>
  {/if}

  <section class="setup-card">
    <div class="card-heading">
      <span class="card-icon blue" aria-hidden="true">▤</span>
      <div>
        <h3>What is Minecraft Server Controller?</h3>
        <p>MSC helps you run and manage Minecraft servers on your computer.</p>
      </div>
    </div>
    <ul>
      <li>
        <span class="feature-icon green" aria-hidden="true">▶</span>Start and stop Java and Bedrock
        servers with one click
      </li>
      <li>
        <span class="feature-icon blue" aria-hidden="true">●●●</span>Invite friends via tunnels,
        port forwarding, or Tailscale
      </li>
      <li>
        <span class="feature-icon purple" aria-hidden="true">◆</span>Install plugins, mods, and
        resource packs from Modrinth
      </li>
      <li>
        <span class="feature-icon orange" aria-hidden="true">▰</span>Schedule backups and manage
        multiple worlds
      </li>
    </ul>
  </section>

  <section class="setup-card">
    <div class="card-heading">
      <span class="card-icon green" aria-hidden="true">✿</span>
      <div>
        <h3>Pick an Accent Color</h3>
        <p>Tints the app shell and overlays. Change it anytime in Preferences.</p>
      </div>
    </div>
    <div class="accent-choices" aria-label="Accent colors">
      {#each ACCENT_PRESETS as choice (choice.id)}
        <button
          class="accent-choice"
          class:selected={selected === choice.id}
          type="button"
          aria-label={choice.label}
          aria-pressed={selected === choice.id}
          style={`--choice-color: ${choice.color}`}
          onclick={() => chooseAccent(choice)}
        >
          {#if selected === choice.id}<span aria-hidden="true">✓</span>{/if}
        </button>
      {/each}
      <label class="custom-accent" title="Pick a custom accent color">
        <input
          aria-label="Custom accent color"
          type="color"
          bind:value={customColor}
          oninput={chooseCustom}
        />
        <span aria-hidden="true">+</span>
      </label>
    </div>
  </section>

  <p class="setup-time">This setup takes about 2 minutes.</p>
  <ActionButton label="Next" onclick={onComplete}
    >Next <span aria-hidden="true">→</span></ActionButton
  >
</div>

<style>
  .setup-intro {
    overflow: hidden;
    margin: -1.5rem;
    border-radius: var(--msc-radius-lg);
    background: var(--msc-surface-raised);
  }
  .setup-heading {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.5rem;
    color: white;
    background: linear-gradient(135deg, var(--msc-accent), #19723a);
  }
  .setup-heading-icon,
  .card-icon {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    border-radius: 0.75rem;
    font-weight: 900;
  }
  .setup-heading-icon {
    width: 3rem;
    height: 3rem;
    background: rgba(255, 255, 255, 0.16);
    font-size: 1.5rem;
  }
  .setup-heading h2,
  .setup-heading p,
  .compact h3,
  .compact > p {
    margin: 0;
  }
  .setup-heading h2 {
    color: white;
  }
  .setup-heading .eyebrow {
    color: rgba(255, 255, 255, 0.78);
  }
  .setup-subtitle {
    color: var(--msc-muted);
  }
  .setup-heading .setup-subtitle {
    color: rgba(255, 255, 255, 0.82);
  }
  .setup-card {
    margin: 1.25rem 1.5rem 0;
    padding: 1rem;
    border-radius: var(--msc-radius-md);
    background: var(--msc-surface);
  }
  .card-heading {
    display: flex;
    gap: 0.7rem;
    align-items: flex-start;
  }
  .card-heading h3 {
    margin: 0;
    font-size: 1rem;
  }
  .card-heading p {
    margin: 0.2rem 0 0;
    color: var(--msc-muted);
    font-size: 0.85rem;
  }
  .card-icon {
    width: 2rem;
    height: 2rem;
    font-size: 1rem;
  }
  .blue {
    color: #60a5fa;
    background: rgba(59, 130, 246, 0.18);
  }
  .green {
    color: #4ade80;
    background: rgba(34, 197, 94, 0.18);
  }
  .purple {
    color: #c084fc;
    background: rgba(168, 85, 247, 0.18);
  }
  .orange {
    color: #fb923c;
    background: rgba(249, 115, 22, 0.18);
  }
  ul {
    display: grid;
    gap: 0.55rem;
    margin: 0.9rem 0 0;
    padding: 0;
    list-style: none;
    color: var(--msc-muted);
    font-size: 0.9rem;
  }
  li {
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
    line-height: 1.35;
  }
  .feature-icon {
    min-width: 1.1rem;
    font-size: 0.75rem;
    text-align: center;
  }
  .accent-choices {
    display: flex;
    flex-wrap: wrap;
    gap: 0.55rem;
    margin-top: 0.9rem;
  }
  .accent-choice,
  .custom-accent {
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    border: 2px solid transparent;
    border-radius: 50%;
    color: white;
    background: var(--choice-color);
    cursor: pointer;
  }
  .accent-choice.selected {
    border-color: white;
    box-shadow: 0 0 0 2px var(--choice-color);
  }
  .accent-choice:focus-visible,
  .custom-accent:focus-within {
    outline: none;
    box-shadow: var(--msc-focus);
  }
  .custom-accent {
    position: relative;
    color: white;
    background: conic-gradient(#22c85a, #3b82f6, #8b5cf6, #ef4444, #22c85a);
  }
  .custom-accent input {
    position: absolute;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }
  .setup-time {
    margin: 1rem 1.5rem;
    color: var(--msc-subtle);
    font-size: 0.85rem;
    text-align: center;
  }
  .setup-intro > :global(.action-button) {
    display: block;
    margin: 0 1.5rem 1.5rem auto;
  }
  .compact {
    margin: 0;
    overflow: visible;
    background: transparent;
  }
  .compact .setup-card {
    margin-inline: 0;
  }
  .compact .setup-time {
    margin-inline: 0;
  }
  .compact > :global(.action-button) {
    margin-right: 0;
    margin-bottom: 0;
  }
  @media (max-width: 520px) {
    .setup-heading {
      align-items: flex-start;
      padding: 1.1rem;
    }
    .setup-card {
      margin-inline: 1rem;
    }
    .setup-time {
      margin-inline: 1rem;
    }
    .setup-intro > :global(.action-button) {
      margin-inline: 1rem 1rem;
    }
  }
</style>
