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
  // step dropped Browse Modrinth entirely, because GET /v1/catalog/search and
  // POST /v1/components/install both hard-required an already-active server
  // (crates/msc-agent/src/routes/components.rs), which doesn't exist yet
  // during the wizard. Cameron asked for real parity with MSC 1 here --
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
  // file at a time on both Tauri and the browser fallback, unlike the
  // oracle's native panel (`allowsMultipleSelection`) or its folder-of-jars
  // sniffing in `processModpackURL`. "Add your own .jar" here is a repeatable
  // single-file pick instead -- click it again to add another -- rather than
  // a batch folder/zip-of-loose-jars import, since MSC 2's contract has no
  // route to unpack an arbitrary zip of jars server-side (only a structured
  // mrpack/CurseForge manifest, via the modpack path below).
  import { onMount } from 'svelte';
  import Button from '../../../components/base/Button.svelte';
  import { getPlatform } from '../../../platform';
  import type { PickedFile } from '../../../platform/types';
  import type { Schema, ScreenApi } from '../../shared/types';
  import { errorMessage, mutate } from '../../shared/types';
  import { addonPaths } from '../../addons/model';
  import PluginBrowserSheet from '../../components/PluginBrowserSheet.svelte';
  import { javaAddOnKind, versionsForCreatePath, type WizardDraft } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;

  let modpackFileInput: HTMLInputElement;
  let jarFileInput: HTMLInputElement;
  let stagingModpack = false;
  let stagingJar = false;
  let stageError: string | undefined;
  let showBrowser = false;
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
    try {
      const response = await api.get<Schema['VersionsResponseDTO']>(
        versionsForCreatePath('java', draft.javaFlavor),
      );
      const versions = response.versions ?? [];
      const picked = draft.versionId
        ? versions.find((entry) => entry.id === draft.versionId)
        : undefined;
      resolvedMcVersion =
        picked?.mcVersion ??
        versions.find((entry) => entry.isLatest)?.mcVersion ??
        versions[0]?.mcVersion;
    } catch {
      resolvedMcVersion = undefined;
    }
  });

  $: addOnKind = javaAddOnKind(draft.javaFlavor);
  $: noun = addOnKind === 'plugin' ? 'plugins' : 'mods';
  $: itemNoun = addOnKind === 'plugin' ? 'plugin' : 'mod';
  $: totalStaged = draft.pendingAddOns.length + (draft.stagedModpack ? 1 : 0);
  $: hasAny = totalStaged > 0;

  function browseBrowserFile(input: HTMLInputElement): Promise<PickedFile | null> {
    return new Promise((resolve) => {
      input.addEventListener(
        'change',
        async () => {
          const browserFile = input.files?.[0];
          resolve(
            browserFile
              ? { name: browserFile.name, bytes: new Uint8Array(await browserFile.arrayBuffer()) }
              : null,
          );
        },
        { once: true },
      );
      input.click();
    });
  }

  async function chooseModpack(): Promise<void> {
    if (!api?.upload || stagingModpack) return;
    stagingModpack = true;
    stageError = undefined;
    try {
      const picked = await (
        await getPlatform()
      ).pickFile({ label: 'Choose a modpack archive', extensions: ['mrpack', 'zip'] }, () =>
        browseBrowserFile(modpackFileInput),
      );
      if (!picked) return;
      const staged = await api.upload('modpack-archive', picked.bytes);
      const inspection = await mutate<Schema['ModpackInspectionResultDTO']>(
        api,
        addonPaths.inspectPack,
        { stagedUploadId: staged.stagedUploadId },
      );
      draft.stagedModpack = {
        fileName: picked.name,
        stagedUploadId: staged.stagedUploadId,
        inspection,
      };
    } catch (error) {
      stageError = errorMessage(error);
    } finally {
      stagingModpack = false;
    }
  }

  function removeStagedModpack(): void {
    draft.stagedModpack = undefined;
    stageError = undefined;
  }

  async function chooseOwnJar(): Promise<void> {
    if (!api?.upload || stagingJar) return;
    stagingJar = true;
    stageError = undefined;
    try {
      const picked = await (
        await getPlatform()
      ).pickFile({ label: `Choose a ${itemNoun} .jar`, extensions: ['jar'] }, () =>
        browseBrowserFile(jarFileInput),
      );
      if (!picked) return;
      const staged = await api.upload('addon-local-file', picked.bytes);
      draft.pendingAddOns = [
        ...draft.pendingAddOns,
        {
          id: crypto.randomUUID(),
          kind: 'localFile',
          fileName: picked.name,
          stagedUploadId: staged.stagedUploadId,
        },
      ];
    } catch (error) {
      stageError = errorMessage(error);
    } finally {
      stagingJar = false;
    }
  }

  function removePendingAddOn(id: string): void {
    draft.pendingAddOns = draft.pendingAddOns.filter((entry) => entry.id !== id);
  }

  function handleStage(item: Schema['CatalogItemDTO'], versionId: string | undefined): void {
    draft.pendingAddOns = [
      ...draft.pendingAddOns,
      {
        id: crypto.randomUUID(),
        kind: 'catalog',
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

<div class="addons">
  <input bind:this={modpackFileInput} type="file" accept=".mrpack,.zip" class="hidden-input" />
  <input bind:this={jarFileInput} type="file" accept=".jar" class="hidden-input" />

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
            : `Search Modrinth, add your own files, or skip this and add ${noun} after the server is created.`}
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
            {stagingJar ? 'Staging…' : 'Add your own…'}
          </Button>
          {#if addOnKind === 'mod' && !draft.stagedModpack}
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
            Browse Modrinth
          </Button>
        </div>
      {/if}
    </div>

    {#if !hasAny}
      <div class="cards">
        <button type="button" class="card" onclick={() => (showBrowser = true)}>
          <span class="card-title">Browse Modrinth</span>
          <span class="card-subtitle">Search and add {noun} by name or keyword.</span>
        </button>
        {#if addOnKind === 'mod'}
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
          <span class="card-title">{stagingJar ? 'Staging…' : 'Add Your Own'}</span>
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
    {/if}
  {/if}
</div>

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

<style>
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
    display: flex;
    gap: 10px;
  }
  .cards > .card {
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
