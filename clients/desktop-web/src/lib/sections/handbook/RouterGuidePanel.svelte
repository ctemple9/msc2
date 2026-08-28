<script lang="ts">
  // MSC 1 RouterPortForwardGuideSheet.swift's 3-screen flow (picker -> guide
  // reader -> troubleshooting), rebuilt to the S0 disciplined system. The
  // oracle's per-guide "verified recently / community based" confidence
  // badge has no backing field in RouterGuide (crates/msc-domain/src/
  // router_guides.rs) -- left out rather than invented, same handling
  // P12.18c gave Bedrock's dead Docker field.
  import Button from '../../components/base/Button.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import type { RouterGuideCatalog } from '../../help/types';
  import { ROUTER_CATEGORY_ORDER } from './catalogOrder';

  export let catalog: RouterGuideCatalog = { guides: [], troubleshooting: [] };

  type Stage = 'picker' | 'reader' | 'troubleshooting';
  let stage: Stage = 'picker';
  let guideId = '';

  $: groups = ROUTER_CATEGORY_ORDER.map((category) => ({
    ...category,
    guides: catalog.guides.filter((guide) => guide.category === category.slug),
  })).filter((group) => group.guides.length > 0);
  $: guide = catalog.guides.find((g) => g.id === guideId) ?? null;

  function openGuide(id: string): void {
    guideId = id;
    stage = 'reader';
  }
</script>

<div class="panel">
  <div class="head">
    <p class="msc2-type-overline">
      {stage === 'picker'
        ? 'Step 1 of 3 — Find your router'
        : stage === 'reader'
          ? `Step 2 of 3 — ${guide?.displayName ?? 'Follow the steps'}`
          : 'Step 3 of 3 — Diagnose issues'}
    </p>
    {#if stage !== 'picker'}
      <Button size="sm" variant="secondary" onclick={() => (stage = 'picker')}>Start over</Button>
    {/if}
  </div>

  {#if stage === 'picker'}
    {#if groups.length === 0}
      <EmptyState title="Router guides are unavailable" message="This agent has not served the router catalog yet." />
    {:else}
      <div class="groups">
        {#each groups as group (group.slug)}
          <div class="group">
            <p class="group-label msc2-type-overline">{group.label}</p>
            <div class="guide-list">
              {#each group.guides as g (g.id)}
                <button type="button" class="guide-row" onclick={() => openGuide(g.id)}>
                  {g.displayName}
                </button>
              {/each}
            </div>
          </div>
        {/each}
      </div>
      <div class="footer single">
        <Button size="sm" variant="secondary" onclick={() => (stage = 'troubleshooting')}>
          Already forwarded a port? Troubleshoot instead
        </Button>
      </div>
    {/if}
  {:else if stage === 'reader' && guide}
    <ol class="steps">
      {#each guide.steps as step, i (i)}<li>{step}</li>{/each}
    </ol>
    <div class="footer">
      <Button size="sm" variant="secondary" onclick={() => (stage = 'picker')}>Back</Button>
      <Button size="sm" variant="secondary" onclick={() => (stage = 'troubleshooting')}>
        Troubleshooting →
      </Button>
    </div>
  {:else if stage === 'troubleshooting'}
    {#if catalog.troubleshooting.length === 0}
      <EmptyState title="No troubleshooting topics yet" />
    {:else}
      <div class="topics">
        {#each catalog.troubleshooting as topic (topic.id)}
          <div class="topic">
            <p class="topic-title msc2-type-card">{topic.title}</p>
            <p class="topic-summary msc2-type-body">{topic.summary}</p>
          </div>
        {/each}
      </div>
    {/if}
    <div class="footer single">
      <Button size="sm" variant="secondary" onclick={() => (stage = guideId ? 'reader' : 'picker')}>
        Back
      </Button>
    </div>
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .groups {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .group-label {
    margin: 0 0 4px;
  }
  .guide-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .guide-row {
    text-align: left;
    padding: 8px 10px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--msc2-text-secondary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .guide-row:hover {
    background: var(--msc2-hairline-subtle);
    color: var(--msc2-text-primary);
  }
  .steps {
    margin: 0;
    padding-left: 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--msc2-text-secondary);
    font-size: 13px;
    line-height: 1.55;
  }
  .topics {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .topic-title {
    color: var(--msc2-text-primary);
    margin: 0 0 2px;
  }
  .topic-summary {
    color: var(--msc2-text-secondary);
    margin: 0;
    line-height: 1.5;
  }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .footer.single {
    justify-content: flex-start;
  }
</style>
