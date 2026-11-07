<script lang="ts">
  // Real port of AddServerWizardView.swift's step5AddOns, shown only when
  // hasAddOnsStep(draft) is true (model.ts). Values only accumulate into the
  // wizard's draft state -- nothing installs yet, matching every prior
  // Configure/Network/World step's own established pattern; the real
  // POST /v1/servers/create call is P12.18g's job, and that same step is
  // where draft.pendingAddOns/stagedModpack actually get redeemed (one
  // POST /v1/components/install call per pending item, and
  // ServerCreateRequestDTO.stagedModpackUploadId), once the server they're
  // for actually exists.
  //
  // Real gap found and fixed, not worked around: an earlier version of this
  // step dropped Browse mods/plugins entirely, because GET /v1/catalog/search and
  // POST /v1/components/install both hard-required an already-active server
  // (crates/msc-agent/src/routes/components.rs), which doesn't exist yet
  // during the wizard. the owner asked for real parity with MSC 1 here --
  // AddServerWizardView.swift's own Add-ons step really does let you search
  // Modrinth and add your own files, via ModrinthBrowserView reused in a
  // staging mode (`onAddToStaging`, `wizardStagingConfig` carrying the
  // flavor being configured) instead of installing directly. So the fix
  // amends the same shape server-side: `search_catalog` now accepts optional
  // `javaFlavor`/`minecraftVersion` query params that resolve loaders/add-on
  // kind directly, bypassing the active-server lookup entirely when given
  // (additive; omitted, the route's original active-server-scoped behavior
  // is unchanged) -- see the contract's own `x-notes` on `/v1/catalog/search`.
  // `PluginBrowserSheet.svelte`/`ProjectDetailSheet.svelte` gained the same
  // additive `mode="stage"` (an `onStage`/`onStaged` callback instead of the
  // real install call) that ModrinthBrowserView's own `onAddToStaging` is,
  // reused here unmodified for their real component-picking/gallery/version
  // UI rather than duplicating it.
  //
  // POST /v1/components/install itself was left untouched -- it still
  // requires an active server, same as before -- because staging now defers
  // every install call (both catalog picks and local jars) until P12.18g's
  // real create call makes the new server active, exactly the point at
  // which that route is correct to call. That mirrors the oracle's own
  // `applyStagedAddOn`, called only after `createNewServer` has a real
  // server directory to install into.
  //
  // What's still an honest simplification rather than a silent drop: the
  // platform file-picker (`platform/types.ts`'s `pickFile`) only returns one
  // file at a time, unlike the
  // oracle's native panel (`allowsMultipleSelection`) or its folder-of-jars
  // sniffing in `processModpackURL`. "Add your own .jar" here is a repeatable
  // single-file pick instead -- click it again to add another -- rather than
  // a batch folder/zip-of-loose-jars import, since MSC 2's contract has no
  // route to unpack an arbitrary zip of jars server-side (only a structured
  // mrpack/CurseForge manifest, via the modpack path below).
  import { onMount, onDestroy } from 'svelte';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import Button from '../../../components/base/Button.svelte';
  import { pickUploadSource, sourceApi, type UploadSource } from '../../shared/upload-source';
  import type { Schema, ScreenApi } from '../../shared/types';
  import { errorMessage, mutate } from '../../shared/types';
  import { addonPaths } from '../../addons/model';
  import PluginBrowserSheet from '../../components/PluginBrowserSheet.svelte';
  import PackCatalogSheet from '../../components/PackCatalogSheet.svelte';
  import { loadCatalogProjectPage } from '../../components/catalog-project';
  import Sheet from '../../../components/base/Sheet.svelte';
  import {
    discardWizardInput,
    retainWizardUpload,
    retainWizardUploadApi,
    javaSelectionKey,
  } from './model';
  import StagedUploadSheet from '../../components/StagedUploadSheet.svelte';
  import {
    hasStagedSimpleVoiceChat,
    javaAddOnKind,
    versionsForCreatePath,
    type WizardDraft,
  } from './model';

  let disposed = false;
  onDestroy(() => {
    disposed = true;
  });
  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;

  let modpackFileInput: HTMLInputElement;
  let jarFileInput: HTMLInputElement;
  let stagingModpack = false;
  let pendingModpackSource: UploadSource | undefined;
  let stagingJar = false;
  let pendingJarSource: UploadSource | undefined;
  let stageError: string | undefined;
  let showBrowser = false;
  let showPacks = false;
  let packCandidate:
    | { id: string; name: string; inspection: Schema['ModpackInspectionResultDTO']; api: ScreenApi }
    | undefined;
  let matchingPackSetup = false;
  let creationPackSetupSupported = false;
  let reviewedStaged = false;
  let omitted = new Set<string>();
  let resolvedLoaderVersion: string | undefined;
  let stagedCompatibility: Record<string, string> = {};
  let checkingStaged = false;
  let acceptingPack = false;
  async function recheckStaged(candidate: NonNullable<typeof packCandidate>): Promise<void> {
    checkingStaged = true;
    const notes: Record<string, string> = {};
    for (const item of draft.pendingAddOns) {
      if (disposed || packCandidate !== candidate) return;
      if (item.kind === 'localFile') {
        notes[item.id] = 'Local file: compatibility unknown; review before keeping.';
        continue;
      }
      try {
        const page = await loadCatalogProjectPage(candidate.api, item.provider ?? 'modrinth', {
          projectId: item.projectId,
          title: item.title,
          projectType: addOnKind,
        });
        const version = page.versions.find((value) => value.id === item.versionId);
        if (!version) {
          notes[item.id] = 'Exact staged release not returned; compatibility unknown.';
          continue;
        }
        const matches = (mc: string | undefined, loader: string | undefined) =>
          !!mc && version.gameVersions.includes(mc) && !!loader && version.loaders.includes(loader);
        notes[item.id] =
          `Pack setup: ${matches(candidate.inspection.minecraftVersion, candidate.inspection.loaderName?.toLowerCase()) ? 'matches declared versions' : 'does not match declared versions'}. Current setup: ${matches(resolvedMcVersion, draft.javaFlavor) ? 'matches declared versions' : 'does not match declared versions'}.`;
      } catch (cause) {
        notes[item.id] = `Compatibility unknown: ${errorMessage(cause)}`;
      }
    }
    if (!disposed && packCandidate === candidate) {
      stagedCompatibility = notes;
      checkingStaged = false;
    }
  }
  function packMatchesCurrentSetup(inspection: Schema['ModpackInspectionResultDTO']): boolean {
    return (
      inspection.success &&
      !!resolvedMcVersion &&
      !!resolvedLoaderVersion &&
      inspection.minecraftVersion === resolvedMcVersion &&
      inspection.loaderName?.toLowerCase() === draft.javaFlavor &&
      inspection.loaderVersion === resolvedLoaderVersion
    );
  }
  function stageReviewedPack(
    candidate: NonNullable<typeof packCandidate>,
    mode: 'matching' | 'current',
  ): void {
    draft.stagedModpack = {
      fileName: candidate.name,
      stagedUploadId: candidate.id,
      inspection: candidate.inspection,
      setupMode: mode,
      currentMinecraftVersion: mode === 'current' ? resolvedMcVersion : undefined,
      currentLoaderVersion: mode === 'current' ? resolvedLoaderVersion : undefined,
    };
    // The setup identity includes the staged pack's Minecraft/loader metadata.
    // Record it after assignment so the pack itself is not mistaken for an edit.
    draft.stagedModpack.reviewedSetupKey = javaSelectionKey(draft);
    packCandidate = undefined;
  }
  function reviewPack(
    id: string,
    name: string,
    inspection: Schema['ModpackInspectionResultDTO'],
    scoped: ScreenApi,
  ): void {
    if (draft.stagedModpack)
      throw new Error('Remove the current base pack before selecting another.');
    retainWizardUploadApi(draft, scoped, id);
    packCandidate = { id, name, inspection, api: scoped };
    matchingPackSetup = packMatchesCurrentSetup(inspection);
    stageError = undefined;
    reviewedStaged = draft.pendingAddOns.length === 0;
    omitted = new Set();
    showPacks = false;
    stagedCompatibility = {};
    if (
      matchingPackSetup &&
      creationPackSetupSupported &&
      !draft.pendingAddOns.length &&
      scoped.isCurrent?.() !== false
    ) {
      stageReviewedPack(packCandidate, 'matching');
      return;
    }
    void recheckStaged(packCandidate);
  }
  async function acceptPack(mode: 'matching' | 'current'): Promise<void> {
    if (!packCandidate || !reviewedStaged || checkingStaged || acceptingPack) return;
    acceptingPack = true;
    const candidate = packCandidate;
    try {
      const caps = await candidate.api.get<Schema['CapabilitiesDTO']>('/v1/capabilities');
      if (!caps.contentCatalog?.creationPackSetup)
        throw new Error('Update this agent before selecting a catalog pack setup.');
      const loader = candidate.inspection.loaderName?.toLowerCase();
      if (!['fabric', 'forge', 'neoforge'].includes(loader ?? ''))
        throw new Error('This pack does not declare a supported MSC loader.');
      if (!candidate.inspection.minecraftVersion || !candidate.inspection.loaderVersion)
        throw new Error('The pack must declare an exact Minecraft and loader version.');
      if (mode === 'current' && !resolvedMcVersion)
        throw new Error('The current Minecraft version could not be resolved.');
      if (mode === 'current' && draft.javaFlavor !== 'fabric' && !resolvedLoaderVersion)
        throw new Error('Select an exact loader version in Configure before trying this setup.');
      const versions = await candidate.api.get<Schema['VersionsResponseDTO']>(
        versionsForCreatePath(
          'java',
          mode === 'matching' ? (loader as typeof draft.javaFlavor) : draft.javaFlavor,
        ),
      );
      const mc = mode === 'matching' ? candidate.inspection.minecraftVersion : resolvedMcVersion;
      const entry = versions.versions.find(
        (value) =>
          value.mcVersion === mc &&
          (mode === 'current' ||
            !value.loaderVersion ||
            value.loaderVersion === candidate.inspection.loaderVersion),
      );
      if (!entry)
        throw new Error('The selected setup is outside the supported Minecraft/loader catalog.');
      if (disposed || packCandidate !== candidate || candidate.api.isCurrent?.() === false)
        throw new Error('The selected host or pack changed.');
      for (const id of omitted) {
        const item = draft.pendingAddOns.find((value) => value.id === id);
        if (item?.kind === 'localFile') await discardWizardInput(draft, item.stagedUploadId);
        draft.pendingAddOns = draft.pendingAddOns.filter((value) => value.id !== id);
      }
      if (mode === 'matching' && !packMatchesCurrentSetup(candidate.inspection)) {
        draft.javaFlavor = loader as typeof draft.javaFlavor;
        draft.javaCategory = 'modded';
        draft.versionId = entry.id;
        draft.javaPath = undefined;
        draft.javaSelectionKey = undefined;
        resolvedMcVersion = mc;
        resolvedLoaderVersion = candidate.inspection.loaderVersion;
      }
      stageReviewedPack(candidate, mode);
    } catch (cause) {
      stageError = errorMessage(cause);
    } finally {
      acceptingPack = false;
    }
  }
  async function discardCandidate(): Promise<void> {
    if (!packCandidate) return;
    try {
      await discardWizardInput(draft, packCandidate.id);
      packCandidate = undefined;
    } catch (cause) {
      stageError = errorMessage(cause);
    }
  }
  /**
   * The oracle's own `resolvedStagingMCVersion`: the Minecraft version add-on
   * compatibility is checked against here. `draft.versionId` only carries a
   * version-entry id (not the raw `mcVersion` string `ServerMinecraftVersion`
   * search filtering and `ProjectDetailSheet`'s compatibility badge need),
   * and Configure's own "Download latest" default leaves it unset entirely
   * -- so, matching the oracle's identical `step5AddOns.task` fetch, this
   * step re-resolves versions for the chosen flavor on its own rather than
   * threading more state through Configure.
   */
  let resolvedMcVersion: string | undefined;

  onMount(async () => {
    if (!api || draft.serverType !== 'java') return;
    const selectionKey = javaSelectionKey(draft);
    try {
      const [response, caps] = await Promise.all([
        api.get<Schema['VersionsResponseDTO']>(versionsForCreatePath('java', draft.javaFlavor)),
        api.get<Schema['CapabilitiesDTO']>('/v1/capabilities'),
      ]);
      if (disposed || api.isCurrent?.() === false || javaSelectionKey(draft) !== selectionKey) return;
      creationPackSetupSupported = caps.contentCatalog?.creationPackSetup === true;
      const versions = response.versions ?? [];
      const picked = draft.versionId
        ? versions.find((entry) => entry.id === draft.versionId)
        : (versions.find((entry) => entry.isLatest) ?? versions[0]);
      resolvedLoaderVersion = picked?.loaderVersion;
      resolvedMcVersion = picked?.mcVersion;
    } catch {
      resolvedMcVersion = undefined;
    }
  });

  $: addOnKind = javaAddOnKind(draft.javaFlavor);
  $: noun = addOnKind === 'plugin' ? 'plugins' : 'mods';
  $: itemNoun = addOnKind === 'plugin' ? 'plugin' : 'mod';
  $: totalStaged = draft.pendingAddOns.length + (draft.stagedModpack ? 1 : 0);
  $: hasAny = totalStaged > 0;
  $: hasStagedVoiceChat = hasStagedSimpleVoiceChat(draft);

  async function chooseModpack(): Promise<void> {
    if (!api || (!api.selectUploadSource && !api.uploadFile) || stagingModpack) return;
    stagingModpack = true;
    stageError = undefined;
    let picked: UploadSource | null = null;
    try {
      picked = await pickUploadSource(
        api,
        'modpack-archive',
        {
          label: 'Choose a modpack archive',
          extensions: ['mrpack', 'zip'],
        },
        { targetServerId: null },
        () => !disposed,
      );
      if (!picked) {
        stagingModpack = false;
        return;
      }
      pendingModpackSource = picked;
    } catch (error) {
      stageError = errorMessage(error);
      stagingModpack = false;
    }
  }

  async function finishModpackUpload(stagedUploadId: string, scoped?: ScreenApi): Promise<void> {
    if (!api || !pendingModpackSource)
      throw new Error('The selected modpack is no longer available.');
    const inspection = await mutate<Schema['ModpackInspectionResultDTO']>(
      scoped ?? sourceApi(pendingModpackSource, api),
      addonPaths.inspectPack,
      { stagedUploadId },
    );
    retainWizardUpload(draft, pendingModpackSource, stagedUploadId);
    reviewPack(
      stagedUploadId,
      pendingModpackSource.name,
      inspection,
      scoped ?? sourceApi(pendingModpackSource, api) ?? api,
    );
  }

  function closePendingModpack(): void {
    pendingModpackSource = undefined;
    stagingModpack = false;
  }

  async function removeStagedModpack(): Promise<void> {
    try {
      if (draft.stagedModpack) await discardWizardInput(draft, draft.stagedModpack.stagedUploadId);
      draft.stagedModpack = undefined;
      stageError = undefined;
    } catch (error) {
      stageError = `Agent cleanup could not be confirmed: ${errorMessage(error)}`;
    }
  }

  async function chooseOwnJar(): Promise<void> {
    if (stagingJar) return;
    stagingJar = true;
    stageError = undefined;
    try {
      pendingJarSource =
        (await pickUploadSource(
          api,
          'addon-local-file',
          { label: `Choose a ${itemNoun} .jar`, extensions: ['jar'] },
          { targetServerId: null },
          () => !disposed,
        )) ?? undefined;
    } catch (error) {
      stageError = errorMessage(error);
    } finally {
      stagingJar = false;
    }
  }
  function finishJarUpload(upload: Schema['StagedUploadCompleteResultDTO']): void {
    if (!pendingJarSource) throw new Error('The selected add-on is no longer available.');
    retainWizardUpload(draft, pendingJarSource, upload.stagedUploadId);
    draft.pendingAddOns = [
      ...draft.pendingAddOns,
      {
        id: crypto.randomUUID(),
        kind: 'localFile',
        fileName: pendingJarSource.name,
        stagedUploadId: upload.stagedUploadId,
      },
    ];
  }

  async function removePendingAddOn(id: string): Promise<void> {
    try {
      const entry = draft.pendingAddOns.find((entry) => entry.id === id);
      if (entry?.kind === 'localFile') await discardWizardInput(draft, entry.stagedUploadId);
      draft.pendingAddOns = draft.pendingAddOns.filter((entry) => entry.id !== id);
    } catch (error) {
      stageError = `Agent cleanup could not be confirmed: ${errorMessage(error)}`;
    }
  }

  function handleStage(
    item: Schema['CatalogItemDTO'],
    versionId: string | undefined,
    provider: 'modrinth' | 'curseforge' = 'modrinth',
    allowIncompatible = false,
  ): void {
    draft.pendingAddOns = [
      ...draft.pendingAddOns,
      {
        id: crypto.randomUUID(),
        kind: 'catalog',
        provider,
        allowIncompatible,
        projectId: item.projectId,
        slug: item.slug,
        title: item.title,
        description: item.description,
        author: item.author,
        iconURL: item.iconURL,
        versionId,
      },
    ];
  }
