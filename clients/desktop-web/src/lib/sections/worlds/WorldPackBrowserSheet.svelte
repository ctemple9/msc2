<script lang="ts">
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';
  import { formatCount } from '../components/model';
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
  let installed = new Set<string>();
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
      installed = new Set(installed).add(item.projectId);
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
      installed = new Set(installed).add(item.projectId);
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
  <div class="header">
    <Field
      bind:value={query}
      placeholder={bedrock ? 'Search behavior packs…' : 'Search datapacks…'}
    />
    <p class="subtitle">
      {bedrock ? 'CurseForge' : 'Modrinth'}{minecraftVersion
        ? ` · Minecraft ${minecraftVersion}`
        : ''}
    </p>
  </div>
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  {#if loading && (bedrock ? bedrockResults.length === 0 : results.length === 0)}
    <p class="explain" role="status">Searching…</p>
  {:else if bedrock && bedrockResults.length === 0}
    <EmptyState title="No behavior packs found" message="Try a different search term.">
      <Icon name="box" size={26} slot="icon" />
    </EmptyState>
  {:else if !bedrock && results.length === 0}
    <EmptyState title="No datapacks found" message="Try a different search term.">
      <Icon name="box" size={26} slot="icon" />
    </EmptyState>
  {:else if bedrock}
    <div class="results">
      {#each bedrockResults as item (item.fileId)}
        <div class="result">
          <div class="result-link">
            <div class="icon">
              {#if item.iconURL}
                <img src={item.iconURL} alt="" width="40" height="40" loading="lazy" />
              {:else}
                <Icon name="box" size={18} />
              {/if}
            </div>
            <div class="info">
              <span class="title">{item.title}</span>
              <p class="meta">{item.fileName} · Minecraft {item.minecraftVersion}</p>
              <p class="description">{item.description}</p>
            </div>
          </div>
          {#if installed.has(item.projectId)}
            <span class="added">Added</span>
          {:else if installing === item.projectId}
            <span class="added">Installing…</span>
          {:else}
            <Button size="sm" variant="secondary" onclick={() => void installBedrock(item)}
              >Add</Button
            >
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <div class="results">
      {#each results as item (item.projectId)}
        <div class="result">
          <div class="result-link">
            <div class="icon">
              {#if item.iconURL}
                <img src={item.iconURL} alt="" width="40" height="40" loading="lazy" />
              {:else}
                <Icon name="box" size={18} />
              {/if}
            </div>
            <div class="info">
              <span class="title">{item.title}</span>
              <p class="meta">by {item.author} · {formatCount(item.downloads)} downloads</p>
              <p class="description">{item.description}</p>
            </div>
          </div>
          {#if installed.has(item.projectId)}
            <span class="added">Added</span>
          {:else if installing === item.projectId}
            <span class="added">Installing…</span>
          {:else}
            <Button size="sm" variant="secondary" onclick={() => void installJava(item)}>Add</Button
            >
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</Sheet>

<style>
  .header {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }
  .subtitle {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .notice {
    margin: 0 0 10px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .explain {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 480px;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .results::-webkit-scrollbar {
    display: none;
    width: 0;
  }
  .result {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .result:first-child {
    border-top: none;
  }
  .result-link {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    min-width: 0;
    flex: 1;
  }
  .info {
    min-width: 0;
    flex: 1;
  }
  .icon {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    border-radius: 8px;
    overflow: hidden;
    background: var(--msc2-tier-chrome);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--msc2-text-tertiary);
  }
  .icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .meta {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .description {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .added {
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-status-ok);
  }
</style>
