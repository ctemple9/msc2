<script lang="ts">
  // Real port of AddServerWizardView.swift's step 2 Fresh/Configure --
  // Server Type, Server Name, then the Java branch (Server Software,
  // Source, Crossplay, Xbox Broadcast) or the Bedrock branch (Bedrock
  // Version, Max Players, Xbox Broadcast -- always available, not gated
  // behind crossplay). MSC 1's Docker Image field is commented-out dead
  // code in the oracle itself (VM backend ignores it), so it's left out
  // here too -- not a silent drop.
  //
  // Selection cards (Server Type, Category, Flavor) port the oracle's
  // WizardServerTypeCard/WizardFlavorCard *information* -- title, subtitle,
  // RECOMMENDED/SOON badges, disabled state -- with the flat neutral
  // selected-state treatment P12.18a's path-card already established, not
  // the oracle's accent-tinted icon + accent-colored checkmark. Per this
  // block's own note in rolling-plan.md (antiAIslop rule #6/#11): no icon,
  // no accent color spent on a choice that isn't running-state, active tab,
  // primary action, a live stat, or a defined status.
  //
  // Crossplay/Xbox Broadcast reuse the Toggle-in-a-Card row shape already
  // established by server-editor/BroadcastTab.svelte, not a new
  // accent-tinted card of their own.
  //
  // Not ported: the oracle's live Geyser/Floodgate and Xbox-broadcast-helper
  // download status shown while this sheet is still open. That's MSC 1
  // pre-fetching local jars before the server exists; here the agent
  // installs whatever `enableCrossPlay`/`enableXboxBroadcast` need as part
  // of the real create operation (P12.18g), so there is nothing to poll yet
  // -- a deliberate omission, not a silent drop.
  import Field from '../../../components/base/Field.svelte';
  import NumberField from '../../../components/base/NumberField.svelte';
  import SegmentedControl from '../../../components/base/SegmentedControl.svelte';
  import Toggle from '../../../components/base/Toggle.svelte';
  import Badge from '../../../components/base/Badge.svelte';
  import { tourServerContext } from '../../../help/onboarding';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import type { Schema, ScreenApi } from '../../shared/types';
  import { errorMessage } from '../../shared/types';
  import {
    BEDROCK_VERSION_NOTE,
    JAVA_CATEGORY_INFO,
    crossPlayUnavailable,
    defaultFlavorForCategory,
    isJavaFlavorImplemented,
    javaFlavorChoices,
    versionEntryLabel,
    versionsForCreatePath,
    type JavaCategory,
    type JavaFlavor,
    type WizardDraft,
  } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;

  const categories: readonly JavaCategory[] = ['standard', 'modded'];

  let sourceMode: 'latest' | 'choose' = draft.versionId ? 'choose' : 'latest';
  let availableVersions: Schema['VersionEntryDTO'][] = [];
  let isLoadingVersions = false;
  let versionsError: string | undefined;

  function publishTourContext(): void {
    tourServerContext.set({
      serverType: draft.serverType,
      javaCategory: draft.javaCategory,
      javaFlavor: draft.javaFlavor,
      enableCrossPlay: draft.enableCrossPlay,
    });
  }

  publishTourContext();

  $: unavailableForCrossPlay = crossPlayUnavailable(draft.javaCategory, draft.javaFlavor);
  // Xbox Broadcast is shared by both branches (`xboxBroadcastSection` in the
  // oracle) but gated differently: Java gates it behind crossplay being both
  // enabled and available, while Bedrock always shows it -- there's no
  // Java-side plugin to enable first.
  $: showXboxBroadcast =
    draft.serverType === 'bedrock' || (draft.enableCrossPlay && !unavailableForCrossPlay);

  function resetVersionSelection(): void {
    draft.versionId = undefined;
    availableVersions = [];
    versionsError = undefined;
    sourceMode = 'latest';
  }

  function selectServerType(type: WizardDraft['serverType']): void {
    draft.serverType = type;
    publishTourContext();
  }

  function selectCategory(category: JavaCategory): void {
    draft.javaCategory = category;
    draft.javaFlavor = defaultFlavorForCategory(category);
    if (category === 'modded') {
      draft.enableCrossPlay = false;
      draft.enableXboxBroadcast = false;
    }
    resetVersionSelection();
    publishTourContext();
  }

  function selectFlavor(flavor: JavaFlavor): void {
    draft.javaFlavor = flavor;
    if (flavor === 'vanilla') {
      draft.enableCrossPlay = false;
      draft.enableXboxBroadcast = false;
    }
    resetVersionSelection();
    publishTourContext();
  }

  async function fetchVersions(): Promise<void> {
    if (!api || isLoadingVersions) return;
    isLoadingVersions = true;
    versionsError = undefined;
    try {
      const response = await api.get<Schema['VersionsResponseDTO']>(
        versionsForCreatePath('java', draft.javaFlavor),
      );
      availableVersions = response.versions ?? [];
      if (!draft.versionId) {
        draft.versionId =
          availableVersions.find((entry) => entry.isLatest)?.id ?? availableVersions[0]?.id;
      }
    } catch (error) {
      versionsError = errorMessage(error);
      availableVersions = [];
    } finally {
      isLoadingVersions = false;
    }
  }

  function selectSourceMode(mode: string): void {
    if (mode === 'latest') {
      sourceMode = 'latest';
      draft.versionId = undefined;
      return;
    }
    sourceMode = 'choose';
    if (availableVersions.length === 0 && !isLoadingVersions) void fetchVersions();
  }

  function toggleCrossPlay(enabled: boolean): void {
    draft.enableCrossPlay = enabled;
    if (!enabled) draft.enableXboxBroadcast = false;
    publishTourContext();
  }
