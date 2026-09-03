<script lang="ts">
  // Real port of AddServerWizardView.swift's step5Confirm/confirmFormView --
  // a Display Name field, a read-only summary of every prior pick, the
  // Modded "every player needs the loader" note, and (while creating) a
  // progress line. This component only presents that state; the real
  // POST /v1/servers/create call, its operation-progress polling, and the
  // staged-world-backup/pending-add-on redemption that follows it all live
  // in AddServerWizard.svelte + model.ts's `createServerFromDraft`, the same
  // "parent owns the footer's primary action, each step is a presentational
  // view over the shared draft" shape ConfigureStep/NetworkStep/WorldStep/
  // AddOnsStep already established -- Create/Done replace Continue on
  // AddServerWizard's own footer rather than this component growing its own.
  //
  // Success state reuses StatusDot (docs/msc2/antiAIslop.md tell #12's own
  // "correct usage" example -- a defined state, always labeled) instead of
  // the oracle's large accent-colored checkmark circle, which is exactly
  // the icon-in-a-tinted-box tell (#6) applied to a status readout.
  //
  // P12.18h adds the Import path's own summary/hint branch alongside the
  // Fresh one this step already had -- same component, same "parent owns
  // the footer" shape, just a `path`-gated content swap (mirrors the
  // oracle's own `confirmFormView`/`postCreateHint`, which branch on
  // `wizardPath` the same way).
  import StatusDot from '../../../components/base/StatusDot.svelte';
  import Field from '../../../components/base/Field.svelte';
  import Toggle from '../../../components/base/Toggle.svelte';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import {
    JAVA_CATEGORY_INFO,
    JAVA_FLAVOR_CATALOG,
    javaAddOnKind,
    hasStagedSimpleVoiceChat,
    versionEntryLabel,
    versionsForCreatePath,
    type WizardDraft,
    type WizardPath,
  } from './model';
  import type { Schema, ScreenApi } from '../../shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let path: WizardPath;
  export let draft: WizardDraft;
  export let displayName: string;
  export let isCreating: boolean;
  export let statusMessage: string;
  export let createSucceeded: boolean;
  export let createWarnings: readonly string[] = [];

  $: importScan = draft.importScan;
  $: importActiveWorldName = draft.importActiveWorldName ?? importScan?.defaultWorldName;
  $: importOtherWorlds =
    importScan && importScan.worlds && importScan.worlds.length > 1
      ? importScan.worlds
          .filter((world) => world.name !== importActiveWorldName)
          .map((world) => world.name)
          .join(', ')
      : '';

  $: flavorInfo = JAVA_FLAVOR_CATALOG.find((entry) => entry.id === draft.javaFlavor);
  $: addOnKind = javaAddOnKind(draft.javaFlavor);
  $: addOnNoun = addOnKind === 'plugin' ? 'Plugins' : 'Mods';
  $: totalStagedAddOns = draft.pendingAddOns.length;
  $: hasStagedVoiceChat = hasStagedSimpleVoiceChat(draft);
  $: isPackCreation = draft.stagedModpack !== undefined;
  $: isExistingImport = path === 'importExisting' && !isPackCreation;
  $: packInspection = draft.stagedModpack?.inspection;

  // The oracle keeps one shared `selectedVersionEntry` state var visible to
  // every step; this port only knows the picked `versionId` by the time it
  // reaches Confirm, so -- matching AddOnsStep.svelte's identical situation
  // -- it re-resolves the flavor's version list here rather than threading
  // more state back through Configure.
  let pinnedVersionLabel: string | undefined;
  $: if (api && draft.serverType === 'java' && draft.versionId) {
    void resolvePinnedVersionLabel(draft.versionId);
  } else {
    pinnedVersionLabel = undefined;
  }

  async function resolvePinnedVersionLabel(versionId: string): Promise<void> {
    try {
      const response = await api?.get<Schema['VersionsResponseDTO']>(
        versionsForCreatePath('java', draft.javaFlavor),
      );
      const entry = response?.versions?.find((candidate) => candidate.id === versionId);
      pinnedVersionLabel = entry ? versionEntryLabel(entry) : undefined;
    } catch {
      pinnedVersionLabel = undefined;
    }
  }
</script>

