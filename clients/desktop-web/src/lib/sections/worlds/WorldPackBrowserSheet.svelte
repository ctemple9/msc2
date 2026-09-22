<script lang="ts">
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Badge from '../../components/base/Badge.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';
  import { formatCount, parseInlineMarkdown, sanitizeCurseForgeBody } from '../components/model';
  import { pollOperation } from './model';

  export let api: ScreenApi | undefined;
  export let bedrock = false;
  export let slotId: string;
  export let minecraftVersion = '';
  export let onClose: () => void;
  export let onInstalled: () => void;

  let selectedMinecraftVersion = minecraftVersion;
  let versionLoading = true;
  let query = '';
  let results: Schema['CatalogItemDTO'][] = [];
  let bedrockResults: Schema['BedrockBehaviorPackCatalogItemDTO'][] = [];
  let detailItem: Schema['BedrockBehaviorPackCatalogItemDTO'] | undefined;
  let detail: Schema['BedrockBehaviorPackDetailDTO'] | undefined;
  let detailLoading = false;
  let detailError = '';
  let stableOnly = true;
  let expandedFileIds = new Set<number>();
  let detailRequestId = 0;
  let loading = false;
  let installing = '';
  let installed = new Set<string>();
  let notice = '';
  let timer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    if (!bedrock || selectedMinecraftVersion.trim() || !api) {
      versionLoading = false;
      return;
    }
    try {
      const version = await api.get<Schema['VersionsResponseDTO']>('/v1/versions');
      selectedMinecraftVersion = version.currentVersion ?? '';
    } catch {
      // The pack list remains browseable if the selected server version is unavailable.
    } finally {
      versionLoading = false;
    }
  });

  $: visibleFiles = (detail?.files ?? []).filter((file) => !stableOnly || file.releaseType === 1);
  $: projectURL =
    detail?.sourceURL ??
    (detail?.slug
      ? `https://www.curseforge.com/minecraft-bedrock/addons/${encodeURIComponent(detail.slug)}`
      : undefined);
  $: aboutParagraphs = sanitizeCurseForgeBody(detail?.description ?? '')
    .split('\n\n')
    .filter((paragraph) => paragraph.trim().length > 0);

  async function showBedrockDetail(
    item: Schema['BedrockBehaviorPackCatalogItemDTO'],
  ): Promise<void> {
    const requestId = ++detailRequestId;
    detailItem = item;
    detail = undefined;
    detailLoading = true;
    detailError = '';
    expandedFileIds = new Set();
    stableOnly = true;
    if (!api) {
      detailLoading = false;
      detailError = 'Connect to an agent to load pack details.';
      return;
    }
    try {
      const loaded = await api.get<Schema['BedrockBehaviorPackDetailDTO']>(
        `/v1/catalog/behaviorpacks/${encodeURIComponent(item.projectId)}`,
      );
      if (requestId !== detailRequestId) return;
      detail = loaded;
      if (!loaded.files.some((file) => file.releaseType === 1)) stableOnly = false;
    } catch (error) {
      if (requestId === detailRequestId) detailError = errorMessage(error);
    } finally {
      if (requestId === detailRequestId) detailLoading = false;
    }
  }

  function closeBedrockDetail(): void {
    detailRequestId += 1;
    detailItem = undefined;
    detail = undefined;
  }

  function toggleFileDetails(fileId: number): void {
    const next = new Set(expandedFileIds);
    if (next.has(fileId)) next.delete(fileId);
    else next.add(fileId);
    expandedFileIds = next;
  }

  function fileReleaseLabel(releaseType: number): string {
    if (releaseType === 1) return 'release';
    if (releaseType === 2) return 'beta';
    return 'alpha';
  }

  function fileReleaseTone(releaseType: number): 'ok' | 'warn' | 'error' {
    if (releaseType === 1) return 'ok';
    if (releaseType === 2) return 'warn';
    return 'error';
  }

  async function search(): Promise<void> {
    if (!api) return;
    loading = true;
    notice = '';
    try {
      const params = new URLSearchParams();
      if (query.trim()) params.set('q', query.trim());
      if (!bedrock && selectedMinecraftVersion.trim()) {
        params.set('gameVersion', selectedMinecraftVersion.trim());
      }
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

  async function installBedrock(
    item: Schema['BedrockBehaviorPackCatalogItemDTO'],
    fileId = item.fileId,
  ): Promise<void> {
    if (!api) return;
    installing = item.projectId;
    notice = '';
    try {
      const result = await mutate<Schema['BedrockBehaviorPackInstallResultDTO']>(
        api,
        `/v1/worlds/${encodeURIComponent(slotId)}/behaviorpacks/install`,
        { projectId: item.projectId, fileId },
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
    selectedMinecraftVersion;
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
      {bedrock ? 'CurseForge' : 'Modrinth'}{selectedMinecraftVersion
        ? ` · Minecraft ${selectedMinecraftVersion}`
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
        {@const compatible =
          !!selectedMinecraftVersion && item.minecraftVersion === selectedMinecraftVersion}
        <div class="result">
          <button type="button" class="result-link" onclick={() => void showBedrockDetail(item)}>
            <div class="icon">
              {#if item.iconURL}
                <img src={item.iconURL} alt="" width="40" height="40" loading="lazy" />
              {:else}
                <Icon name="box" size={18} />
              {/if}
            </div>
            <div class="info">
              <span class="title-row">
                <span class="title">{item.title}</span>
                <Icon name="chevron" size={10} />
              </span>
              <p class="meta">{item.fileName} · Minecraft {item.minecraftVersion}</p>
              <p class="description">{item.description}</p>
            </div>
          </button>
          {#if installed.has(item.projectId)}
            <span class="added">Added</span>
          {:else if installing === item.projectId}
            <span class="added">Installing…</span>
          {:else if versionLoading}
            <span class="added">Checking version…</span>
          {:else if !selectedMinecraftVersion}
            <Badge variant="status" tone="warn">Version unknown</Badge>
          {:else if !compatible}
            <Badge variant="status" tone="warn">Other version</Badge>
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

{#if detailItem}
  <Sheet title={detail?.title ?? detailItem.title} size="lg" onClose={closeBedrockDetail}>
    {#if detailLoading}
      <p class="explain" role="status">Loading pack details…</p>
    {:else if detailError}
      <p class="detail-error" role="alert">{detailError}</p>
    {:else if detail}
      <div class="detail-header">
        <div class="detail-icon">
          {#if detail.iconURL}
            <img src={detail.iconURL} alt="" width="56" height="56" />
          {:else}
            <Icon name="box" size={22} />
          {/if}
        </div>
        <div class="detail-heading">
          <span class="detail-title">{detail.title}</span>
          {#if detail.author}<p class="detail-byline">by {detail.author}</p>{/if}
          <p class="detail-stats">{formatCount(detail.downloads)} downloads</p>
        </div>
      </div>

      {#if projectURL}
        <a class="provider-link" href={projectURL} target="_blank" rel="noopener noreferrer"
          >View on CurseForge</a
        >
      {/if}

      {#if selectedMinecraftVersion}
        {@const hasCompatibleFile = detail.files.some((file) =>
          file.gameVersions.includes(selectedMinecraftVersion),
        )}
        <p class="compat" class:warn={!hasCompatibleFile}>
          {hasCompatibleFile
            ? `A version is available for your server (${selectedMinecraftVersion}).`
            : `No version yet for Minecraft ${selectedMinecraftVersion}. You can still install another version below, at your own risk.`}
        </p>
      {/if}

      {#if detail.gallery.length > 0}
        <section class="detail-section">
          <h3>Gallery</h3>
          <div class="gallery">
            {#each detail.gallery as image (image.url)}
              <img src={image.url} alt={image.title ?? ''} loading="lazy" />
            {/each}
          </div>
        </section>
      {/if}

      <section class="detail-section">
        <h3>About</h3>
        {#if aboutParagraphs.length > 0}
          <div class="about">
            {#each aboutParagraphs as paragraph, index (index)}
              <p>
                {#each parseInlineMarkdown(paragraph) as segment}
                  {#if segment.type === 'bold'}<strong>{segment.text}</strong
                    >{:else if segment.type === 'link'}<a
                      href={segment.href}
                      target="_blank"
                      rel="noopener noreferrer">{segment.text}</a
                    >{:else}{segment.text}{/if}
                {/each}
              </p>
            {/each}
          </div>
        {:else}
          <p class="explain">No description provided.</p>
        {/if}
      </section>

      <section class="detail-section">
        <div class="section-header">
          <h3>Versions</h3>
          <div class="stable-toggle">
            <Toggle
              checked={stableOnly}
              label="Stable only"
              onchange={(value) => (stableOnly = value)}
            />
            <span>Stable only</span>
          </div>
        </div>
        {#if visibleFiles.length === 0}
          <p class="explain">No published files found.</p>
        {:else}
          <div class="versions">
            {#each visibleFiles.slice(0, 40) as file (file.id)}
              {@const compatible =
                !!selectedMinecraftVersion && file.gameVersions.includes(selectedMinecraftVersion)}
              {@const compatibilityLabel = !selectedMinecraftVersion
                ? 'Version unknown'
                : compatible
                  ? 'Compatible'
                  : 'Other version'}
              {@const expanded = expandedFileIds.has(file.id)}
              <div class="version-row">
                <button
                  type="button"
                  class="version-toggle"
                  onclick={() => toggleFileDetails(file.id)}
                  aria-expanded={expanded}
                >
                  <span class="chevron" class:open={expanded}
                    ><Icon name="chevron" size={11} /></span
                  >
                  <span class="version-main">
                    <span class="version-line">
                      <span class="version-name">{file.displayName}</span>
                      <Badge variant="status" tone={fileReleaseTone(file.releaseType)}>
                        {fileReleaseLabel(file.releaseType)}
                      </Badge>
                      <Badge variant="status" tone={compatible ? 'ok' : 'warn'}>
                        {compatibilityLabel}
                      </Badge>
                    </span>
                    <span class="version-mc">{file.gameVersions.slice(0, 4).join(', ')}</span>
                  </span>
                </button>
                {#if installed.has(detail.projectId)}
                  <span class="added">Added</span>
                {:else if installing === detail.projectId}
                  <span class="added">Installing…</span>
                {:else}
                  <Button
                    size="sm"
                    variant="secondary"
                    onclick={() => void installBedrock(detailItem!, file.id)}
                    >{compatible ? 'Install' : 'Install anyway'}</Button
                  >
                {/if}
              </div>
              {#if expanded}
                <div class="version-detail">
                  <p class="detail-label">Supported Minecraft versions</p>
                  <div class="version-tags">
                    {#each file.gameVersions as version (version)}
                      <span class:highlight={version === selectedMinecraftVersion} class="tag"
                        >{version}</span
                      >
                    {/each}
                  </div>
                  <p class="version-downloads">
                    {formatCount(file.downloads)} downloads · {file.fileName}
                  </p>
                </div>
              {/if}
            {/each}
          </div>
        {/if}
      </section>

      {#if notice}<p class="notice" role="status">{notice}</p>{/if}
    {/if}
  </Sheet>
{/if}

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
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--msc2-text-secondary);
  }
  .title-row .title {
    color: var(--msc2-text-primary);
  }
  .title-row :global(svg) {
    flex-shrink: 0;
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
  .detail-header {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 18px;
  }
  .detail-icon {
    flex: 0 0 56px;
    width: 56px;
    height: 56px;
    overflow: hidden;
    border-radius: 8px;
    background: var(--msc2-tier-chrome);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--msc2-text-tertiary);
  }
  .detail-icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .detail-heading {
    min-width: 0;
  }
  .detail-title {
    font-size: 15px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .detail-stats {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .detail-byline {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .provider-link {
    display: inline-flex;
    align-items: center;
    min-height: 30px;
    padding: 0 10px;
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 7px;
    color: var(--msc2-text-primary);
    font-size: 12px;
    text-decoration: none;
  }
  .provider-link:hover {
    background: var(--msc2-neutral-muted);
  }
  .compat {
    margin: 14px 0;
    font-size: 12px;
    color: var(--msc2-status-ok);
  }
  .compat.warn,
  .detail-error {
    color: var(--msc2-status-warn);
  }
  .detail-section {
    margin: 18px 0;
  }
  .detail-section h3 {
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .gallery {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    scrollbar-width: thin;
  }
  .gallery img {
    width: 220px;
    height: 124px;
    object-fit: cover;
    border-radius: 8px;
    flex-shrink: 0;
  }
  .about {
    font-size: 12.5px;
    line-height: 1.6;
    color: var(--msc2-text-secondary);
  }
  .about p {
    margin: 0 0 8px;
    white-space: pre-wrap;
  }
  .about a {
    color: var(--msc2-text-primary);
  }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .stable-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .versions {
    display: flex;
    flex-direction: column;
  }
  .version-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .version-toggle {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    min-width: 0;
    flex: 1;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font: inherit;
  }
  .chevron {
    display: inline-flex;
    flex: 0 0 12px;
    margin-top: 4px;
    color: var(--msc2-text-tertiary);
    transition: transform 120ms ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .version-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .version-line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }
  .version-name {
    font-size: 12px;
    color: var(--msc2-text-primary);
  }
  .version-mc,
  .version-downloads {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .version-detail {
    padding: 4px 0 10px 22px;
  }
  .detail-label {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .version-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-top: 6px;
  }
  .tag {
    padding: 3px 7px;
    border-radius: 5px;
    background: var(--msc2-neutral-muted);
    color: var(--msc2-text-secondary);
    font-size: 10px;
  }
  .tag.highlight {
    background: var(--msc2-status-ok-tint);
    color: var(--msc2-status-ok);
  }
  .version-downloads {
    margin: 8px 0 0;
  }
</style>