</script>

<div class="addons" use:onboardingAnchor={'ob_wizard_sheet'}>
  {#if addOnKind}
    <div class="header-row">
      <div class="intro">
        <h2>
          {hasAny
            ? `${itemNoun[0].toUpperCase()}${itemNoun.slice(1)}s (${totalStaged})`
            : `Add ${noun}?`}
        </h2>
        <p>
          {hasAny
            ? 'These install after the server folder is created. You can add more or remove any below.'
            : `Search Modrinth or CurseForge, add your own files, or skip this and add ${noun} after the server is created.`}
        </p>
      </div>
      {#if hasAny}
        <div class="header-actions">
          <Button
            size="sm"
            variant="secondary"
            disabled={stagingJar}
            onclick={() => void chooseOwnJar()}
          >
            {stagingJar ? 'Staging…' : addOnKind === 'mod' ? 'Import Mods…' : 'Import Plugins…'}
          </Button>
          {#if addOnKind === 'mod' && !draft.stagedModpack}
            <Button
              size="sm"
              variant="secondary"
              disabled={!api || stagingModpack || !!packCandidate}
              onclick={() => (showPacks = true)}>Browse Modpacks…</Button
            >
            <Button
              size="sm"
              variant="secondary"
              disabled={stagingModpack}
              onclick={() => void chooseModpack()}
            >
              {stagingModpack ? 'Staging…' : 'Import Modpack…'}
            </Button>
          {/if}
          <Button size="sm" variant="secondary" onclick={() => (showBrowser = true)}>
            Browse {noun}
          </Button>
        </div>
      {/if}
    </div>

    {#if !hasAny}
      <div class="cards">
        <button type="button" class="card" onclick={() => (showBrowser = true)}>
          <span class="card-title">Browse {addOnKind === 'mod' ? 'Mods' : 'Plugins'}</span>
          <span class="card-subtitle">Search and add {noun} by name or keyword.</span>
        </button>
        {#if addOnKind === 'mod'}
          <button
            type="button"
            class="card"
            disabled={!api || stagingModpack || !!packCandidate}
            onclick={() => (showPacks = true)}
          >
            <span class="card-title">Browse Modpacks</span>
            <span class="card-subtitle">Find a modpack on Modrinth or CurseForge.</span>
          </button>
          <button
            type="button"
            class="card"
            disabled={stagingModpack}
            onclick={() => void chooseModpack()}
          >
            <span class="card-title">{stagingModpack ? 'Staging…' : 'Import Modpack'}</span>
            <span class="card-subtitle">Import a .mrpack or CurseForge .zip modpack archive.</span>
          </button>
        {/if}
        <button
          type="button"
          class="card"
          disabled={stagingJar}
          onclick={() => void chooseOwnJar()}
        >
          <span class="card-title"
            >{stagingJar
              ? 'Staging…'
              : addOnKind === 'mod'
                ? 'Import Mods'
                : 'Import Plugins'}</span
          >
          <span class="card-subtitle">Already have a {itemNoun} file? Add its .jar directly.</span>
        </button>
      </div>
      {#if stageError}
        <p class="hint warn">{stageError}</p>
      {/if}
    {:else}
      {#if stageError}
        <p class="hint warn">{stageError}</p>
      {/if}

      {#if draft.stagedModpack}
        {@const inspection = draft.stagedModpack.inspection}
        <div class="summary">
          <div class="row">
            <span class="label">Pack</span>
            <span class="value">{inspection.packName ?? draft.stagedModpack.fileName}</span>
          </div>
          {#if inspection.packVersion}
            <div class="row">
              <span class="label">Version</span>
              <span class="value">{inspection.packVersion}</span>
            </div>
          {/if}
          <div class="row">
            <span class="label">Minecraft</span>
            <span class="value"
              >{inspection.minecraftVersion ?? 'Not reported'}{inspection.loaderName
                ? ` · ${inspection.loaderName}`
                : ''}</span
            >
          </div>
          <div class="row">
            <span class="label">Files</span>
            <span class="value">{inspection.fileCount}</span>
          </div>
          {#if inspection.manualFiles.length > 0}
            <div class="row">
              <span class="label">Manual downloads</span>
              <span class="value warn">{inspection.manualFiles.length} blocked by author</span>
            </div>
          {/if}
          {#if inspection.warnings && inspection.warnings.length > 0}
            <ul class="warnings">
              {#each inspection.warnings as warning}
                <li>{warning}</li>
              {/each}
            </ul>
          {/if}
          <div class="actions">
            <Button variant="secondary" onclick={removeStagedModpack}>Remove</Button>
          </div>
        </div>
      {/if}

      {#if draft.pendingAddOns.length > 0}
        <div class="list">
          {#each draft.pendingAddOns as addOn (addOn.id)}
            <div class="item-row">
              <div class="icon">
                {#if addOn.kind === 'catalog' && addOn.iconURL}
                  <img src={addOn.iconURL} alt="" width="32" height="32" loading="lazy" />
                {/if}
              </div>
              <div class="item-info">
                <span class="item-title"
                  >{addOn.kind === 'catalog' ? addOn.title : addOn.fileName}</span
                >
                {#if addOn.kind === 'catalog' && addOn.description}
                  <span class="item-subtitle">{addOn.description}</span>
                {/if}
              </div>
              <Button size="sm" variant="secondary" onclick={() => removePendingAddOn(addOn.id)}>
                Remove
              </Button>
            </div>
          {/each}
        </div>
      {/if}
      {#if hasStagedVoiceChat}
        <p class="hint voice-hint">
          Simple Voice Chat is staged. When Playit is selected, MSC will add its voice tunnel after
          this add-on is installed.
        </p>
      {/if}
    {/if}
  {/if}
</div>

{#if pendingModpackSource}
  <StagedUploadSheet
    {api}
    purpose="modpack-archive"
    completionStage="inspecting"
    source={pendingModpackSource}
    onComplete={(upload, scoped) => finishModpackUpload(upload.stagedUploadId, scoped)}
    onClose={closePendingModpack}
  />
{/if}

{#if showBrowser && addOnKind}
  <PluginBrowserSheet
    {api}
    addOnKind={addOnKind === 'plugin' ? 'plugin' : 'mod'}
    javaFlavor={draft.javaFlavor}
    serverMinecraftVersion={resolvedMcVersion}
    mode="stage"
    onStage={handleStage}
    onClose={() => (showBrowser = false)}
    onInstalled={() => {}}
  />
{/if}

{#if pendingJarSource}
  <StagedUploadSheet
    {api}
    purpose="addon-local-file"
    source={pendingJarSource}
    onComplete={finishJarUpload}
    onClose={() => (pendingJarSource = undefined)}
  />
{/if}

{#if showPacks && api}
  <PackCatalogSheet
    {api}
    javaFlavor={draft.javaFlavor}
    minecraftVersion={resolvedMcVersion}
    onClose={() => (showPacks = false)}
    onSelected={reviewPack}
  />
{/if}
{#if packCandidate}
  <Sheet
    title="Review modpack setup"
    size="lg"
    onClose={acceptingPack ? undefined : () => void discardCandidate()}
  >
    <div class="pack-setup">
      <h2>{packCandidate.inspection.packName ?? packCandidate.name}</h2>
      <p>
        Pack: Minecraft {packCandidate.inspection.minecraftVersion ?? 'unknown'} · {packCandidate
          .inspection.loaderName ?? 'unknown'} · loader {packCandidate.inspection.loaderVersion ??
          'unknown'}
      </p>
      <p>
        Current: Minecraft {resolvedMcVersion ?? 'unknown'} · {draft.javaFlavor} · loader {resolvedLoaderVersion ??
          'unknown'}.
        {#if matchingPackSetup}The pack matches your setup. Review the staged content below before
          adding it.
        {:else}Matching setup updates Minecraft and loader automatically. Trying the current setup
          may leave incompatible content and does not prove the pack will run.{/if}
      </p>
      {#each packCandidate.inspection.warnings ?? [] as warning}<p class="hint warn">
          {warning}
        </p>{/each}
      {#if draft.pendingAddOns.length}
        <h3>Staged individual content</h3>
        <p>
          These selections were made before this pack. Uncheck items to remove them; retained items
          are checked again when installed. Local files have no proven provider compatibility.
        </p>
        {#each draft.pendingAddOns as item (item.id)}
          <label
            ><input
              type="checkbox"
              checked={!omitted.has(item.id)}
              onchange={(event) => {
                const next = new Set(omitted);
                if (event.currentTarget.checked) next.delete(item.id);
                else next.add(item.id);
                omitted = next;
              }}
            />{item.kind === 'catalog' ? item.title : item.fileName} — {stagedCompatibility[
              item.id
            ] ?? 'Checking compatibility…'}</label
          >
        {/each}
        <label
          ><input type="checkbox" bind:checked={reviewedStaged} />I reviewed which individual items
          to keep with this pack.</label
        >
      {/if}
      {#if stageError}<p class="hint warn">{stageError}</p>{/if}
      <div class="setup-actions">
        <Button
          disabled={!reviewedStaged || checkingStaged || acceptingPack}
          onclick={() => void acceptPack('matching')}
          >{matchingPackSetup ? 'Add modpack' : 'Use matching setup'}</Button
        >
        {#if !matchingPackSetup}<Button
            disabled={!reviewedStaged || checkingStaged || acceptingPack || !resolvedMcVersion}
            onclick={() => void acceptPack('current')}>Try current setup anyway</Button
          >{/if}
      </div>
    </div>
  </Sheet>
{/if}

<style>
  .pack-setup {
    display: flex;
    flex-direction: column;
    gap: 12px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--msc2-text-secondary);
  }
  .pack-setup h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .pack-setup h3 {
    margin: 4px 0 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .pack-setup p {
    margin: 0;
  }
  .pack-setup label {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .pack-setup input[type='checkbox'] {
    flex-shrink: 0;
    margin: 3px 0 0;
  }
  .setup-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 4px;
  }

  .addons {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .header-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
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
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }

  .header-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
    padding-top: 2px;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
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
  .card:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .card-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .card-subtitle {
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--msc2-text-tertiary);
  }

  .summary {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--msc2-tier-chrome);
    border-radius: 10px;
    padding: 12px 14px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .label {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .value {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .value.warn {
    color: var(--msc2-status-warn);
  }
  .warnings {
    margin: 0;
    padding-left: 18px;
    font-size: 12px;
    color: var(--msc2-status-warn);
    line-height: 1.6;
  }
  .actions {
    display: flex;
    gap: 8px;
    padding-top: 4px;
  }

  .list {
    display: flex;
    flex-direction: column;
    background: var(--msc2-tier-chrome);
    border-radius: 10px;
    overflow: hidden;
  }
  .item-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .item-row:first-child {
    border-top: none;
  }
  .icon {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    border-radius: 7px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.06);
  }
  .icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .item-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item-subtitle {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint {
    margin: 0;
    font-size: 11.5px;
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
