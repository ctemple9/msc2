<script lang="ts">
  // Real port of AddServerWizardView.swift's step4FreshWorld -- the source
  // picker stays here, while world-local settings use the same form as the
  // Worlds tab. The complete profile is carried into the create request so
  // creation-only choices reach the server before its first world starts.
  //
  // Real gap found and handled, not silently worked around: the oracle
  // offers a third World Source, an existing world *folder*, alongside New
  // World and a backup ZIP. `ServerCreateRequestDTO` has no field to carry a
  // pre-made world into creation at all (confirmed against the frozen
  // contract -- worldName/worldSeed are the only world-shape fields it
  // takes), so both non-fresh sources need the same staged-upload mechanism
  // `worlds/ImportWorldZipSheet.svelte` uses to stream a selected ZIP in
  // bounded chunks. A folder has no such primitive anywhere in this codebase or the
  // contract -- `worlds/ReplaceWorldSheet.svelte` already hit this exact
  // gap for the identical oracle picker ("World Folder…") and dropped it
  // for the same reason: "a file picker has no folder-to-archive
  // equivalent." This step follows that same precedent rather than
  // re-litigating it: the World Source picker below offers only New World
  // and From Backup (.zip); `WizardDraft.worldSourceMode` has no `folder`
  // value to select in the first place, so there is nothing to fake a
  // picker control for.
  //
  // The backup ZIP is staged after the user confirms the transfer size;
  // staging commits nothing -- only P12.18g's real create call redeems the
  // resulting stagedUploadId.
  import Button from '../../../components/base/Button.svelte';
  import SegmentedControl from '../../../components/base/SegmentedControl.svelte';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import { getPlatform } from '../../../platform';
  import type { FileChunkSource } from '../../../platform/types';
  import type { Schema, ScreenApi } from '../../shared/types';
  import { errorMessage } from '../../shared/types';
  import StagedUploadSheet from '../../components/StagedUploadSheet.svelte';
  import WorldSettingsForm from '../../worlds/WorldSettingsForm.svelte';
  import {
    defaultWorldSettingsValues,
    type WorldSettingsCapabilities,
    type WorldSettingsValues,
  } from '../../worlds/model';
  import { type WizardDraft, type WorldSourceMode, versionsForCreatePath } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;
  export let resolvingVersion = false;

  let staging = false;
  let pendingWorldSource: FileChunkSource | undefined;
  let stageError: string | undefined;
  let capabilities: WorldSettingsCapabilities | undefined;
  let capabilitiesError: string | undefined;
  let capabilityRequestKey = '';
  let worldSettings: WorldSettingsValues = draft.worldSettings ?? {
    ...defaultWorldSettingsValues(draft.serverType),
    name: draft.worldName,
    seed: draft.worldSeed,
    difficulty: draft.worldDifficulty,
    defaultGameMode: draft.worldGamemode,
  };

  if (!draft.worldSettings) draft.worldSettings = worldSettings;

  $: {
    const pack = draft.stagedModpack?.inspection;
    const requestKey = [
      draft.serverType,
      draft.javaFlavor,
      pack?.minecraftVersion ??
        (draft.serverType === 'bedrock' ? draft.bedrockVersion : draft.versionId) ??
        'latest',
      pack?.loaderVersion ?? '',
    ].join('|');
    if (requestKey !== capabilityRequestKey) {
      capabilityRequestKey = requestKey;
      capabilities = undefined;
      if (api) void loadCapabilities(requestKey);
    }
  }

  async function loadCapabilities(requestKey: string): Promise<void> {
    if (!api) return;
    capabilitiesError = undefined;
    resolvingVersion = true;
    try {
      const pack = draft.stagedModpack?.inspection;
      let minecraftVersion =
        pack?.minecraftVersion ??
        (draft.serverType === 'bedrock' ? draft.bedrockVersion : draft.versionId);
      if (!minecraftVersion || minecraftVersion.toLowerCase() === 'latest') {
        const versions = await api.get<Schema['VersionsResponseDTO']>(
          versionsForCreatePath(draft.serverType, draft.javaFlavor),
        );
        if (requestKey !== capabilityRequestKey) return;
        const resolved =
          versions.versions?.find((entry) => entry.isLatest) ?? versions.versions?.[0];
        minecraftVersion = resolved?.mcVersion;
        // Pin the catalog and download to the same release. Loader selections keep their full ID.
        if (resolved && minecraftVersion) {
          if (draft.serverType === 'bedrock') draft.bedrockVersion = minecraftVersion;
          else draft.versionId = resolved.id;
        }
      }
      const params = new URLSearchParams({ serverType: draft.serverType });
      if (minecraftVersion) params.set('minecraftVersion', minecraftVersion);
      if (draft.serverType === 'java') params.set('javaFlavor', draft.javaFlavor);
      if (pack?.loaderVersion) params.set('loaderVersion', pack.loaderVersion);
      const response = await api.get<{ worldSettings?: WorldSettingsCapabilities }>(
        `/v1/capabilities?${params.toString()}`,
      );
      if (requestKey !== capabilityRequestKey) return;
      capabilities = response.worldSettings;
    } catch (error) {
      if (requestKey === capabilityRequestKey) {
        capabilities = undefined;
        capabilitiesError = errorMessage(error);
      }
    } finally {
      if (requestKey === capabilityRequestKey) resolvingVersion = false;
    }
  }

  function selectSourceMode(mode: string): void {
    draft.worldSourceMode = mode as WorldSourceMode;
    stageError = undefined;
  }

  async function chooseBackup(): Promise<void> {
    if (!api?.uploadFile || staging) return;
    staging = true;
    stageError = undefined;
    try {
      const picked = await (
        await getPlatform()
      ).pickFileStream({
        label: 'Choose a world backup ZIP',
        extensions: ['zip'],
      });
      if (!picked) {
        staging = false;
        return;
      }
      pendingWorldSource = picked;
    } catch (error) {
      stageError = errorMessage(error);
      staging = false;
    }
  }

  function finishWorldUpload(upload: Schema['StagedUploadCompleteResultDTO']): void {
    if (!pendingWorldSource) throw new Error('The selected world archive is no longer available.');
    draft.stagedWorldBackup = {
      fileName: pendingWorldSource.name,
      stagedUploadId: upload.stagedUploadId,
    };
  }

  function closePendingWorld(): void {
    pendingWorldSource = undefined;
    staging = false;
  }

  function updateWorldSettings(next: WorldSettingsValues): void {
    worldSettings = next;
    draft.worldName = next.name;
    draft.worldSeed = next.seed;
    draft.worldDifficulty = next.difficulty as WizardDraft['worldDifficulty'];
    draft.worldGamemode = next.defaultGameMode as WizardDraft['worldGamemode'];
  }
