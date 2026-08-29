<script lang="ts">
  // MSC 1 RouterPortForwardGuidePicker.swift's global search + browse
  // funnel, rebuilt to S0 against the real content and routes P12.16a
  // restored (previously a flat picker/steps-list/topics-list stub built
  // before that content existed). Screen depth now matches the oracle's
  // 3-screen flow (picker -> guide reader -> troubleshooting); see
  // RouterGuideReader.svelte / RouterGuideTroubleshooting.svelte for the
  // other two.
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Field from '../../components/base/Field.svelte';
  import type { ScreenApi } from '../shared/types';
  import { call } from '../shared/types';
  import type { RouterGuideCatalog, RouterGuideSearchResult, RouterGuideSummary } from '../../help/types';
  import { ROUTER_CATEGORY_ORDER } from './catalogOrder';
  import RouterGuideReader from './RouterGuideReader.svelte';
  import RouterGuideTroubleshooting from './RouterGuideTroubleshooting.svelte';

  export let api: ScreenApi | undefined = undefined;
  export let catalog: RouterGuideCatalog = { guides: [], troubleshooting: [], symptoms: [] };

  type Stage = 'picker' | 'reader' | 'troubleshooting';
  let stage: Stage = 'picker';
  let guideId = '';
  let query = '';
  let searchResult: RouterGuideSearchResult | null = null;
  let searchToken = 0;

  const dedicatedCategories = new Set(['isp_gateway', 'retail_router', 'mesh_system']);

  $: groups = ROUTER_CATEGORY_ORDER.filter((category) => dedicatedCategories.has(category.slug))
    .map((category) => ({
      ...category,
      guides: catalog.guides
        .filter((guide) => guide.category === category.slug)
        .slice()
        .sort((a, b) => a.displayName.localeCompare(b.displayName)),
    }))
    .filter((group) => group.guides.length > 0);

  $: void runSearch(query);

  async function runSearch(value: string): Promise<void> {
    const trimmed = value.trim();
    if (!trimmed) {
      searchResult = null;
      return;
    }
    const token = ++searchToken;
    const result = await call<RouterGuideSearchResult | null>(
      api,
      null,
      `/v1/guides/router/search?q=${encodeURIComponent(trimmed)}`,
    );
    if (token === searchToken) searchResult = result;
  }

  function openGuide(id: string): void {
    guideId = id;
    stage = 'reader';
  }
  function openGenericGuide(): void {
    const generic = catalog.guides.find((guide) => guide.family === 'generic_router') ?? catalog.guides[0];
    if (generic) openGuide(generic.id);
  }
  function backToPicker(): void {
    stage = 'picker';
    guideId = '';
    query = '';
    searchResult = null;
  }

  function summaryLabel(guide: RouterGuideSummary): string {
    return guide.providerDisplayName ?? guide.deviceDisplayName ?? guide.category;
  }
</script>

