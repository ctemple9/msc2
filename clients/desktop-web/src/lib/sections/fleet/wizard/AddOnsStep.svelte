<script lang="ts">
  // Real port of AddServerWizardView.swift's step5AddOns, shown only when
  // hasAddOnsStep(draft) is true (model.ts). Values only accumulate into the
  // wizard's draft state -- nothing installs yet, matching every prior
  // Configure/Network/World step's own established pattern; the real
  // POST /v1/servers/create call is P12.18g's job.
  //
  // Real gap found and handled, not silently worked around: this step's own
  // plan text called for reusing PluginBrowserSheet.svelte's Browse Modrinth
  // tile and ImportModpackSheet.svelte's Import tile as-is. Reading both
  // routes they call found neither has a pre-create mode:
  //   - GET /v1/catalog/search (search_catalog, components.rs) hard-requires
  //     `state.lifecycle.active_config_server()` and resolves the loader/game
  //     version to filter by *from that server*, not from any query param.
  //     During the wizard there is no server yet -- or worse, some unrelated
  //     server could already be active, in which case this would silently
  //     search and later install against the WRONG server's flavor/version,
  //     not merely show nothing.
  //   - POST /v1/components/install (install_component, same file) hard-
  //     requires the same active server and installs immediately -- there is
  //     no "stage this pick for later" mode for a catalog item or a loose
  //     local jar (CatalogInstallRequestDTO.stagedUploadId still redeems
  //     through this same active-server-scoped route).
  // So Browse Modrinth cannot be offered here at all, for any flavor --
  // dropped entirely rather than wired to a route that would act on the
  // wrong server. The one real pre-create primitive the frozen contract does
  // offer is `ServerCreateRequestDTO.stagedModpackUploadId`, redeemed by
  // POST /v1/servers/create itself (confirmed against
  // crates/msc-agent/src/routes/servers.rs's `redeem_modpack_upload` call) --
  // and POST /v1/modpacks/inspect (inspect_modpack) that stages a modpack
  // archive resolves entirely from the staged upload, with no active-server
  // dependency at all. So this step ports the "stage a modpack archive" half
  // of ImportModpackSheet.svelte's own flow (its `chooseAndStage`, using the
  // same `api.upload('modpack-archive', …)` + `addonPaths.inspectPack` calls)
  // -- the same "reuse the staging primitive, not the whole sheet" precedent
  // WorldStep.svelte already established for ImportWorldZipSheet.svelte --
  // and stops there instead of continuing into that sheet's own Import
  // button, which calls POST /v1/modpacks/import: a route that always acts
  // on "the active server" (its own request DTO says so explicitly) and so,
  // like Browse Modrinth, cannot be called correctly before the server this
  // wizard is building actually exists.
  //
  // This only helps mod-kind flavors (Fabric/NeoForge/Forge) -- a modpack
  // archive is what they take. Plugin-kind flavors (Paper/Purpur) have no
  // pre-create staging primitive in the contract at all: their oracle
  // equivalent imports a loose .zip/folder of .jar files directly onto disk
  // (AppViewModel+ServerCreation.swift's `applyStagedAddOn`), and MSC 2 has
  // no agent route that accepts a raw jar bundle without an active server to
  // install it into. Rather than fake a picker for a route that doesn't
  // exist, plugin-kind flavors get the oracle's own reassurance copy
  // ("add plugins after the server is created") as the real, only path --
  // not just a footnote next to working controls.
  import Button from '../../../components/base/Button.svelte';
  import EmptyState from '../../../components/base/EmptyState.svelte';
  import Icon from '../../../components/base/Icon.svelte';
  import { getPlatform } from '../../../platform';
  import type { PickedFile } from '../../../platform/types';
  import type { Schema, ScreenApi } from '../../shared/types';
  import { errorMessage, mutate } from '../../shared/types';
  import { addonPaths } from '../../addons/model';
  import { javaAddOnKind, type WizardDraft } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;

  let fileInput: HTMLInputElement;
  let staging = false;
  let stageError: string | undefined;

  $: addOnKind = javaAddOnKind(draft.javaFlavor);
  $: noun = addOnKind === 'plugin' ? 'plugins' : 'mods';

  function browseBrowserFile(): Promise<PickedFile | null> {
    return new Promise((resolve) => {
      fileInput.addEventListener(
        'change',
        async () => {
          const browserFile = fileInput.files?.[0];
          resolve(
            browserFile
              ? { name: browserFile.name, bytes: new Uint8Array(await browserFile.arrayBuffer()) }
              : null,
          );
        },
        { once: true },
      );
      fileInput.click();
    });
  }

  async function chooseModpack(): Promise<void> {
    if (!api?.upload || staging) return;
    staging = true;
    stageError = undefined;
    try {
      const picked = await (
        await getPlatform()
      ).pickFile({ label: 'Choose a modpack archive', extensions: ['mrpack', 'zip'] }, () =>
        browseBrowserFile(),
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
      staging = false;
    }
  }

  function removeStagedModpack(): void {
    draft.stagedModpack = undefined;
    stageError = undefined;
  }
</script>

<div class="addons">
  <input bind:this={fileInput} type="file" accept=".mrpack,.zip" class="hidden-input" />

  {#if addOnKind === 'mod'}
    <div class="intro">
      <h2>{draft.stagedModpack ? 'Modpack staged' : `Add ${noun}?`}</h2>
      <p>
        {draft.stagedModpack
          ? 'This pack installs after the server folder is created. You can remove it or choose a different archive.'
          : `Import a .mrpack or CurseForge .zip modpack archive. You can also skip this and add ${noun} after the server is created.`}
      </p>
    </div>

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
      </div>
      {#if inspection.warnings && inspection.warnings.length > 0}
        <ul class="warnings">
          {#each inspection.warnings as warning}
            <li>{warning}</li>
          {/each}
        </ul>
      {/if}
      <div class="actions">
        <Button variant="secondary" onclick={removeStagedModpack}>Remove</Button>
        <Button variant="secondary" disabled={staging} onclick={() => void chooseModpack()}>
          {staging ? 'Staging…' : 'Choose a different archive…'}
        </Button>
      </div>
    {:else}
      <section class="block">
        <Button variant="secondary" disabled={staging} onclick={() => void chooseModpack()}>
          {staging ? 'Staging…' : 'Import Modpack…'}
        </Button>
        {#if stageError}
          <p class="hint warn">{stageError}</p>
        {/if}
      </section>
    {/if}
  {:else}
    <EmptyState
      title={`Add ${noun} after the server is created`}
      message={`The ${noun} catalog needs a server to search against, so this happens once the server exists — right from the ${noun === 'plugins' ? 'Plugins' : 'Mods'} browser.`}
    >
      <Icon name="box" size={26} slot="icon" />
    </EmptyState>
  {/if}
</div>

<style>
  .addons {
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
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
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
