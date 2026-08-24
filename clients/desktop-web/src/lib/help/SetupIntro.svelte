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

  const javaFamilies = [
    {
      name: 'Paper',
      description: 'Plugin-based. Fast, stable, largest ecosystem.',
      icon: '▣',
      color: '#67e8f9',
    },
    {
      name: 'Purpur',
      description: 'Paper + extra gameplay tweaks. All Paper plugins work.',
      icon: '✣',
      color: '#c084fc',
    },
    {
      name: 'Vanilla',
      description: 'No plugins. Fully authentic Mojang experience.',
      icon: '◆',
      color: '#9ca3af',
    },
    {
      name: 'Fabric',
      description: 'Lightweight mods. Fast updates, great for optimization.',
      icon: '✂',
      color: '#60a5fa',
    },
    {
      name: 'Forge',
      description: 'Classic modding platform. Widest mod selection.',
      icon: '⚒',
      color: '#fb923c',
    },
    {
      name: 'NeoForge',
      description: 'Forge’s modern successor. More active development.',
      icon: '⚒',
      color: '#2dd4bf',
    },
  ] as const;
  const bedrockFamilies = [
    {
      name: 'BDS',
      description: 'Official Mojang Bedrock server. Runs in a built-in VM, no Docker needed.',
      icon: '▥',
      color: '#4ade80',
    },
  ] as const;

  let selected = 'green';
  let customColor = '#22c85a';
  let setupPage = 0;
  let wantsJava = true;
  let wantsBedrock = false;

  onMount(() => {
    selected = storedAccent();
    if (selected.startsWith('#')) customColor = selected;
    applyAccent(selected);
    if (typeof localStorage !== 'undefined') {
      try {
        const stored = JSON.parse(localStorage.getItem('msc.server-types') ?? '{}') as {
          java?: boolean;
          bedrock?: boolean;
        };
        if (typeof stored.java === 'boolean') wantsJava = stored.java;
        if (typeof stored.bedrock === 'boolean') wantsBedrock = stored.bedrock;
      } catch {
        // An invalid preference should not prevent first launch from opening.
      }
    }
  });

  function chooseAccent(choice: AccentChoice): void {
    selected = choice.id;
    saveAccent(choice);
  }

  function chooseCustom(): void {
    selected = customColor;
    saveAccent(customColor);
  }

  function toggleServerType(type: 'java' | 'bedrock'): void {
    if (type === 'java') wantsJava = !wantsJava;
    else wantsBedrock = !wantsBedrock;
  }

  function nextSetupPage(): void {
    if (setupPage === 0) {
      setupPage = 1;
      return;
    }
    if (!wantsJava && !wantsBedrock) return;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(
        'msc.server-types',
        JSON.stringify({ java: wantsJava, bedrock: wantsBedrock }),
      );
    }
    onComplete();
  }
</script>