</script>

<div class="world">
  <div class="intro">
    <h2>What should the first world be?</h2>
    <p>Start with a brand new world, or bring in one from a backup.</p>
  </div>

  <section class="block" use:onboardingAnchor={'ob_world_source'}>
    <p class="msc2-type-overline">World Source</p>
    <SegmentedControl
      options={[
        { value: 'fresh', label: 'New World' },
        { value: 'backupZip', label: 'From Backup (.zip)' },
      ]}
      value={draft.worldSourceMode}
      onchange={selectSourceMode}
    />
  </section>

  {#if draft.worldSourceMode === 'fresh'}
    <div use:onboardingAnchor={'ob_world_creation'}>
      <WorldSettingsForm
        {api}
        mode="wizard"
        heading="First world settings"
        serverType={draft.serverType}
        values={worldSettings}
        {capabilities}
        onChange={updateWorldSettings}
      />
      {#if capabilitiesError}
        <p class="hint warn" role="status">
          Advanced settings could not be checked: {capabilitiesError} Native fields remain available where
          this edition supports them.
        </p>
      {/if}
    </div>
  {:else}
    <section class="block">
      <Button variant="secondary" disabled={staging} onclick={() => void chooseBackup()}>
        {staging ? 'Preparing backup…' : 'Choose backup .zip…'}
      </Button>
      {#if draft.stagedWorldBackup}
        <p class="hint">Selected: {draft.stagedWorldBackup.fileName}</p>
      {:else if stageError}
        <p class="hint warn">{stageError}</p>
      {:else}
        <p class="hint">No file selected.</p>
      {/if}
    </section>
  {/if}
</div>

{#if pendingWorldSource}
  <StagedUploadSheet
    {api}
    purpose="world-import"
    source={pendingWorldSource}
    onComplete={finishWorldUpload}
    onClose={closePendingWorld}
  />
{/if}

<style>
  .world {
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
  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .hint.warn {
    color: var(--msc2-status-warn);
  }
  .hidden-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    overflow: hidden;
  }
</style>
