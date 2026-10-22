<script lang="ts">
  // MSC 1 ServerHandbookView.swift, rebuilt to the S0 disciplined system
  // (docs/msc2/antiAIslop.md). The oracle's per-category colored icon tiles
  // and hero banner image are dropped -- category is a neutral Badge, and
  // navigation is a plain grouped list, matching rule #6/#9's fix.
  import { onMount, tick } from 'svelte';
  import Badge from '../../components/base/Badge.svelte';
  import Button from '../../components/base/Button.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Field from '../../components/base/Field.svelte';
  import Menu from '../../components/base/Menu.svelte';
  import { renderMarkdown } from '../../help/markdown';
  import type { HelpCatalog, HelpTopic } from '../../help/types';
  import { HANDBOOK_CATEGORY_ORDER, HANDBOOK_TOPIC_ORDER, topicOrderIndex } from './catalogOrder';

  export let catalog: HelpCatalog = { topics: [] };
  export let topic: HelpTopic | null = null;
  export let topicId = '';
  export let onSelect: (id: string) => void = () => {};
  export let onOpenRouter: () => void = () => {};
  export let onBack: () => void = () => {};
  export let canGoBack = false;

  let search = '';
  const favoritesKey = 'msc2.handbook.favorites.v1';
  const defaultFavorites = [
    'handbook.first-server',
    'handbook.first-modded-server',
    'handbook.first-bedrock-server',
  ];
  let favorites = [...defaultFavorites];
  let readerElement: HTMLElement;
  let scrolledTopicId = '';
  let menuTopicId: string | null = null;
  let menuPosition = { x: 0, y: 0 };

  onMount(() => {
    try {
      const saved = localStorage.getItem(favoritesKey);
      if (saved !== null) {
        const parsed: unknown = JSON.parse(saved);
        if (Array.isArray(parsed)) {
          favorites = parsed.filter(
            (id): id is string =>
              typeof id === 'string' && catalog.topics.some((item) => item.helpId === id),
          );
        }
      }
    } catch {
      // An unavailable or malformed local store should not hide the default guides.
    }
  });

  function toggleFavorite(id: string): void {
    favorites = favorites.includes(id)
      ? favorites.filter((item) => item !== id)
      : [...favorites, id];
    try {
      localStorage.setItem(favoritesKey, JSON.stringify(favorites));
    } catch {
      // Favorites remain usable for this session when storage is unavailable.
    }
  }

  async function openFavoriteMenu(id: string, event: MouseEvent | KeyboardEvent): Promise<void> {
    event.preventDefault();
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menuPosition =
      event instanceof MouseEvent && event.clientX !== 0
        ? { x: event.clientX, y: event.clientY }
        : { x: rect.left, y: rect.bottom };
    menuTopicId = id;
    await tick();
    document.querySelector<HTMLButtonElement>('[role="menu"] [role="menuitem"]')?.focus();
  }

  function handleTopicKeydown(id: string, event: KeyboardEvent): void {
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      openFavoriteMenu(id, event);
    }
  }

  function openArticleLink(event: MouseEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const link = target.closest('a[data-help-topic], a[data-help-view]');
    if (link?.getAttribute('data-help-view') === 'router') {
      event.preventDefault();
      onOpenRouter();
      return;
    }
    const id = link?.getAttribute('data-help-topic');
    if (!id) return;
    event.preventDefault();
    if (catalog.topics.some((item) => item.helpId === id)) onSelect(id);
  }

  function bindArticleLinks(node: HTMLElement): { destroy(): void } {
    node.addEventListener('click', openArticleLink);
    return { destroy: () => node.removeEventListener('click', openArticleLink) };
  }

  $: orderedTopics = [...catalog.topics].sort(
    (a, b) => topicOrderIndex(a.helpId) - topicOrderIndex(b.helpId),
  );
  $: filtered = search.trim()
    ? orderedTopics.filter((item) => item.title.toLowerCase().includes(search.trim().toLowerCase()))
    : orderedTopics;
  $: groups = HANDBOOK_CATEGORY_ORDER.map((category) => ({
    ...category,
    topics: filtered.filter((item) => item.category === category.slug),
  })).filter((group) => group.topics.length > 0);
  $: favoriteTopics = favorites
    .map((id) => filtered.find((item) => item.helpId === id))
    .filter((item): item is (typeof filtered)[number] => item !== undefined);
  $: currentIndex = HANDBOOK_TOPIC_ORDER.indexOf(topicId);
  $: previousId = currentIndex > 0 ? HANDBOOK_TOPIC_ORDER[currentIndex - 1] : null;
  $: nextId =
    currentIndex >= 0 && currentIndex < HANDBOOK_TOPIC_ORDER.length - 1
      ? HANDBOOK_TOPIC_ORDER[currentIndex + 1]
      : null;
  $: categoryLabel = HANDBOOK_CATEGORY_ORDER.find((c) => c.slug === topic?.category)?.label ?? '';
  $: if (topicId && topicId !== scrolledTopicId) {
    scrolledTopicId = topicId;
    if (readerElement) readerElement.scrollTop = 0;
  }
