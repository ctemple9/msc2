<script lang="ts">
  // MSC 1 ServerHandbookView.swift, rebuilt to the S0 disciplined system
  // (docs/msc2/antiAIslop.md). The oracle's per-category colored icon tiles
  // and hero banner image are dropped -- category is a neutral Badge, and
  // navigation is a plain grouped list, matching rule #6/#9's fix.
  import Badge from '../../components/base/Badge.svelte';
  import Button from '../../components/base/Button.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Field from '../../components/base/Field.svelte';
  import { renderMarkdown } from '../../help/markdown';
  import type { HelpCatalog, HelpTopic } from '../../help/types';
  import { HANDBOOK_CATEGORY_ORDER, HANDBOOK_TOPIC_ORDER, topicOrderIndex } from './catalogOrder';

  export let catalog: HelpCatalog = { topics: [] };
  export let topic: HelpTopic | null = null;
  export let topicId = '';
  export let onSelect: (id: string) => void = () => {};

  let search = '';

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
  $: currentIndex = HANDBOOK_TOPIC_ORDER.indexOf(topicId);
  $: previousId = currentIndex > 0 ? HANDBOOK_TOPIC_ORDER[currentIndex - 1] : null;
  $: nextId =
    currentIndex >= 0 && currentIndex < HANDBOOK_TOPIC_ORDER.length - 1
      ? HANDBOOK_TOPIC_ORDER[currentIndex + 1]
      : null;
  $: categoryLabel = HANDBOOK_CATEGORY_ORDER.find((c) => c.slug === topic?.category)?.label ?? '';
</script>

<div class="browser">
  <aside class="topics">
    <Field placeholder="Search topics…" bind:value={search} />
    <div class="topic-list" role="tree" aria-label="Handbook topics">
      {#each groups as group (group.slug)}
        <p class="group-label msc2-type-overline">{group.label}</p>
        {#each group.topics as item (item.helpId)}
          <button
            type="button"
            class="topic-row"
            class:selected={item.helpId === topicId}
            onclick={() => onSelect(item.helpId)}
          >
            {item.title}
          </button>
        {/each}
      {/each}
      {#if groups.length === 0}
        <p class="no-results msc2-type-meta">No topics match “{search}”.</p>
      {/if}
    </div>
  </aside>

  <article class="reader">
    {#if topic}
      {#if categoryLabel}<Badge>{categoryLabel}</Badge>{/if}
      <h3 class="msc2-type-page">{topic.title}</h3>
      {#if topic.analogy}<p class="analogy msc2-type-body">{topic.analogy}</p>{/if}
      <div class="markdown msc2-type-body" data-safe-markdown="true">
        {@html renderMarkdown(topic.body)}
      </div>
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
          Back
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

<style>
  .browser {
    display: grid;
    grid-template-columns: minmax(11rem, 0.32fr) minmax(0, 1fr);
    gap: 20px;
  }
  .topics {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .topic-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    max-height: 26rem;
    overflow-y: auto;
  }
  .group-label {
    margin: 10px 0 2px;
  }
  .group-label:first-child {
    margin-top: 0;
  }
  .topic-row {
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
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .reader h3 {
    margin: 2px 0 0;
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
  .markdown :global(h1),
  .markdown :global(h2),
  .markdown :global(h3) {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .markdown :global(a) {
    color: var(--msc2-text-primary);
    text-decoration: underline;
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
  @media (max-width: 720px) {
    .browser {
      grid-template-columns: 1fr;
    }
    .topic-list {
      max-height: 13rem;
    }
  }
</style>
