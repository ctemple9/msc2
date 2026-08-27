<script lang="ts">
  // Ports ModrinthProjectDetailView (ModrinthBrowserView.swift:316-785): the
  // page a search result opens into -- gallery, full About text, and a
  // per-version list scoped to this server's compatibility, with a specific
  // build installable instead of always "latest". Needs the P12.7c routes
  // (GET /v1/catalog/projects/:id[/versions]) and the CatalogInstallRequestDTO
  // .versionId field; P12.7/P12.7a left this page undone since neither existed.
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Badge from '../../components/base/Badge.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import {
    addonPaths,
    catalogDetailPaths,
    collapseVersions,
    conflictCount,
    expandedLoaders,
    filterVisibleVersions,
    formatCount,
    isStableVersion,
    isVersionCompatible,
    modrinthLoaderFacets,
    parseInlineMarkdown,
    pollOperation,
    sanitizeModrinthBody,
  } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let item: Schema['CatalogItemDTO'];
  export let javaFlavor: string | undefined = undefined;
  export let serverMinecraftVersion: string | undefined = undefined;
  export let onClose: () => void;
  export let onInstalled: (projectId: string) => void;

  let project: Schema['CatalogProjectDetailDTO'] | undefined;
  let versions: Schema['CatalogVersionDTO'][] = [];
  let loading = true;
  let errorText: string | undefined;
  let stableOnly = true;
  let installingVersionId: string | undefined;
  let installedVersionIds: Set<string> = new Set();
  let expandedVersionIds: Set<string> = new Set();
  let notice = '';

  $: serverLoaders = new Set(modrinthLoaderFacets(javaFlavor));
  $: loaderFilter = expandedLoaders(javaFlavor, modrinthLoaderFacets(javaFlavor));
  $: collapsed = collapseVersions(versions, serverLoaders);
  $: visible = filterVisibleVersions(collapsed, { stableOnly, loaders: loaderFilter });
  $: hasCompatibleVersion = versions.some((v) => isVersionCompatible(v, serverMinecraftVersion));
  $: aboutParagraphs = sanitizeModrinthBody(project?.body ?? item.description)
    .split('\n\n')
    .filter((p) => p.trim().length > 0);
  $: modrinthURL = `https://modrinth.com/${item.projectType}/${item.slug}`;

  onMount(async () => {
    if (!api) {
      loading = false;
      errorText = 'Not connected to an agent.';
      return;
    }
    try {
      const [detail, versionsResponse] = await Promise.all([
        api.get<Schema['CatalogProjectDetailDTO']>(catalogDetailPaths.project(item.projectId)),
        api.get<Schema['CatalogVersionsResponseDTO']>(catalogDetailPaths.versions(item.projectId)),
      ]);
      project = detail;
      versions = versionsResponse.versions ?? [];
      // Projects like Geyser/Floodgate publish every build through the beta
      // channel and never mark a version "release" -- stableOnly would hide
      // everything, so turn it off automatically (load(), line 762-764).
      if (versions.length > 0 && !versions.some(isStableVersion)) {
        stableOnly = false;
      }
    } catch (error) {
      errorText =
        error instanceof Error ? error.message : "Couldn't load this project from Modrinth.";
    } finally {
      loading = false;
    }
  });

  function toggleExpanded(versionId: string): void {
    const next = new Set(expandedVersionIds);
    if (next.has(versionId)) next.delete(versionId);
    else next.add(versionId);
    expandedVersionIds = next;
  }

  async function installVersion(version: Schema['CatalogVersionDTO']): Promise<void> {
    if (!api) return;
    installingVersionId = version.id;
    try {
      const result = await mutate<Schema['CatalogInstallResultDTO']>(api, addonPaths.install, {
        projectId: item.projectId,
        slug: item.slug,
        title: item.title,
        versionId: version.id,
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
      installedVersionIds = new Set(installedVersionIds).add(version.id);
      onInstalled(item.projectId);
    } catch (error) {
      notice =
        error instanceof Error ? error.message : `Failed to install ${version.versionNumber}.`;
    } finally {
      installingVersionId = undefined;
    }
  }

  function channelTone(versionType: string): 'ok' | 'warn' | 'error' {
    if (versionType === 'release') return 'ok';
    if (versionType === 'beta') return 'warn';
    return 'error';
  }

  function serverSideLabel(serverSide: string): string {
    if (serverSide === 'unsupported') return 'Client-only — does nothing on a server';
    if (serverSide === 'required') return 'Server-side required';
    return 'Server-side optional';
  }
</script>

<Sheet title={item.title} size="lg" {onClose}>
  <div class="header">
    <div class="icon">
      {#if item.iconURL}
        <img src={item.iconURL} alt="" width="56" height="56" loading="lazy" />
      {:else}
        <Icon name="box" size={22} />
      {/if}
    </div>
    <div class="header-info">
      <span class="title">{item.title}</span>
      <p class="byline">by {item.author}</p>
      <p class="stats">
        {formatCount(project?.downloads ?? item.downloads)} downloads{#if project}
          · {formatCount(project.followers)} followers{/if}
      </p>
      {#if project}
        <Badge variant="status" tone={project.serverSide === 'unsupported' ? 'warn' : 'ok'}>
          {serverSideLabel(project.serverSide)}
        </Badge>
      {/if}
    </div>
  </div>

  <a class="modrinth-link" href={modrinthURL} target="_blank" rel="noopener noreferrer">
    View on Modrinth
  </a>

  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  {#if loading}
    <p class="explain">Loading…</p>
  {:else if errorText}
    <p class="explain warn">{errorText}</p>
  {:else}
    {#if serverMinecraftVersion}
      <p class="compat" class:warn={!hasCompatibleVersion}>
        {hasCompatibleVersion
          ? `A version is available for your server (${serverMinecraftVersion}).`
          : `No version yet for Minecraft ${serverMinecraftVersion}. You can still install another version below, at your own risk.`}
      </p>
    {/if}

    {#if project && project.gallery.length > 0}
      <div class="section">
        <h3>Gallery</h3>
        <div class="gallery">
          {#each project.gallery as image (image.url)}
            <img src={image.url} alt={image.title ?? ''} loading="lazy" />
          {/each}
        </div>
      </div>
    {/if}

    <div class="section">
      <h3>About</h3>
      <div class="about">
        {#each aboutParagraphs as paragraph}
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
    </div>

    <div class="section">
      <div class="section-header">
        <h3>Versions</h3>
        <div class="stable-toggle">
          <Toggle checked={stableOnly} label="Stable only" onchange={(v) => (stableOnly = v)} />
          <span>Stable only</span>
        </div>
      </div>
      {#if visible.length === 0}
        {#if stableOnly && collapsed.length > 0}
          <p class="explain">
            Only pre-release builds are available.
            <button type="button" class="link-button" onclick={() => (stableOnly = false)}>
              Show them
            </button>
          </p>
        {:else}
          <p class="explain">No versions found for this loader.</p>
        {/if}
      {:else}
        <div class="versions">
          {#each visible.slice(0, 40) as version (version.id)}
            {@const compatible = isVersionCompatible(version, serverMinecraftVersion)}
            {@const conflicts = conflictCount(version)}
            {@const isExpanded = expandedVersionIds.has(version.id)}
            <div class="version-row">
              <button
                type="button"
                class="version-toggle"
                onclick={() => toggleExpanded(version.id)}
              >
                <span class="chevron" class:open={isExpanded}
                  ><Icon name="chevron" size={11} /></span
                >
                <span class="version-main">
                  <span class="version-line">
                    <span class="version-number">{version.versionNumber}</span>
                    <Badge variant="status" tone={channelTone(version.versionType)}>
                      {version.versionType}
                    </Badge>
                    <Badge variant="status" tone={compatible ? 'ok' : 'warn'}>
                      {compatible ? 'Compatible' : 'Other version'}
                    </Badge>
                    {#if conflicts > 0}
                      <Badge variant="status" tone="error">
                        {conflicts === 1 ? '1 conflict' : `${conflicts} conflicts`}
                      </Badge>
                    {/if}
                  </span>
                  <span class="version-mc">
                    MC {version.gameVersions.slice(0, 4).join(', ')}{version.gameVersions.length > 4
                      ? '…'
                      : ''}
                  </span>
                </span>
              </button>
              {#if installedVersionIds.has(version.id)}
                <span class="added">Added</span>
              {:else if installingVersionId === version.id}
                <span class="added">Installing…</span>
              {:else}
                <Button size="sm" variant="secondary" onclick={() => void installVersion(version)}>
                  {compatible ? 'Install' : 'Install anyway'}
                </Button>
              {/if}
            </div>
            {#if conflicts > 0}
              <p class="conflict-note">
                This version declares {conflicts === 1 ? 'another mod' : `${conflicts} other mods`} incompatible.
                Check before installing.
              </p>
            {/if}
            {#if isExpanded}
              <div class="version-detail">
                <p class="detail-label">Supported Minecraft versions</p>
                <div class="version-tags">
                  {#each version.gameVersions as gv (gv)}
                    <span class="tag" class:highlight={gv === serverMinecraftVersion}>{gv}</span>
                  {/each}
                </div>
                {#if version.loaders.length > 0}
                  <p class="platforms">
                    Platforms: {version.loaders
                      .map((l) => l.charAt(0).toUpperCase() + l.slice(1))
                      .join(', ')}
                  </p>
                {/if}
              </div>
            {/if}
          {/each}
        </div>
      {/if}
    </div>

    {#if project && (project.sourceURL || project.issuesURL || project.wikiURL || project.discordURL)}
      <div class="section links">
        {#if project.sourceURL}<a href={project.sourceURL} target="_blank" rel="noopener noreferrer"
            >Source</a
          >{/if}
        {#if project.issuesURL}<a href={project.issuesURL} target="_blank" rel="noopener noreferrer"
            >Issues</a
          >{/if}
        {#if project.wikiURL}<a href={project.wikiURL} target="_blank" rel="noopener noreferrer"
            >Wiki</a
          >{/if}
        {#if project.discordURL}<a
            href={project.discordURL}
            target="_blank"
            rel="noopener noreferrer">Discord</a
          >{/if}
      </div>
    {/if}
  {/if}
</Sheet>

<style>
  .header {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    margin-bottom: 10px;
  }
  .icon {
    flex-shrink: 0;
    width: 56px;
    height: 56px;
    border-radius: 10px;
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
  .header-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .byline,
  .stats {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .modrinth-link {
    display: inline-flex;
    align-items: center;
    font-size: 12px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.9);
    background: transparent;
    border: 1px solid var(--msc2-hairline);
    border-radius: 7px;
    padding: 5px 12px;
    margin-bottom: 14px;
    text-decoration: none;
  }
  .modrinth-link:hover {
    background: rgba(255, 255, 255, 0.06);
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
  .explain.warn {
    color: var(--msc2-status-warn);
  }
  .link-button {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    font-size: 12px;
    color: var(--msc2-neutral-fill);
    cursor: pointer;
    text-decoration: underline;
  }
  .compat {
    margin: 0 0 14px;
    font-size: 12px;
    color: var(--msc2-status-ok);
  }
  .compat.warn {
    color: var(--msc2-status-warn);
  }
  .section {
    margin-bottom: 18px;
  }
  .section h3 {
    margin: 0 0 8px;
    font-size: 12px;
    font-weight: 600;
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
  .gallery {
    display: flex;
    gap: 8px;
    overflow-x: auto;
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
    color: rgba(255, 255, 255, 0.9);
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
  .versions .version-row:first-child {
    border-top: none;
  }
  .version-toggle {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    color: inherit;
    min-width: 0;
  }
  .chevron {
    display: inline-flex;
    color: var(--msc2-text-tertiary);
    margin-top: 3px;
    transition: transform 120ms ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .version-main {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .version-line {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .version-number {
    font-size: 12px;
    font-weight: 500;
    font-family: var(--msc2-font-mono, monospace);
    color: var(--msc2-text-primary);
  }
  .version-mc {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .conflict-note {
    margin: 0 0 8px 19px;
    font-size: 11px;
    color: var(--msc2-status-error);
  }
  .version-detail {
    margin: 0 0 8px 19px;
    padding: 8px;
    border-radius: 6px;
    background: var(--msc2-tier-chrome);
  }
  .detail-label {
    margin: 0 0 4px;
    font-size: 10px;
    font-weight: 600;
    color: var(--msc2-text-tertiary);
  }
  .version-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tag {
    font-size: 9.5px;
    font-family: var(--msc2-font-mono, monospace);
    padding: 2px 5px;
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--msc2-text-tertiary);
  }
  .tag.highlight {
    font-weight: 700;
    background: var(--msc2-status-ok-tint);
    color: var(--msc2-status-ok);
  }
  .platforms {
    margin: 6px 0 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .added {
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-status-ok);
  }
  .links {
    display: flex;
    gap: 14px;
  }
  .links a {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.85);
  }
</style>
