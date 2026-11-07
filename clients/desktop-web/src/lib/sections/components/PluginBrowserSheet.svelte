<script lang="ts">
  // Ports the browsing half of ModrinthBrowserView.swift: search the active
  // server's add-on catalog and install a result straight from the flat list,
  // or click into ProjectDetailSheet (P12.7d) for the full ModrinthProjectDetailView
  // experience -- gallery, About text, and a per-version compatibility/install list.
  //
  // `mode="stage"` (AddOnsStep.svelte, P12.18f) reuses this same component the
  // way the oracle's own ModrinthBrowserView is reused for wizard-time staging
  // (AddServerWizardView.swift's `onAddToStaging` sheet, `wizardStagingConfig`
  // carrying the flavor being configured): search resolves against an explicit
  // javaFlavor/minecraftVersion instead of the active server (there isn't one
  // yet), and "Add" stages the pick through `onStage` instead of calling
  // POST /v1/components/install, which also requires an active server.
  //
  // The per-result icon here is real Modrinth artwork (CatalogItemDTO
  // .iconURL), the same category of content as a world slot's thumbnail --
  // not the rule #6 tell (a generic icon in a same-hue tinted box standing
  // in for an informational element). A flat neutral placeholder tile
  // covers projects with no icon, same shape as ModrinthBrowserView's own
  // `Color.secondary.opacity(0.15)` fallback.
  import ProviderCatalogBrowser from './ProviderCatalogBrowser.svelte';
  import CatalogBrowseSheet from './CatalogBrowseSheet.svelte';
  import CatalogBrowseContent from './CatalogBrowseContent.svelte';
  import ProjectDetailSheet from './ProjectDetailSheet.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { addonPaths, pollOperation } from './model';

  export let serverId = '';
  export let serverName = '';
  export let api: ScreenApi | undefined = undefined;
  export let addOnKind: 'mod' | 'plugin' = 'plugin';
  export let javaFlavor: string | undefined = undefined;
  export let serverMinecraftVersion: string | undefined = undefined;
  export let onClose: () => void;
  /** Called after an install so the owning screen can re-run its SVC/Playit
   * detection against the refreshed add-on inventory. */
  export let onInstalled: () => void;
  /**
   * 'install' (default): every caller today -- ComponentsSection's real
   * add-on browser for the active server. 'stage': the Add Server wizard's
   * AddOnsStep.svelte, browsing for a flavor with no server created yet.
   * In stage mode, javaFlavor/serverMinecraftVersion (already props above,
   * previously display-only) also drive the search itself via GET
   * /v1/catalog/search's javaFlavor/minecraftVersion query params, and
   * "Add" stages the pick through onStage instead of calling
   * POST /v1/components/install -- that route (like search without these
   * params) hard-requires an already-active server, which is exactly what
   * doesn't exist yet during the wizard. See AddOnsStep.svelte's own note
   * for the full finding.
   */
  export let mode: 'install' | 'stage' = 'install';
  export let onStage:
    | ((
        item: Schema['CatalogItemDTO'],
        versionId: string | undefined,
        provider?: 'modrinth' | 'curseforge',
        allowIncompatible?: boolean,
      ) => void)
    | undefined = undefined;

  export let initialProvider: 'modrinth' | 'curseforge' = 'modrinth';
  let provider: 'modrinth' | 'curseforge' = initialProvider;
  let detailItem: Schema['CatalogItemDTO'] | undefined;

  const noun = addOnKind === 'mod' ? 'mods' : 'plugins';

  let query = '';
  let results: Schema['CatalogItemDTO'][] = [];
  let supportsAddons = true;
  let subtitle = '';
  let note: string | undefined;
  let loading = false;
  let installing: Set<string> = new Set();
  let installed: Set<string> = new Set();
  let notice = '';
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  async function search(): Promise<void> {
    if (!api) return;
    loading = true;
    try {
      let path = `${addonPaths.search}${encodeURIComponent(query)}`;
      if (mode === 'stage' && javaFlavor) {
        path += `&javaFlavor=${encodeURIComponent(javaFlavor)}`;
        if (serverMinecraftVersion) {
          path += `&minecraftVersion=${encodeURIComponent(serverMinecraftVersion)}`;
        }
      }
      const response = await api.get<Schema['CatalogSearchResponseDTO']>(path);
      supportsAddons = response.supportsAddons;
      results = response.results ?? [];
      note = response.note;
      subtitle = [response.loaderName, response.gameVersion].filter(Boolean).join(' · ');
    } catch (error) {
      note = error instanceof Error ? error.message : 'Could not reach the add-on catalog.';
      results = [];
    } finally {
      loading = false;
    }
  }

  function scheduleSearch(): void {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void search(), 350);
  }

  // Field.svelte only binds `value` (no change-event prop), so the search is
  // driven off the reactive dependency on `query` -- fires once immediately
  // (debounced 350ms, same as the initial load) and again on every edit.
  $: {
    query;
    scheduleSearch();
  }

  async function install(item: Schema['CatalogItemDTO']): Promise<void> {
    if (mode === 'stage') {
      onStage?.(item, undefined);
      installed = new Set(installed).add(item.projectId);
      return;
    }
    if (!api) return;
    installing = new Set(installing).add(item.projectId);
    try {
      const result = await mutate<Schema['CatalogInstallResultDTO']>(api, addonPaths.install, {
        projectId: item.projectId,
        slug: item.slug,
        title: item.title,
      });
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId);
        notice =
          operation?.state === 'succeeded'
            ? `${result.message} — restart the server to apply.`
            : (operation?.error?.message ?? result.message);
      } else {
        notice = result.message;
      }
      installed = new Set(installed).add(item.projectId);
      onInstalled();
    } catch (error) {
      notice = error instanceof Error ? error.message : `Failed to install ${item.title}.`;
    } finally {
      const next = new Set(installing);
      next.delete(item.projectId);
      installing = next;
    }
  }

  function handleDetailInstalled(projectId: string): void {
    installed = new Set(installed).add(projectId);
    onInstalled();
  }

  function handleDetailStaged(versionId: string): void {
    if (!detailItem) return;
    onStage?.(detailItem, versionId);
    installed = new Set(installed).add(detailItem.projectId);
  }
</script>

<CatalogBrowseSheet
  title={`Browse ${noun}`}
  bind:provider
  {onClose}
  onProviderChanged={() => (detailItem = undefined)}
>
  {#if provider === 'curseforge'}
    <ProviderCatalogBrowser
      {api}
      {mode}
      onStage={(item, versionId, allowIncompatible) =>
        onStage?.(item, versionId, 'curseforge', allowIncompatible)}
      kind={addOnKind}
      {javaFlavor}
      minecraftVersion={serverMinecraftVersion}
      {serverId}
      {serverName}
      {onInstalled}
    />
  {:else}
    <CatalogBrowseContent
      bind:query
      {noun}
      subtitle={`Modrinth${subtitle ? ` · ${subtitle}` : ''}`}
      {results}
      {loading}
      {notice}
      emptyTitle={!supportsAddons ? `This server doesn't accept ${noun}` : (note ?? '')}
      {installed}
      working={installing}
      onDetail={(item) => (detailItem = item)}
      onAdd={(item) => void install(item)}
    />
  {/if}
</CatalogBrowseSheet>

{#if detailItem && provider === 'modrinth'}
  <ProjectDetailSheet
    {api}
    item={detailItem}
    {javaFlavor}
    {serverMinecraftVersion}
    {mode}
    onClose={() => (detailItem = undefined)}
    onInstalled={handleDetailInstalled}
    onStaged={handleDetailStaged}
  />
{/if}
