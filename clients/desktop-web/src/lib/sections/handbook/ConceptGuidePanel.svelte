<script lang="ts">
  // MSC 1 ConceptGuideView.swift, rebuilt to the S0 disciplined system. The
  // oracle's per-page accent color (blue/teal/orange/indigo backdrops),
  // gradient background, and staggered fade-in animation are dropped --
  // one flat neutral diagram tile per page (ConceptDiagram.svelte), no
  // page-level color theme, matching antiAIslop.md rules #1/#3/#13.
  import Button from '../../components/base/Button.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import HelpLink from '../../help/HelpLink.svelte';
  import type { ConceptGuide } from '../../help/types';
  import ConceptDiagram from './ConceptDiagram.svelte';

  export let concept: ConceptGuide | null = null;
  export let page = 0;
  export let hostId = 'local-agent';
  export let serverId = 'survival';
  export let onFinish: () => void = () => {};
  export let onOpenHandbook: () => void = () => {};

  $: current = concept?.pages[page] ?? null;
  $: isLast = concept ? page === concept.pages.length - 1 : false;

  function advance(): void {
    if (!concept) return;
    if (isLast) {
      onFinish();
      return;
    }
    page += 1;
  }
</script>

{#if current}
  <div class="guide">
    <div class="head">
      <p class="msc2-type-overline">{current.eyebrow}</p>
      <span class="msc2-type-meta">{page + 1} / {concept?.pages.length}</span>
    </div>
    <h3 class="msc2-type-page">{current.title}</h3>
    <ConceptDiagram diagram={current.diagram} />
    <p class="body msc2-type-body">{current.body}</p>
    <HelpLink helpId={current.helpId} {hostId} {serverId} />
    <div class="footer">
      <Button size="sm" variant="secondary" disabled={page === 0} onclick={() => (page -= 1)}>
        Previous
      </Button>
      <div class="dots" aria-hidden="true">
        {#each concept?.pages ?? [] as p, i (p.helpId)}
          <span class="dot" class:filled={i === page}></span>
        {/each}
      </div>
      {#if isLast}
        <div class="last-actions">
          <Button size="sm" variant="secondary" onclick={onOpenHandbook}>Open Handbook</Button>
          <Button size="sm" variant="primary" onclick={advance}>Done</Button>
        </div>
      {:else}
        <Button size="sm" variant="primary" onclick={advance}>Next</Button>
      {/if}
    </div>
  </div>
{:else}
  <EmptyState title="How MSC Works is unavailable" message="This agent has not served the concept guide yet." />
{/if}

<style>
  .guide {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .guide h3 {
    margin: 0;
  }
  .body {
    color: var(--msc2-text-secondary);
    line-height: 1.6;
    margin: 0;
  }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: 6px;
    padding-top: 10px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .dots {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--msc2-hairline);
  }
  .dot.filled {
    background: var(--msc2-text-secondary);
  }
  .last-actions {
    display: flex;
    gap: 8px;
  }
</style>