</script>

<div class="configure">
  <section class="block" use:onboardingAnchor={'ob_server_type'}>
    <p class="msc2-type-overline">Server Type</p>
    <div class="cards two-up">
      <button
        type="button"
        class="card"
        class:selected={draft.serverType === 'java'}
        onclick={() => selectServerType('java')}
      >
        <span class="card-title">Java</span>
        <span class="card-subtitle">PC · Cross-play optional</span>
      </button>
      <button
        type="button"
        class="card"
        class:selected={draft.serverType === 'bedrock'}
        onclick={() => selectServerType('bedrock')}
      >
        <span class="card-title">Bedrock</span>
        <span class="card-subtitle">PC · Console · Mobile</span>
      </button>
    </div>
  </section>

  <section class="block">
    <p class="msc2-type-overline">Server Name</p>
    <Field
      bind:value={draft.serverName}
      placeholder="Enter server name"
      anchorId="ob_server_name"
    />
  </section>

  {#if draft.serverType === 'java'}
    <section class="block" use:onboardingAnchor={'ob_server_category'}>
      <p class="msc2-type-overline">Server Software</p>
      <div class="cards two-up">
        {#each categories as category (category)}
          <button
            type="button"
            class="card"
            class:selected={draft.javaCategory === category}
            onclick={() => selectCategory(category)}
          >
            <span class="card-title">{JAVA_CATEGORY_INFO[category].displayName}</span>
            <span class="card-subtitle">{JAVA_CATEGORY_INFO[category].subtitle}</span>
          </button>
        {/each}
      </div>
    </section>

    <section class="block" use:onboardingAnchor={'ob_server_flavor'}>
      <div class="flavor-list">
        {#each javaFlavorChoices(draft.javaCategory) as flavor (flavor.id)}
          {@const available = isJavaFlavorImplemented(flavor.id)}
          <button
            type="button"
            class="flavor-row"
            class:selected={draft.javaFlavor === flavor.id}
            disabled={!available}
            onclick={() => selectFlavor(flavor.id)}
          >
            <span class="flavor-text">
              <span class="flavor-name-row">
                <span class="flavor-name">{flavor.displayName}</span>
                {#if flavor.isRecommended}<Badge>Recommended</Badge>{/if}
                {#if !available}<Badge>Soon</Badge>{/if}
              </span>
              <span class="flavor-description">{flavor.shortDescription}</span>
            </span>
          </button>
        {/each}
      </div>
    </section>

    <section class="block" use:onboardingAnchor={'ob_server_source'}>
      <p class="msc2-type-overline">Source</p>
      <SegmentedControl
        options={[
          { value: 'latest', label: 'Download latest' },
          { value: 'choose', label: 'Choose version…' },
        ]}
        value={sourceMode}
        onchange={selectSourceMode}
      />
      {#if sourceMode === 'choose'}
        {#if isLoadingVersions}
          <p class="hint">Loading versions…</p>
        {:else if versionsError}
          <p class="hint warn">{versionsError}</p>
        {:else if availableVersions.length === 0}
          <p class="hint">No versions were reported for {draft.javaFlavor}.</p>
        {:else}
          <!-- NeoForge/Forge list every stable loader build, not just the
               newest per Minecraft version -- often 50+ rows. A bounded,
               scrollable list (mirroring components/VersionPickerSheet.svelte's
               own .list/.row shape) keeps that from taking over the screen the
               way a native <select>'s own OS popup does. -->
          <div class="version-list" role="listbox" aria-label="Available versions">
            {#each availableVersions as entry (entry.id)}
              <button
                type="button"
                class="version-row"
                class:selected={draft.versionId === entry.id}
                onclick={() => (draft.versionId = entry.id)}
              >
                <span class="version-label">{versionEntryLabel(entry)}</span>
                {#if entry.isLatest}<Badge>Latest</Badge>{/if}
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    </section>

    <section class="block" use:onboardingAnchor={'ob_server_crossplay'}>
      <p class="msc2-type-overline">Crossplay</p>
      <div class="toggle-card">
        <div class="toggle-row">
          <Toggle
            checked={draft.enableCrossPlay}
            label="Enable Bedrock Cross-play"
            disabled={unavailableForCrossPlay}
            onchange={toggleCrossPlay}
          />
          <span class="toggle-text">
            <span class="toggle-name">Enable Bedrock Cross-play</span>
            <span class="toggle-hint"
              >Geyser and Floodgate are plugins that let Bedrock players (console, mobile, Windows)
              join your Java server. Enable here rather than adding them through the plugin browser.</span
            >
          </span>
        </div>
      </div>
      {#if draft.javaCategory === 'modded'}
        <p class="hint">
          Bedrock players can't join modded Java servers — cross-play is unavailable.
        </p>
      {:else if draft.javaFlavor === 'vanilla'}
        <p class="hint">
          Vanilla servers have no plugin API, so Geyser can't run — cross-play is unavailable.
        </p>
      {/if}
    </section>
  {:else}
    <section class="block">
      <p class="msc2-type-overline">Bedrock Version</p>
      <Field bind:value={draft.bedrockVersion} placeholder="LATEST" width="200px" />
      <p class="hint">{BEDROCK_VERSION_NOTE}</p>
    </section>

    <section class="block">
      <p class="msc2-type-overline">Max Players</p>
      <NumberField
        value={draft.bedrockMaxPlayers}
        min={1}
        max={10000}
        width="80px"
        onchange={(value) => (draft.bedrockMaxPlayers = Number(value) || 10)}
      />
      <p class="hint">Port and connectivity options are on the next step.</p>
    </section>
  {/if}

  {#if showXboxBroadcast}
    <section class="block" use:onboardingAnchor={'ob_server_xbox_broadcast'}>
      <p class="msc2-type-overline">Xbox Broadcast</p>
      <div class="toggle-card">
        <div class="toggle-row">
          <Toggle
            checked={draft.enableXboxBroadcast}
            label="Enable Xbox Broadcast"
            onchange={(enabled) => (draft.enableXboxBroadcast = enabled)}
          />
          <span class="toggle-text">
            <span class="toggle-name">Enable Xbox Broadcast</span>
            <span class="toggle-hint"
              >Let console, mobile, and PC players see your server in the Xbox Friends tab. MSC
              downloads the broadcast tool automatically.</span
            >
          </span>
        </div>
      </div>
    </section>
  {/if}
</div>

<style>
  .configure {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .cards {
    display: flex;
    gap: 10px;
  }
  .cards.two-up > .card {
    flex: 1;
  }

  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    text-align: left;
    padding: 12px 14px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 10px;
    font: inherit;
    cursor: pointer;
  }
  .card.selected {
    border-color: rgba(255, 255, 255, 0.32);
    background: rgba(255, 255, 255, 0.05);
  }
  .card-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .card-subtitle {
    font-size: 11.5px;
    color: var(--msc2-text-tertiary);
  }

  .flavor-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .flavor-row {
    display: flex;
    align-items: flex-start;
    text-align: left;
    padding: 10px 12px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 10px;
    font: inherit;
    cursor: pointer;
  }
  .flavor-row.selected {
    border-color: rgba(255, 255, 255, 0.32);
    background: rgba(255, 255, 255, 0.05);
  }
  .flavor-row:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .flavor-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .flavor-name-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .flavor-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .flavor-description {
    font-size: 11.5px;
    color: var(--msc2-text-tertiary);
  }

  .hint {
    margin: 0;
    font-size: 11.5px;
    color: var(--msc2-text-tertiary);
  }
  .hint.warn {
    color: var(--msc2-status-warn);
  }

  .version-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 220px;
    overflow-y: auto;
    padding: 4px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 10px;
  }
  .version-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 7px 9px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 7px;
    color: var(--msc2-text-primary);
    font: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .version-row.selected {
    border-color: rgba(255, 255, 255, 0.28);
    background: rgba(255, 255, 255, 0.05);
  }
  .version-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toggle-card {
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 10px;
    padding: 12px 14px;
  }
  .toggle-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }
  .toggle-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .toggle-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .toggle-hint {
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
</style>
