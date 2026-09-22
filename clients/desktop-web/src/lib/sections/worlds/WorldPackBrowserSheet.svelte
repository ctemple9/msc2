<script lang="ts">
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import Card from '../../components/base/Card.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';
  import { pollOperation } from './model';

  export let api: ScreenApi | undefined;
  export let bedrock = false;
  export let slotId: string;
  export let minecraftVersion = '';
  export let onClose: () => void;
  export let onInstalled: () => void;

  let query = '';
  let results: Schema['CatalogItemDTO'][] = [];
  let bedrockResults: Schema['BedrockBehaviorPackCatalogItemDTO'][] = [];
  let loading = false;
  let installing = '';
  let notice = '';
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function search(): Promise<void> {
    if (!api) return;
    loading = true;
    notice = '';
    try {
      const params = new URLSearchParams();
      if (query.trim()) params.set('q', query.trim());
      if (minecraftVersion.trim()) params.set('gameVersion', minecraftVersion.trim());
      const path = `${bedrock ? '/v1/catalog/behaviorpacks' : '/v1/catalog/datapacks'}?${params}`;
      if (bedrock) {
        const response = await api.get<Schema['BedrockBehaviorPackSearchResponseDTO']>(path);
        bedrockResults = response.results;
      } else {
        const response = await api.get<Schema['CatalogSearchResponseDTO']>(path);
        results = response.results ?? [];
      }
    } catch (error) {
      results = [];
      bedrockResults = [];
      notice = errorMessage(error);
    } finally {
      loading = false;
    }
  }

  function scheduleSearch(): void {
    clearTimeout(timer);
    timer = setTimeout(() => void search(), 350);
  }

  async function installJava(item: Schema['CatalogItemDTO']): Promise<void> {
    if (!api) return;
    installing = item.projectId;
    notice = '';
    try {
      const versions = await api.get<Schema['CatalogVersionsResponseDTO']>(
        `/v1/catalog/projects/${encodeURIComponent(item.projectId)}/versions`,
      );
      const version =
        (minecraftVersion
          ? versions.versions.find((candidate) => candidate.gameVersions.includes(minecraftVersion))
          : undefined) ?? versions.versions[0];
      if (!version) throw new Error('No published datapack version is available.');
      const result = await mutate<Schema['JavaDatapackInstallResultDTO']>(
        api,
        `/v1/worlds/${encodeURIComponent(slotId)}/datapacks/install`,
        { projectId: item.projectId, versionId: version.id },
      );
      notice = `${result.pack.name} installed.`;
      onInstalled();
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      installing = '';
    }
  }

  async function installBedrock(item: Schema['BedrockBehaviorPackCatalogItemDTO']): Promise<void> {
    if (!api) return;
    installing = item.projectId;
    notice = '';
    try {
      const result = await mutate<Schema['BedrockBehaviorPackInstallResultDTO']>(
        api,
        `/v1/worlds/${encodeURIComponent(slotId)}/behaviorpacks/install`,
        { projectId: item.projectId, fileId: item.fileId },
      );
      const operation = await pollOperation(api, result.operationId);
      if (operation?.state !== 'succeeded') {
        throw new Error(
          operation?.error?.message ?? 'The behavior pack installation did not complete.',
        );
      }
      notice = `${item.title} installed.`;
      onInstalled();
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      installing = '';
    }
  }

  $: {
    query;
    scheduleSearch();
  }
</script>

<Sheet title={bedrock ? 'Browse Behavior Packs' : 'Browse Datapacks'} size="lg" {onClose}>
  <div class="browser">
    <p class="intro">
      {bedrock
        ? 'Search CurseForge Bedrock add-ons. The host agent uses its configured API key.'
        : 'Search Modrinth Java datapacks. Select a result to install its latest published version.'}
      {#if minecraftVersion}
        Minecraft {minecraftVersion}.{/if}
    </p>
    <Field
      bind:value={query}
      placeholder={bedrock ? 'Search behavior packs' : 'Search datapacks'}
    />
    {#if notice}<p class="notice" role="status">{notice}</p>{/if}
    {#if loading}
      <p class="quiet" role="status">Searching…</p>
    {:else if bedrock}
      {#each bedrockResults as item (item.fileId)}
        <Card padding="12px 14px">
          <div class="result">
            <div class="info">
              <span class="name">{item.title}</span>
              <span class="quiet">{item.fileName} · Minecraft {item.minecraftVersion}</span>
              <span class="quiet">{item.description}</span>
            </div>
            <Button
              size="sm"
              variant="secondary"
              disabled={!!installing}
              onclick={() => void installBedrock(item)}
            >
              {installing === item.projectId ? 'Installing…' : 'Install'}
            </Button>
          </div>
        </Card>
      {:else}
        <p class="quiet">No behavior packs found. Try another search.</p>
      {/each}
    {:else}
      {#each results as item (item.projectId)}
        <Card padding="12px 14px">
          <div class="result">
            <div class="info">
              <span class="name">{item.title}</span>
              <span class="quiet">{item.author} · {item.downloads.toLocaleString()} downloads</span>
              <span class="quiet">{item.description}</span>
            </div>
            <Button
              size="sm"
              variant="secondary"
              disabled={!!installing}
              onclick={() => void installJava(item)}
            >
              {installing === item.projectId ? 'Installing…' : 'Install latest'}
            </Button>
          </div>
        </Card>
      {:else}
        <p class="quiet">No datapacks found. Try another search.</p>
      {/each}
    {/if}
  </div>
</Sheet>

<style>
  .browser {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .intro,
  .notice,
  .quiet {
    margin: 0;
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }
  .notice {
    color: var(--msc2-status-warn);
  }
  .result {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .name {
    color: var(--msc2-text-primary);
    font-size: 13px;
    font-weight: 500;
  }
  .quiet {
    color: var(--msc2-text-tertiary);
    overflow-wrap: anywhere;
  }
</style>