<div class="confirm" use:onboardingAnchor={'ob_confirm_page'}>
  {#if createSucceeded}
    <div class="success">
      <StatusDot tone="ok" label="{displayName || draft.serverName} created" />
      <p class="hint">
        {#if isExistingImport}
          Open Server Settings to review defaults.{#if !draft.importEulaAccepted}
            The EULA still needs to be accepted before this server can start.{/if}
        {:else if draft.worldSourceMode === 'fresh'}
          Open Server Settings to review defaults, then choose Initiate in the server controls for
          the one-time file, world, and connection setup.
        {:else if draft.javaCategory === 'modded'}
          Add mods in the Components tab before starting — world-gen mods must be present on first
          boot.
        {:else}
          Open Server Settings to review defaults. Install {addOnKind === 'plugin'
            ? 'plugins'
            : 'add-ons'} from the Components tab any time.
        {/if}
      </p>
      {#each createWarnings as warning (warning)}
        <p class="hint warn">{warning}</p>
      {/each}
    </div>
  {:else}
    <div class="intro">
      <h2>Name and confirm</h2>
      <p>Review the summary below, then give your server a display name.</p>
    </div>

    <section class="block">
      <p class="msc2-type-overline">Display Name</p>
      <Field bind:value={displayName} placeholder="Server display name" disabled={isCreating} />
    </section>

    <section class="block">
      <p class="msc2-type-overline">Server settings — apply to every world</p>
      <div class="summary">
        {#if isExistingImport}
          <div class="row">
            <span class="label">Method</span>
            <span class="value">Import existing</span>
          </div>
          <div class="row">
            <span class="label">Server type</span>
            <span class="value">{draft.serverType === 'java' ? 'Java' : 'Bedrock'}</span>
          </div>
          <div class="row">
            <span class="label">Port</span>
            <span class="value"
              >{draft.serverType === 'java' ? draft.javaPort : draft.bedrockPort}</span
            >
          </div>
          <div class="row">
            <span class="label">Connectivity</span>
            <span class="value"
              >{draft.enablePlayit ? 'Tunnel (playit.gg)' : 'Port Forwarding'}</span
            >
          </div>
          <div class="row">
            <span class="label">Max players</span>
            <span class="value">{draft.importMaxPlayers}</span>
          </div>
          {#if !draft.importEulaAccepted}
            <div class="row">
              <span class="label">EULA</span>
              <span class="value warn">Not yet accepted — accept before starting</span>
            </div>
          {/if}
        {:else}
          <div class="row">
            <span class="label">Server type</span>
            <span class="value">{draft.serverType === 'java' ? 'Java' : 'Bedrock'}</span>
          </div>
          {#if draft.serverType === 'java'}
            <div class="row">
              <span class="label">Software</span>
              <span class="value"
                >{flavorInfo?.displayName ?? draft.javaFlavor} · {JAVA_CATEGORY_INFO[
                  draft.javaCategory
                ].displayName}</span
              >
            </div>
            <div class="row">
              <span class="label">Version</span>
              <span class="value"
                >{packInspection?.minecraftVersion
                  ? `${packInspection.minecraftVersion} · ${packInspection.loaderName ?? flavorInfo?.displayName ?? draft.javaFlavor}${packInspection.loaderVersion ? ` ${packInspection.loaderVersion}` : ''}`
                  : (pinnedVersionLabel ??
                    `Latest ${flavorInfo?.displayName ?? draft.javaFlavor}`)}</span
              >
            </div>
            <div class="row">
              <span class="label">Java Port</span>
              <span class="value">{draft.javaPort}</span>
            </div>
            {#if draft.enableCrossPlay}
              <div class="row">
                <span class="label">Bedrock Port</span>
                <span class="value">{draft.crossPlayBedrockPort}</span>
              </div>
            {/if}
          {:else}
            <div class="row">
              <span class="label">Bedrock Version</span>
              <span class="value">{draft.bedrockVersion.trim() || 'LATEST'}</span>
            </div>
            <div class="row">
              <span class="label">Max Players</span>
              <span class="value">{draft.bedrockMaxPlayers}</span>
            </div>
            <div class="row">
              <span class="label">Port</span>
              <span class="value">{draft.bedrockPort}</span>
            </div>
          {/if}
          <div class="row">
            <span class="label">Connectivity</span>
            <span class="value"
              >{draft.enablePlayit ? 'Tunnel (playit.gg)' : 'Port Forwarding'}</span
            >
          </div>
          {#if hasStagedVoiceChat}
            <div class="row">
              <span class="label">Voice chat</span>
              <span class="value">Simple Voice Chat staged</span>
            </div>
          {/if}
          {#if draft.stagedModpack}
            <div class="row">
              <span class="label">Modpack</span>
              <span class="value"
                >{draft.stagedModpack.inspection.packName ?? draft.stagedModpack.fileName}</span
              >
            </div>
          {/if}
          {#if isPackCreation}
            <div class="row">
              <span class="label">Component policy</span>
              <span class="value">Managed by this modpack</span>
            </div>
          {/if}
          {#if totalStagedAddOns > 0}
            <div class="row">
              <span class="label">{addOnNoun}</span>
              <span class="value">{totalStagedAddOns} staged</span>
            </div>
          {/if}
        {/if}
      </div>
    </section>

    <section class="block">
      <p class="msc2-type-overline">
        {isExistingImport
          ? 'Imported world settings — saved with this world'
          : 'First world settings — saved with this world'}
      </p>
      <div class="summary">
        {#if isExistingImport}
          <div class="row">
            <span class="label">Active world</span>
            <span class="value">{importActiveWorldName ?? '—'}</span>
          </div>
          {#if importOtherWorlds}
            <div class="row">
              <span class="label">Other worlds</span>
              <span class="value">{importOtherWorlds}</span>
            </div>
          {/if}
          <div class="row">
            <span class="label">World profile</span>
            <span class="value">Imported from the selected server</span>
          </div>
        {:else}
          <div class="row">
            <span class="label">World source</span>
            <span class="value"
              >{draft.worldSourceMode === 'fresh' ? 'New world' : 'From backup (.zip)'}</span
            >
          </div>
          {#if draft.worldSourceMode === 'fresh'}
            <div class="row">
              <span class="label">World name</span>
              <span class="value">{draft.worldName.trim() || draft.serverName || '—'}</span>
            </div>
            <div class="row">
              <span class="label">Difficulty</span>
              <span class="value">{draft.worldDifficulty}</span>
            </div>
            <div class="row">
              <span class="label">Default game mode</span>
              <span class="value">{draft.worldGamemode}</span>
            </div>
            <div class="row">
              <span class="label">Seed</span>
              <span class="value">{draft.worldSeed.trim() || 'Random'}</span>
            </div>
          {:else if draft.stagedWorldBackup}
            <div class="row">
              <span class="label">Backup file</span>
              <span class="value">{draft.stagedWorldBackup.fileName}</span>
            </div>
          {/if}
        {/if}
      </div>

      {#if !isExistingImport && draft.worldGamemode === 'creative'}
        <p class="hint warn">
          Creative is saved with this world. The agent will ask for an acknowledgement before it
          applies the gameplay change.
        </p>
      {/if}
      {#if !isExistingImport && draft.serverType === 'java' && draft.javaCategory === 'modded'}
        <p class="hint">
          To join, every player needs the {flavorInfo?.displayName ?? draft.javaFlavor} loader for this
          Minecraft version, plus the same mods installed.
        </p>
      {/if}
    </section>

    <section class="block preference-block">
      <div class="preference-copy">
        <p class="msc2-type-overline">Update checks</p>
        <p class="preference-title">Check for mod/plugin updates</p>
        <p class="hint">
          Off by default. When enabled, MSC checks providers for compatible updates after the local
          component list loads. This is especially useful for Paper servers using Geyser and
          Floodgate for Bedrock cross-play.
        </p>
      </div>
      <Toggle
        checked={draft.checkAddonUpdates}
        label="Check for mod/plugin updates"
        disabled={isCreating}
        onchange={(checked) => (draft.checkAddonUpdates = checked)}
      />
    </section>

    {#if isCreating}
      <div class="progress">
        <span class="spinner" aria-hidden="true"></span>
        <span class="hint">{statusMessage || 'Working…'}</span>
      </div>
    {:else if statusMessage}
      <p class="hint warn">{statusMessage}</p>
    {/if}
  {/if}
</div>

<style>
  .confirm {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .intro {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .intro h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .intro p {
    margin: 0;
    font-size: 12.5px;
    color: var(--msc2-text-tertiary);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  .block > .summary {
    width: 100%;
  }

  .preference-block {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .preference-copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .preference-copy .msc2-type-overline,
  .preference-copy p {
    margin: 0;
  }
  .preference-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }

  .summary {
    display: flex;
    flex-direction: column;
    background: var(--msc2-tier-chrome);
    border-radius: 10px;
    overflow: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .row:first-child {
    border-top: none;
  }
  .label {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .value {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-primary);
    text-align: right;
  }
  .value.warn {
    color: var(--msc2-status-warn);
  }

  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .hint.warn {
    color: var(--msc2-status-warn);
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .spinner {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    border-radius: 50%;
    border: 2px solid var(--msc2-hairline-subtle);
    border-top-color: var(--msc2-text-secondary);
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .success {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 0;
  }
</style>
