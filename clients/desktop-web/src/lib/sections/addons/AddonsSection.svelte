<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import PlayerContentSheet from '../components/PlayerContentSheet.svelte';
  import PluginBrowserSheet from '../components/PluginBrowserSheet.svelte';
  import ContentSourcesSheet from '../components/ContentSourcesSheet.svelte';
  import { addOnKind } from '../components/model';
  import ActionButton from '../../components/ActionButton.svelte';
  import { discardUpload } from '../shared/upload-source';
  import { reportTransferFailure } from '../shared/transfer-notices';
  import TransferPanel from '../transfers/TransferPanel.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { pollOperation } from '../worlds/model';
  import { addonPaths, demoAddons } from './model';

  export let api: ScreenProps['api'] = undefined;
  export let serverId = '';
  let browserOpen = false;
  let sourcesOpen = false;
  let playersOpen = false;
  let server: Schema['ServerDTO'] | undefined;
  let minecraftVersion: string | undefined;
  let addons: Schema['AddonItemDTO'][] = demoAddons;
  let pack: Schema['ModpackInspectionResultDTO'] | null = null;
  let transferApi: ScreenProps['api'];
  let stagedPack = '';
  let packAction: 'import' | 'replace' = 'import';
  let notice = '';

  onDestroy(() => {
    if (stagedPack)
      void discardUpload(transferApi, stagedPack).catch((error) =>
        reportTransferFailure(transferApi, error),
      );
  });
  async function keepPack(id: string, scoped?: ScreenProps['api']): Promise<void> {
    if (stagedPack) await discardUpload(transferApi, stagedPack);
    stagedPack = id;
    transferApi = scoped;
    pack = null;
  }
  onMount(async () => {
    const result = await call<Schema['AddonsResponseDTO']>(
      api,
      { addons, isResolving: false, packManaged: false, serverSupportsAddons: true },
      addonPaths.list,
    );
    addons = result.addons;
    await refreshCatalogContext();
  });
  async function refreshCatalogContext(): Promise<void> {
    if (!api || api.isCurrent?.() === false) return;
    const scoped = api.snapshot?.() ?? api;
    try {
      const [servers, versions] = await Promise.all([
        scoped.get<Schema['ServerDTO'][]>('/v1/servers'),
        scoped.get<Schema['VersionsResponseDTO']>('/v1/versions'),
      ]);
      if (scoped.isCurrent?.() === false) return;
      server = servers.find((item) => item.id === serverId);
      minecraftVersion = versions.currentVersion ?? undefined;
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function catalogChanged(): Promise<void> {
    if (!api || api.isCurrent?.() === false) return;
    const scoped = api.snapshot?.() ?? api;
    try {
      const [result, content] = await Promise.all([
        scoped.get<Schema['AddonsResponseDTO']>(addonPaths.list),
        scoped.get<{ requirements: { state: string }[] }>(
          `/v1/servers/${encodeURIComponent(serverId)}/content/collection`,
        ),
      ]);
      if (scoped.isCurrent?.() === false) return;
      addons = result.addons;
      const count = content.requirements.filter(
        (item) => !['matched', 'serverOnly', 'optionalMissing'].includes(item.state),
      ).length;
      notice = count
        ? `${count} player content requirements need review. Open Player content.`
        : 'Content changed; player requirements checked.';
    } catch (error) {
      if (scoped.isCurrent?.() !== false) notice = errorMessage(error);
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
      pack = await mutate<Schema['ModpackInspectionResultDTO']>(
        transferApi ?? api,
        '/v1/modpacks/inspect',
        {
          stagedUploadId: stagedPack,
        },
      );
      notice = 'Modpack inspected. Review its provenance before importing.';
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function importPack(): Promise<void> {
    if (!stagedPack) return;
    try {
      const result = await mutate<Schema['ModpackImportResultDTO']>(
        transferApi ?? api,
        addonPaths.importPack,
        {
          action: packAction,
          stagedUploadId: stagedPack,
        },
      );
      stagedPack = '';
      if (!result.pendingManualFiles?.length) {
        const operation = await pollOperation(transferApi ?? api, result.operationId, (tick) => {
          notice = tick.statusLine ?? 'Importing modpack…';
        });
        if (operation?.state !== 'succeeded')
          throw new Error(operation?.error?.message ?? 'Modpack import did not complete.');
        notice = 'Modpack imported.';
      } else notice = result.message;
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
    <div class="screen-card-header"><h3>Catalog</h3></div>
    <div class="screen-actions">
      <ActionButton
        label="Browse mods or plugins"
        disabled={!server}
        onclick={() => (browserOpen = true)}>Browse Modrinth or CurseForge…</ActionButton
      >
      <ActionButton
        kind="quiet"
        label="Player content"
        disabled={!server}
        onclick={() => (playersOpen = true)}>Player content…</ActionButton
      >
      <ActionButton
        kind="quiet"
        label="Content sources and updates"
        disabled={!server}
        onclick={() => (sourcesOpen = true)}>Content sources and updates…</ActionButton
      >
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
      onComplete={keepPack}
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

{#if browserOpen && server}
  <PluginBrowserSheet
    {api}
    {serverId}
    serverName={server.name}
    addOnKind={addOnKind(server.javaFlavor)}
    javaFlavor={server.javaFlavor ?? undefined}
    serverMinecraftVersion={minecraftVersion}
    onClose={() => (browserOpen = false)}
    onInstalled={() => void catalogChanged()}
  />
{/if}
{#if sourcesOpen && api}
  <ContentSourcesSheet
    {api}
    {serverId}
    onClose={() => (sourcesOpen = false)}
    onChanged={() => void catalogChanged()}
  />
{/if}

{#if playersOpen && api && server}
  <PlayerContentSheet
    {api}
    {serverId}
    javaFlavor={server.javaFlavor ?? ''}
    minecraftVersion={minecraftVersion ?? ''}
    onClose={() => (playersOpen = false)}
    onChanged={() => void catalogChanged()}
  />
{/if}