{#snippet guideRow(guide: RouterGuideSummary, last: boolean, onSelect: () => void)}
  <button type="button" class="guide-row" class:last onclick={onSelect}>
    <span class="text">
      <span class="msc2-type-card">{guide.displayName}</span>
      <span class="msc2-type-meta">{summaryLabel(guide)}</span>
    </span>
    <span class="chevron" aria-hidden="true">›</span>
  </button>
{/snippet}

<div class="panel">
  {#if stage === 'reader' && guideId}
    <RouterGuideReader
      {api}
      {guideId}
      onBack={backToPicker}
      onTroubleshoot={() => (stage = 'troubleshooting')}
    />
  {:else if stage === 'troubleshooting'}
    <RouterGuideTroubleshooting
      {api}
      symptoms={catalog.symptoms}
      onBack={() => (stage = guideId ? 'reader' : 'picker')}
      onOpenGuide={openGuide}
    />
  {:else if catalog.guides.length === 0}
    <EmptyState
      title="Router guides are unavailable"
      message="This agent has not served the router catalog yet."
    />
  {:else}
    <p class="msc2-type-overline">Step 1 of 3 — Find your router</p>
    <p class="msc2-type-body muted">
      Search across supported providers, routers, and mesh systems, or browse the inventory below.
    </p>

    <Field placeholder="Search by provider, brand, model, or product line…" bind:value={query} />

    <button type="button" class="generic-card" onclick={openGenericGuide}>
      <span class="msc2-type-card">I don't know my router</span>
      <span class="msc2-type-meta"
        >Open the generic guide and continue with the broadest supported path.</span
      >
    </button>

    {#if query.trim()}
      {#if !searchResult}
        <p class="msc2-type-meta">Searching…</p>
      {:else if searchResult.candidates.length === 0}
        {#if searchResult.suggestedFallbackGuide}
          {@const fallback = searchResult.suggestedFallbackGuide}
          <EmptyState title="No supported guide matched that search.">
            <Button size="sm" variant="secondary" slot="action" onclick={() => openGuide(fallback.id)}>
              Open closest guide: {fallback.displayName}
            </Button>
          </EmptyState>
        {:else}
          <EmptyState title="No supported guide matched that search." />
        {/if}
      {:else}
        <div class="results">
          {#if !searchResult.matchedDirectGuide && searchResult.suggestedFallbackGuide}
            {@const fallback = searchResult.suggestedFallbackGuide}
            <p class="msc2-type-meta banner">
              No specific guide for this family yet — showing the closest match.
            </p>
            <Card padding="0">
              {@render guideRow(fallback, true, () => openGuide(fallback.id))}
            </Card>
          {/if}
          <p class="msc2-type-overline">
            {searchResult.isAmbiguous ? 'Closest available guides' : 'Results'}
          </p>
          <Card padding="0">
            {#each searchResult.candidates.slice(0, 8) as candidate, index (candidate.guide.id)}
              {@render guideRow(
                candidate.guide,
                index === Math.min(searchResult.candidates.length, 8) - 1,
                () => openGuide(candidate.guide.id),
              )}
            {/each}
          </Card>
        </div>
      {/if}
    {:else}
      {#each groups as group (group.slug)}
        <div class="group">
          <p class="msc2-type-overline">{group.label}</p>
          <Card padding="0">
            {#each group.guides as guide, index (guide.id)}
              {@render guideRow(guide, index === group.guides.length - 1, () => openGuide(guide.id))}
            {/each}
          </Card>
        </div>
      {/each}
    {/if}
  {/if}
</div>

<style>
  /* Router Guide runs a size step above the shared type scale -- Cameron's
     own visual-review call, this component only. */
  .panel :global(.msc2-type-overline) {
    font-size: 11px;
  }
  .panel :global(.msc2-type-body),
  .panel :global(.msc2-type-card) {
    font-size: 15px;
  }
  .panel :global(.msc2-type-meta) {
    font-size: 13px;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .muted {
    color: var(--msc2-text-secondary);
  }
  .generic-card {
    display: flex;
    flex-direction: column;
    gap: 3px;
    text-align: left;
    padding: 12px 14px;
    border: 1px solid var(--msc2-hairline);
    border-radius: var(--msc2-radius-2);
    background: transparent;
    color: var(--msc2-text-primary);
    font: inherit;
    cursor: pointer;
  }
  .generic-card:hover {
    background: var(--msc2-hairline-subtle);
  }
  .results,
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .banner {
    color: var(--msc2-status-warn);
  }
  .guide-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    width: 100%;
    padding: 11px 14px;
    border: none;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
    background: transparent;
    color: var(--msc2-text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .guide-row.last {
    border-bottom: none;
  }
  .guide-row:hover {
    background: var(--msc2-hairline-subtle);
  }
  .guide-row .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .guide-row .chevron {
    flex: none;
    color: var(--msc2-text-tertiary);
  }
</style>
