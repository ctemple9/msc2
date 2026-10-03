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
  import WorldPackBrowserSheet from '../../worlds/WorldPackBrowserSheet.svelte';
  import type { PendingWorldPack } from './model';
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

  let showPackBrowser = false;
  let pendingPackSource: FileChunkSource | undefined;
  let packError = '';
  let resolvedMinecraftVersion = '';
  $: packs = (draft.pendingWorldPacks ?? []).filter((pack) => pack.edition === draft.serverType);

  function stagePack(pack: PendingWorldPack): void {
    const current = draft.pendingWorldPacks ?? [];
    const duplicate = current.some((entry) =>
      entry.kind === 'javaCatalog' && pack.kind === 'javaCatalog'
        ? entry.projectId === pack.projectId && entry.versionId === pack.versionId
        : entry.kind === 'bedrockCatalog' && pack.kind === 'bedrockCatalog'
          ? entry.projectId === pack.projectId && entry.fileId === pack.fileId
          : false,
    );
    if (!duplicate) draft.pendingWorldPacks = [...current, pack];
  }

  async function choosePack(): Promise<void> {
    packError = '';
    try {
      pendingPackSource =
        (await (
          await getPlatform()
        ).pickFileStream({
          label: draft.serverType === 'java' ? 'Choose a datapack ZIP' : 'Choose a Bedrock pack',
          extensions: draft.serverType === 'java' ? ['zip'] : ['mcpack', 'mcaddon', 'zip'],
        })) ?? undefined;
    } catch (error) {
      packError = errorMessage(error);
    }
  }

  function finishPackUpload(upload: Schema['StagedUploadCompleteResultDTO']): void {
    if (!pendingPackSource) throw new Error('The selected pack is no longer available.');
    stagePack({
      id: crypto.randomUUID(),
      kind: 'localFile',
      edition: draft.serverType,
      title: pendingPackSource.name,
      stagedUploadId: upload.stagedUploadId,
    });
  }

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
      resolvedMinecraftVersion = '';
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
      if (!pack?.minecraftVersion) {
        const versions = await api.get<Schema['VersionsResponseDTO']>(
          versionsForCreatePath(draft.serverType, draft.javaFlavor),
        );
        if (requestKey !== capabilityRequestKey) return;
        const resolved =
          versions.versions?.find(
            (entry) => entry.id === minecraftVersion || entry.mcVersion === minecraftVersion,
          ) ??
          versions.versions?.find((entry) => entry.isLatest) ??
          versions.versions?.[0];
        minecraftVersion = resolved?.mcVersion;
        // Pin the catalog and download to the same release. Loader selections keep their full ID.
        if (resolved && minecraftVersion) {
          if (draft.serverType === 'bedrock') draft.bedrockVersion = minecraftVersion;
          else draft.versionId = resolved.id;
        }
      }
      resolvedMinecraftVersion = minecraftVersion ?? '';
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
  <details class="packs-disclosure">
    <summary
      ><span
        ><span class="packs-title">Packs{packs.length ? ` (${packs.length})` : ''}</span><span
          class="packs-subtitle"
          >{draft.serverType === 'java'
            ? 'Datapacks for this world'
            : 'Behavior and resource packs for this world'}</span
        ></span
      ><span class="chevron" aria-hidden="true">⌄</span></summary
    >
    {#if packs.length === 0}
      <div class="pack-choices">
        <button
          type="button"
          class="pack-choice"
          disabled={!api || !resolvedMinecraftVersion || resolvingVersion}
          onclick={() => (showPackBrowser = true)}
        >
          <span>{draft.serverType === 'java' ? 'Browse Datapacks' : 'Browse Packs'}</span>
          <small
            >{draft.serverType === 'java'
              ? 'Search Modrinth for datapacks for this Minecraft version.'
              : 'Browse behavior and resource packs on CurseForge.'}</small
          >
        </button>
        <button
          type="button"
          class="pack-choice"
          disabled={!api?.uploadFile}
          onclick={() => void choosePack()}
        >
          <span>{draft.serverType === 'java' ? 'Import Datapacks' : 'Import Packs'}</span>
          <small
            >{draft.serverType === 'java'
              ? 'Add your own datapack ZIP.'
              : 'Add a .mcpack, .mcaddon, or .zip file.'}</small
          >
        </button>
      </div>
    {:else}
      <div class="pack-heading">
        <span>{draft.serverType === 'java' ? 'Datapacks' : 'Packs'} ({packs.length})</span>
        <div class="pack-actions">
          <Button
            size="sm"
            variant="secondary"
            disabled={!api?.uploadFile}
            onclick={() => void choosePack()}>Import…</Button
          >
          <Button
            size="sm"
            variant="secondary"
            disabled={!api || !resolvedMinecraftVersion || resolvingVersion}
            onclick={() => (showPackBrowser = true)}
            >{draft.serverType === 'java' ? 'Browse Datapacks' : 'Browse Packs'}</Button
          >
        </div>
      </div>
      {#each packs as pack (pack.id)}
        <div class="pack-row">
          {#if pack.iconURL}<img src={pack.iconURL} alt="" width="32" height="32" />{/if}
          <div class="pack-info">
            <span>{pack.title}</span>{#if pack.description}<small>{pack.description}</small>{/if}
          </div>
          <Button
            size="sm"
            variant="secondary"
            onclick={() =>
              (draft.pendingWorldPacks = (draft.pendingWorldPacks ?? []).filter(
                (entry) => entry.id !== pack.id,
              ))}>Remove</Button
          >
        </div>
      {/each}
    {/if}
    <p class="hint">
      These install in the first world after the server is created.{draft.serverType === 'bedrock'
        ? ' Linked behavior and resource packs install together.'
        : ' Imported datapack compatibility is not verified.'}
    </p>
    {#if packError}<p class="hint warn" role="status">{packError}</p>{/if}
  </details>
</div>

{#if showPackBrowser}
  <WorldPackBrowserSheet
    {api}
    bedrock={draft.serverType === 'bedrock'}
    minecraftVersion={resolvedMinecraftVersion}
    stagedPacks={packs}
    onStage={stagePack}
    onInstalled={() => {}}
    onClose={() => (showPackBrowser = false)}
  />
{/if}
{#if pendingPackSource}
  <StagedUploadSheet
    {api}
    purpose="addon-local-file"
    source={pendingPackSource}
    onComplete={finishPackUpload}
    onClose={() => (pendingPackSource = undefined)}
  />
{/if}

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
  .packs-disclosure {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 0;
    cursor: pointer;
    list-style: none;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  .packs-title {
    display: block;
    font-size: 13px;
    font-weight: 500;
  }
  .packs-subtitle,
  small {
    display: block;
    margin-top: 2px;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }
  .chevron {
    color: var(--msc2-text-tertiary);
  }
  details[open] .chevron {
    transform: rotate(180deg);
  }
  .pack-choices {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 12px;
  }
  .pack-choice {
    text-align: left;
    padding: 14px;
    background: none;
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 10px;
    color: var(--msc2-text-primary);
    font: inherit;
    cursor: pointer;
  }
  .pack-choice:hover {
    background: var(--msc2-neutral-muted);
  }
  .pack-choice:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .pack-choice span,
  .pack-heading {
    font-size: 13px;
    font-weight: 500;
  }
  .pack-heading,
  .pack-actions,
  .pack-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .pack-heading {
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .pack-row {
    padding: 12px 0;
  }
  .pack-row img {
    object-fit: cover;
    border-radius: 6px;
  }
  .pack-info {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 500;
  }
  .pack-info small {
    font-weight: 400;
  }

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