</script>

<div class="browser">
  <div class="welcome-banner">
    <h2>You don’t need to read all of this right now.</h2>
    <p>
      Come back when you want to learn how something works in MSC 2. Start with a guide in
      Favorites, or find a topic when you need it.
    </p>
  </div>
  <div class="browser-columns">
    <aside class="topics">
      <Field placeholder="Search topics…" bind:value={search} />
      <div class="topic-list" role="tree" aria-label="Handbook topics">
        <p class="group-label msc2-type-overline">Favorites</p>
        {#if favoriteTopics.length > 0}
          {#each favoriteTopics as item (item.helpId)}
            <button
              type="button"
              class="topic-row"
              class:selected={item.helpId === topicId}
              aria-keyshortcuts="Shift+F10 ContextMenu"
              onclick={() => onSelect(item.helpId)}
              oncontextmenu={(event) => openFavoriteMenu(item.helpId, event)}
              onkeydown={(event) => handleTopicKeydown(item.helpId, event)}>{item.title}</button
            >
          {/each}
        {:else if !search.trim()}
          <p class="no-results msc2-type-meta">Save topics you use often.</p>
        {/if}
        {#each groups as group (group.slug)}
          <p class="group-label msc2-type-overline">{group.label}</p>
          {#each group.topics as item (item.helpId)}
            <button
              type="button"
              class="topic-row"
              class:selected={item.helpId === topicId}
              aria-keyshortcuts="Shift+F10 ContextMenu"
              onclick={() => onSelect(item.helpId)}
              oncontextmenu={(event) => openFavoriteMenu(item.helpId, event)}
              onkeydown={(event) => handleTopicKeydown(item.helpId, event)}>{item.title}</button
            >
          {/each}
        {/each}
        {#if groups.length === 0}
          <p class="no-results msc2-type-meta">No topics match “{search}”.</p>
        {/if}
      </div>
    </aside>

    <article class="reader" bind:this={readerElement} use:bindArticleLinks>
      {#if topic}
        {#if categoryLabel}<Badge>{categoryLabel}</Badge>{/if}
        <div class="reader-heading">
          <h3 class="msc2-type-page">{topic.title}</h3>
          <Button size="sm" variant="secondary" disabled={!canGoBack} onclick={onBack}>Back</Button>
        </div>
        {#if topic.subtitle}
          <p
            class="subtitle msc2-type-body"
            class:overview-subtitle={topic.helpId === 'handbook.overview'}
          >
            {topic.subtitle}
          </p>
        {/if}
        {#if topic.analogy}<p class="analogy msc2-type-body">{topic.analogy}</p>{/if}
        <div class="markdown msc2-type-body" data-safe-markdown="true">
          {@html renderMarkdown(topic.body)}
        </div>
        {#each topic.sections ?? [] as block, blockIndex (blockIndex)}
          {#if block.type === 'body'}
            <div class="markdown msc2-type-body" data-safe-markdown="true">
              {@html renderMarkdown(block.markdown)}
            </div>
          {:else if block.type === 'bulletList'}
            <div class="markdown msc2-type-body" data-safe-markdown="true">
              {@html renderMarkdown(block.items.map((item) => `- ${item}`).join('\n'))}
            </div>
          {:else if block.type === 'callout'}
            <div class="callout">
              <p class="callout-label msc2-type-overline">{block.style}</p>
              <p class="callout-text msc2-type-body">{block.text}</p>
            </div>
          {:else if block.type === 'inApp'}
            <div class="callout">
              <p class="callout-label msc2-type-overline">In this app</p>
              <div class="markdown msc2-type-body" data-safe-markdown="true">
                {@html renderMarkdown(block.items.map((item) => `- ${item}`).join('\n'))}
              </div>
            </div>
          {:else if block.type === 'advanced'}
            <details class="advanced">
              <summary class="msc2-type-overline">Advanced details</summary>
              <div class="markdown msc2-type-body" data-safe-markdown="true">
                {@html renderMarkdown(block.markdown)}
              </div>
            </details>
          {:else if block.type === 'checklist'}
            <div class="checklist">
              <p class="checklist-phase msc2-type-section">{block.phase}</p>
              <ol class="checklist-steps msc2-type-body" start={block.steps[0]?.number}>
                {#each block.steps as step (step.number)}
                  <li data-safe-markdown="true">
                    <strong>{step.title}</strong> — {@html renderMarkdown(step.detail)}
                  </li>
                {/each}
              </ol>
            </div>
          {:else if block.type === 'table'}
            <div class="table-wrap">
              <table class="guide-table msc2-type-body">
                <thead>
                  <tr>
                    {#each block.headers as header (header)}<th>{header}</th>{/each}
                  </tr>
                </thead>
                <tbody>
                  {#each block.rows as row, rowIndex (rowIndex)}
                    <tr>
                      {#each row as cell, cellIndex (cellIndex)}<td>{cell}</td>{/each}
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        {/each}
        {#if topic.relatedIds.length}
          <p class="msc2-type-overline related-label">Related topics</p>
          <div class="related-topics">
            {#each topic.relatedIds as relatedId (relatedId)}
              {@const relatedTitle =
                catalog.topics.find((t) => t.helpId === relatedId)?.title ?? relatedId}
              <button type="button" class="related-pill" onclick={() => onSelect(relatedId)}>
                {relatedTitle}
              </button>
            {/each}
          </div>
        {/if}
        <div class="reader-footer">
          <Button
            size="sm"
            variant="secondary"
            disabled={!previousId}
            onclick={() => previousId && onSelect(previousId)}
          >
            Previous topic
          </Button>
          <span class="progress msc2-type-meta">
            {currentIndex >= 0 ? currentIndex + 1 : 1} / {HANDBOOK_TOPIC_ORDER.length}
          </span>
          <Button
            size="sm"
            variant="secondary"
            disabled={!nextId}
            onclick={() => nextId && onSelect(nextId)}
          >
            Next
          </Button>
        </div>
      {:else}
        <EmptyState
          title="That topic is not available on this agent"
          message={`The link is preserved so a newer agent can provide it later. (${topicId})`}
        />
      {/if}
    </article>
  </div>
</div>
{#if menuTopicId}
  <Menu
    x={menuPosition.x}
    y={menuPosition.y}
    onClose={() => (menuTopicId = null)}
    items={[
      {
        label: favorites.includes(menuTopicId) ? 'Remove from Favorites' : 'Save to Favorites',
        onSelect: () => menuTopicId && toggleFavorite(menuTopicId),
      },
    ]}
  />
{/if}

<style>
  .browser {
    display: flex;
    flex-direction: column;
    flex: 1;
    width: 100%;
    min-width: 0;
    min-height: 0;
  }
  .welcome-banner {
    padding: 1px 0 16px;
    margin-bottom: 18px;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .welcome-banner h2 {
    margin: 0 0 5px;
    color: var(--msc2-text-primary);
    font-size: 18px;
    font-weight: 600;
  }
  .welcome-banner p {
    margin: 0;
    color: var(--msc2-text-secondary);
    font-size: 13px;
    line-height: 1.5;
  }
  .browser-columns {
    display: flex;
    align-items: stretch;
    gap: 20px;
    flex: 1;
    width: 100%;
    min-width: 0;
    min-height: 0;
  }
  .topics {
    display: flex;
    flex-direction: column;
    flex: 0 0 clamp(12rem, 26%, 22rem);
    gap: 10px;
    min-width: 0;
    min-height: 0;
  }
  .topic-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .group-label {
    margin: 10px 0 2px;
  }
  .group-label:first-child {
    margin-top: 0;
  }
  .topic-row {
    flex: 1;
    min-width: 0;
    text-align: left;
    padding: 6px 8px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--msc2-text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .topic-row:hover {
    background: var(--msc2-hairline-subtle);
    color: var(--msc2-text-primary);
  }
  .topic-row.selected {
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .no-results {
    padding: 6px 8px;
  }
  .reader {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
  }
  .reader-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .checklist-steps li :global(p) {
    display: inline;
    margin: 0;
  }
  .reader h3 {
    margin: 2px 0 0;
  }
  .subtitle {
    color: var(--msc2-text-secondary);
    margin: 0;
  }
  .overview-subtitle {
    font-style: italic;
  }
  .analogy {
    color: var(--msc2-text-secondary);
    font-style: italic;
    margin: 0;
  }
  .markdown {
    color: var(--msc2-text-secondary);
    line-height: 1.6;
  }
  .markdown :global(p) {
    margin: 0 0 10px;
  }
  .markdown :global(p:last-child) {
    margin-bottom: 0;
  }
  .markdown :global(h1),
  .markdown :global(h2),
  .markdown :global(h3) {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .markdown :global(h2) {
    margin: 18px 0 10px;
    font-size: 1.1em;
    line-height: 1.3;
  }
  .markdown :global(a) {
    color: var(--msc2-text-primary);
    text-decoration: underline;
  }
  .markdown :global(strong) {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .markdown :global(code) {
    font-family: var(--msc2-font-mono);
    font-size: 0.9em;
    color: var(--msc2-text-primary);
    background: var(--msc2-hairline-subtle);
    padding: 0.1em 0.35em;
    border-radius: 4px;
  }
  .markdown :global(ul) {
    margin: 0 0 10px;
    padding-left: 1.1rem;
  }
  .markdown :global(li) {
    margin-bottom: 4px;
  }
  .callout,
  .checklist,
  .advanced {
    border: 1px solid var(--msc2-hairline);
    border-radius: var(--msc2-radius-2);
    padding: 12px 14px;
  }
  .callout-label {
    margin: 0 0 4px;
  }
  .callout-text {
    color: var(--msc2-text-secondary);
    margin: 0;
    line-height: 1.6;
  }
  .checklist-phase {
    margin: 0 0 8px;
  }
  .checklist-steps {
    margin: 0;
    padding-left: 1.2rem;
    color: var(--msc2-text-secondary);
    line-height: 1.6;
  }
  .checklist-steps li {
    margin-bottom: 6px;
  }
  .checklist-steps strong {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .advanced summary {
    cursor: pointer;
    color: var(--msc2-text-tertiary);
  }
  .advanced .markdown {
    margin-top: 10px;
  }
  .table-wrap {
    overflow-x: auto;
  }
  .guide-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .guide-table th,
  .guide-table td {
    text-align: left;
    padding: 6px 10px;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
    color: var(--msc2-text-secondary);
  }
  .guide-table th {
    color: var(--msc2-text-tertiary);
    font-weight: 500;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }
  .related-label {
    margin: 6px 0 0;
  }
  .related-topics {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .related-pill {
    border: 1px solid var(--msc2-hairline);
    border-radius: 999px;
    padding: 4px 11px;
    background: transparent;
    color: var(--msc2-text-secondary);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .related-pill:hover {
    color: var(--msc2-text-primary);
    background: var(--msc2-hairline-subtle);
  }
  .reader-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 6px;
    padding-top: 10px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  @media (max-width: 560px) {
    .browser-columns {
      flex-direction: column;
      flex: none;
    }
    .browser {
      flex: none;
    }
    .topics {
      flex: none;
      min-height: auto;
    }
    .topic-list {
      max-height: 13rem;
      flex: none;
    }
    .reader {
      overflow-y: visible;
    }
  }
</style>
