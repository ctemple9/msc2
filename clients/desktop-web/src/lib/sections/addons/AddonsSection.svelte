<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import TransferPanel from '../transfers/TransferPanel.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { addonPaths, demoAddons } from './model';

  export let api: ScreenProps['api'] = undefined;
  let addons: Schema['AddonItemDTO'][] = demoAddons;
  let catalog: Schema['CatalogItemDTO'][] = [];
  let query = '';
  let pack: Schema['ModpackInspectionResultDTO'] | null = null;
  let stagedPack = '';
  let packAction: 'import' | 'replace' = 'import';
  let notice = '';

  onMount(async () => {
    const result = await call<Schema['AddonsResponseDTO']>(
      api,
      { addons, isResolving: false, packManaged: false, serverSupportsAddons: true },
      addonPaths.list,
    );
    addons = result.addons;
  });
  async function search(): Promise<void> {
    const result = await call<Schema['CatalogSearchResponseDTO']>(
      api,
      { supportsAddons: true, results: [] },
      `${addonPaths.search}${encodeURIComponent(query)}`,
    );
    catalog = result.results ?? [];
  }
  async function install(item: Schema['CatalogItemDTO']): Promise<void> {
    try {
      const result = await mutate<Schema['CatalogInstallResultDTO']>(api, addonPaths.install, {
        projectId: item.projectId,
        slug: item.slug,
        title: item.title,
      });
      notice = result.message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function toggle(addon: Schema['AddonItemDTO']): Promise<void> {
    try {
      const result = await mutate<Schema['AddonUpdateResultDTO']>(api, addonPaths.update, {
        jarStem: addon.jarStem,
        enabled: !addon.isEnabled,
      });
      notice = result.result;
      addons = addons.map((item) =>
        item.jarStem === addon.jarStem ? { ...item, isEnabled: !item.isEnabled } : item,
      );
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function remove(addon: Schema['AddonItemDTO']): Promise<void> {
    try {
      const result = await mutate<Schema['AddonRemoveResultDTO']>(api, addonPaths.remove, {
        jarStem: addon.jarStem,
      });
      notice = result.message;
      addons = addons.filter((item) => item.jarStem !== addon.jarStem);
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function inspectPack(): Promise<void> {
    if (!stagedPack) return;
    try {
      pack = await mutate<Schema['ModpackInspectionResultDTO']>(api, '/v1/modpacks/inspect', {
        stagedUploadId: stagedPack,
      });
      notice = 'Modpack inspected. Review its provenance before importing.';
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function importPack(): Promise<void> {
    if (!stagedPack) return;
    try {
      const result = await mutate<Schema['ModpackImportResultDTO']>(api, addonPaths.importPack, {
        action: packAction,
        stagedUploadId: stagedPack,
      });
      notice = result.message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Mods and plugins"
    title="Add-ons"
    description="Search, install, update, toggle, remove, and inspect provenance without hardcoding provider or server-family lists."
    status={`${addons.length} installed`}
    statusTone="positive"
  />
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Installed add-ons</h3>
      <span class="metric-label">State comes from the agent</span>
    </div>
    <table class="data-table">
      <thead
        ><tr
          ><th>Name</th><th>Bucket</th><th>Version</th><th>State</th><th class="actions">Actions</th
          ></tr
        ></thead
      ><tbody
        >{#each addons as addon (addon.jarStem)}<tr
            ><td><strong>{addon.displayName}</strong></td><td>{addon.bucket}</td><td
              >{addon.currentVersion ?? 'Unknown'}{#if addon.availableVersion}<br /><small
                  >Available: {addon.availableVersion}</small
                >{/if}</td
            ><td>{addon.isEnabled ? 'Enabled' : 'Disabled'}</td><td class="actions"
              ><ActionButton kind="quiet" label="Toggle add-on" onclick={() => toggle(addon)}
                >{addon.isEnabled ? 'Disable' : 'Enable'}</ActionButton
              ><ActionButton kind="danger" label="Remove add-on" onclick={() => remove(addon)}
                >Remove</ActionButton
              ></td
            ></tr
          >{:else}<tr><td colspan="5" class="empty-row">No add-ons installed.</td></tr
          >{/each}</tbody
      >
    </table>
  </section>
  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Catalog</h3>
      <div class="inline-form">
        <div class="field">
          <label for="catalog-search">Search provider catalog</label><input
            id="catalog-search"
            bind:value={query}
            placeholder="Lithium, performance…"
          />
        </div>
        <ActionButton label="Search" onclick={search}>Search</ActionButton>
      </div>
    </div>
    <div class="screen-grid" style="margin-top: 1rem">
      {#each catalog as item (item.projectId)}<article class="screen-card">
          <h3>{item.title}</h3>
          <p>{item.description}</p>
          <small>{item.author} · {item.downloads.toLocaleString()} downloads</small>
          <div class="screen-actions" style="margin-top: .7rem">
            <ActionButton label={`Install ${item.title}`} onclick={() => install(item)}
              >Install</ActionButton
            >
          </div>
        </article>{:else}<p class="muted">
          Search only when the connected agent advertises an add-on provider.
        </p>{/each}
    </div>
  </section>
  <section class="screen-card">
    <h3>Modpack transfer</h3>
    <p>
      D-027 keeps browser download and upload explicit: inspect the staged archive, then choose
      import or whole-pack replace.
    </p>
    <TransferPanel
      {api}
      purpose="modpack-archive"
      label="Modpack archive"
      onComplete={(id) => (stagedPack = id)}
    />{#if stagedPack}<div class="screen-actions" style="margin-top: .7rem">
        <ActionButton kind="quiet" label="Inspect modpack" onclick={inspectPack}
          >Inspect</ActionButton
        ><select bind:value={packAction} aria-label="Modpack action"
          ><option value="import">Import</option><option value="replace">Replace</option></select
        ><ActionButton label="Apply modpack" disabled={!pack} onclick={importPack}
          >Apply</ActionButton
        >
      </div>{/if}{#if pack}<div class="capability-notice" style="margin-top: .7rem">
        <strong>{pack.packName ?? 'Modpack'}</strong>
        <p>
          {pack.minecraftVersion ?? 'Version not reported'} · {pack.loaderName ??
            'Loader not reported'} · {pack.fileCount} files
        </p>
      </div>{/if}
  </section>
  <CapabilityNotice
    title="Provider and pack state stay honest"
    message="Provider-unavailable, dependency, pack-managed, cancellation, and provenance messages come from the agent response and remain visible here."
  />
</div>