<div class="setup-intro" class:compact>
  {#if !compact}
    <header class="setup-heading">
      <div class="setup-track" aria-label={`Setup step ${setupPage + 1} of 7`}>
        {#each Array(7) as _, index}
          {#if index > 0}<span class:complete={index <= setupPage} class="track-line"></span>{/if}
          <span
            class:active={index === setupPage}
            class:complete={index < setupPage}
            class="track-dot"
          ></span>
        {/each}
      </div>
      <div class="setup-heading-body">
        <div class="setup-heading-icon" aria-hidden="true">{setupPage === 0 ? '▤' : '▣'}</div>
        <div>
          <p class="eyebrow">First-time Setup</p>
          <h2 id={headingId}>{setupPage === 0 ? 'First-time Setup' : 'Server Type'}</h2>
          <p class="setup-subtitle">
            {setupPage === 0
              ? 'Let’s get Minecraft Server Controller configured.'
              : 'Choose which platform you’ll host servers on.'}
          </p>
        </div>
      </div>
    </header>
  {:else}
    <h3 id={headingId}>{setupPage === 0 ? 'First-time Setup' : 'Server Type'}</h3>
    <p class="setup-subtitle">
      {setupPage === 0
        ? 'Let’s get Minecraft Server Controller configured.'
        : 'Choose which platform you’ll host servers on.'}
    </p>
  {/if}

  {#key setupPage}
    <div class="setup-page">
      {#if setupPage === 0}
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
              <span class="feature-icon green" aria-hidden="true">▶</span>Start and stop Java and
              Bedrock servers with one click
            </li>
            <li>
              <span class="feature-icon blue" aria-hidden="true">●●●</span>Invite friends via
              tunnels, port forwarding, or Tailscale
            </li>
            <li>
              <span class="feature-icon purple" aria-hidden="true">◆</span>Install plugins, mods,
              and resource packs from Modrinth
            </li>
            <li>
              <span class="feature-icon orange" aria-hidden="true">▰</span>Schedule backups and
              manage multiple worlds
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
      {:else}
        <section class="server-type-page">
          <div class="type-grid">
            <button
              class="type-card java"
              class:on={wantsJava}
              type="button"
              aria-pressed={wantsJava}
              onclick={() => toggleServerType('java')}
            >
              <span class="type-icon" aria-hidden="true">☕</span>
              <span><strong>Java Servers</strong><small>Plugins, mods &amp; crossplay</small></span>
              <span class="type-check" aria-hidden="true">{wantsJava ? '✓' : '○'}</span>
            </button>
            <button
              class="type-card bedrock"
              class:on={wantsBedrock}
              type="button"
              aria-pressed={wantsBedrock}
              onclick={() => toggleServerType('bedrock')}
            >
              <span class="type-icon" aria-hidden="true">◆</span>
              <span
                ><strong>Bedrock Servers</strong><small>Mobile, console &amp; Windows</small></span
              >
              <span class="type-check" aria-hidden="true">{wantsBedrock ? '✓' : '○'}</span>
            </button>
          </div>

          {#if !wantsJava && !wantsBedrock}
            <p class="selection-warning" role="status">
              Select at least one type to continue. You can change this later.
            </p>
          {/if}

          {#if wantsJava}
            {#if wantsBedrock}<p class="family-label">JAVA</p>{/if}
            <div class="family-list">
              {#each javaFamilies as family, index (family.name)}
                <div class="family-row" style={`--row-index: ${index}`}>
                  <span class="family-icon" style={`color: ${family.color}`} aria-hidden="true"
                    >{family.icon}</span
                  >
                  <strong>{family.name}</strong><span class="family-separator">·</span><span
                    >{family.description}</span
                  >
                </div>
              {/each}
            </div>
            <p class="crossplay-note">
              <span aria-hidden="true">●●●</span> Java Edition players always. Bedrock, mobile, and console
              can join standard servers via Geyser crossplay (set up per server).
            </p>
          {/if}

          {#if wantsBedrock}
            {#if wantsJava}<p class="family-label">BEDROCK</p>{/if}
            {#each bedrockFamilies as family (family.name)}
              <div class="family-row bedrock-row" style={`--row-index: 0`}>
                <span class="family-icon" style={`color: ${family.color}`} aria-hidden="true"
                  >{family.icon}</span
                >
                <strong>{family.name}</strong><span class="family-separator">·</span><span
                  >{family.description}</span
                >
              </div>
            {/each}
            <p class="crossplay-note bedrock-note">
              <span aria-hidden="true">●●●</span> Mobile (iOS/Android), console (Xbox, PlayStation, Switch),
              and Windows Bedrock Edition. Java Edition players cannot join.
            </p>
          {/if}
        </section>
      {/if}
    </div>
  {/key}

  {#if setupPage === 0}<p class="setup-time">This setup takes about 2 minutes.</p>{/if}
  <div class="setup-actions">
    {#if setupPage === 1}<ActionButton kind="quiet" label="Back" onclick={() => (setupPage = 0)}
        >‹ Back</ActionButton
      >{/if}
    <ActionButton
      label="Next"
      disabled={setupPage === 1 && !wantsJava && !wantsBedrock}
      onclick={nextSetupPage}>Next <span aria-hidden="true">→</span></ActionButton
    >
  </div>
</div>

<style>
  .setup-intro {
    overflow: hidden;
    margin: -1.5rem;
    border-radius: var(--msc-radius-lg);
    background: var(--msc-surface-raised);
  }
  .setup-heading {
    display: grid;
    gap: 1rem;
    padding: 1.5rem;
    color: white;
    background: linear-gradient(135deg, var(--msc-accent), #19723a);
  }
  .setup-heading-body {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .setup-track {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .track-dot {
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.25);
    transition: all 180ms ease;
  }
  .track-dot.active {
    width: 0.6rem;
    height: 0.6rem;
    background: var(--msc-accent-strong);
  }
  .track-dot.complete {
    background: rgba(255, 255, 255, 0.65);
  }
  .track-line {
    flex: 1;
    min-width: 0.9rem;
    height: 1px;
    background: rgba(255, 255, 255, 0.18);
  }
  .track-line.complete {
    background: var(--msc-accent-strong);
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
  .setup-page {
    animation: setup-page-in 260ms ease both;
  }
  @keyframes setup-page-in {
    from {
      opacity: 0;
      transform: translateX(1rem);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }
  .server-type-page {
    padding: 1.25rem 1.5rem 0;
  }
  .type-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }
  .type-card {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.8rem;
    border: 1.5px solid transparent;
    border-radius: var(--msc-radius-md);
    color: var(--msc-text);
    background: var(--msc-surface);
    text-align: left;
    cursor: pointer;
  }
  .type-card.java.on {
    border-color: rgba(249, 115, 22, 0.7);
    background: rgba(249, 115, 22, 0.12);
  }
  .type-card.bedrock.on {
    border-color: rgba(34, 197, 94, 0.7);
    background: rgba(34, 197, 94, 0.12);
  }
  .type-card:focus-visible,
  .accent-choice:focus-visible {
    outline: none;
    box-shadow: var(--msc-focus);
  }
  .type-icon {
    display: grid;
    place-items: center;
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 0.7rem;
    color: white;
    background: #f97316;
    font-size: 1.1rem;
  }
  .bedrock .type-icon {
    background: #22c55e;
  }
  .type-card strong,
  .type-card small {
    display: block;
  }
  .type-card small {
    margin-top: 0.15rem;
    color: var(--msc-muted);
    font-size: 0.78rem;
  }
  .type-check {
    margin-left: auto;
    color: var(--msc-muted);
    font-size: 1.1rem;
  }
  .type-card.on .type-check {
    color: var(--msc-accent);
  }
  .selection-warning,
  .crossplay-note {
    margin: 0.75rem 0 0;
    padding: 0.7rem;
    border-radius: var(--msc-radius-sm);
    color: var(--msc-muted);
    background: rgba(59, 130, 246, 0.12);
    font-size: 0.8rem;
    line-height: 1.4;
  }
  .family-label {
    margin: 1rem 0 0.45rem;
    color: var(--msc-subtle);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.1em;
  }
  .family-list {
    display: grid;
    gap: 0.25rem;
  }
  .family-row {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    min-height: 2.55rem;
    padding: 0.55rem 0.7rem;
    border-radius: var(--msc-radius-sm);
    color: var(--msc-muted);
    background: rgba(232, 238, 242, 0.07);
    font-size: 0.82rem;
    animation: family-row-in 220ms ease both;
    animation-delay: calc(var(--row-index) * 55ms);
  }
  .family-row strong {
    color: var(--msc-text);
  }
  .family-icon {
    width: 1.1rem;
    font-size: 1rem;
    text-align: center;
  }
  .family-separator {
    color: var(--msc-subtle);
  }
  .crossplay-note {
    background: rgba(59, 130, 246, 0.14);
  }
  .bedrock-row {
    background: rgba(34, 197, 94, 0.1);
  }
  .bedrock-note {
    background: rgba(34, 197, 94, 0.12);
  }
  @keyframes family-row-in {
    from {
      opacity: 0;
      transform: translateY(0.35rem);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .setup-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin: 1rem 1.5rem 1.5rem;
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
  .compact .server-type-page {
    padding-inline: 0;
  }
  .compact .setup-actions {
    margin-inline: 0;
    margin-bottom: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .setup-page,
    .family-row {
      animation: none;
    }
    .track-dot,
    .track-line {
      transition: none;
    }
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
    .setup-actions {
      margin-inline: 1rem;
    }
  }
</style>
