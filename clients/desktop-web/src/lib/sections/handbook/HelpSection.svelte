<script lang="ts">
  import { onMount } from 'svelte';
  import HelpLink from '../../help/HelpLink.svelte';
  import { renderMarkdown } from '../../help/markdown';
  import { firstLaunchStage, nextTourStep, type FirstLaunchState } from '../../help/onboarding';
  import type {
    ConceptGuide,
    HelpCatalog,
    HelpTopic,
    OnboardingGuide,
    RouterGuideCatalog,
  } from '../../help/types';
  import ActionButton from '../../components/ActionButton.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { ScreenProps } from '../shared/types';
  import { call } from '../shared/types';

  export let api: ScreenProps['api'] = undefined;
  export let hostId = 'local-agent';
  export let serverId = 'survival';

  let catalog: HelpCatalog = { topics: [] };
  let topic: HelpTopic | null = null;
  let topicId = '';
  let concept: ConceptGuide | null = null;
  let routerGuides: RouterGuideCatalog = { guides: [], troubleshooting: [] };
  let onboarding: OnboardingGuide | null = null;
  let guidePage = 0;
  let tourIndex = 0;
  let loaded = false;

  const defaultState: FirstLaunchState = {
    setupComplete: false,
    conceptGuideSeen: false,
    tourComplete: false,
  };
  let launchState = defaultState;

  function storageKey(name: string): string {
    return `msc.${name}`;
  }
  function readLaunchState(): FirstLaunchState {
    if (typeof localStorage === 'undefined') return defaultState;
    return {
      setupComplete: localStorage.getItem(storageKey('setup-complete')) === 'true',
      conceptGuideSeen: localStorage.getItem(storageKey('concept-guide-seen')) === 'true',
      tourComplete: onboarding
        ? localStorage.getItem(onboarding.reopen.persistenceKey) === 'true'
        : false,
    };
  }
  function saveLaunchState(next: FirstLaunchState): void {
    launchState = next;
    if (typeof localStorage === 'undefined' || !onboarding) return;
    localStorage.setItem(storageKey('setup-complete'), String(next.setupComplete));
    localStorage.setItem(storageKey('concept-guide-seen'), String(next.conceptGuideSeen));
    localStorage.setItem(onboarding.reopen.persistenceKey, String(next.tourComplete));
  }
  $: launchStage = firstLaunchStage(launchState);

  onMount(async () => {
    catalog = await call(api, catalog, '/v1/help/catalog');
    concept = await call(api, concept, '/v1/guides/concept-guide');
    routerGuides = await call(api, routerGuides, '/v1/guides/router-catalog');
    onboarding = await call(api, onboarding, '/v1/guides/onboarding');
    launchState = readLaunchState();
    topicId =
      new URLSearchParams(window.location.search).get('topic') ?? catalog.topics[0]?.id ?? '';
    await selectTopic(topicId);
    loaded = true;
  });

  async function selectTopic(id: string): Promise<void> {
    topicId = id;
    topic = await call<HelpTopic | null>(api, null, `/v1/help/${encodeURIComponent(id)}`);
  }
  function completeSetup(): void {
    saveLaunchState({ ...launchState, setupComplete: true });
  }
  function finishConcept(): void {
    saveLaunchState({ ...launchState, conceptGuideSeen: true });
  }
  function restartTour(): void {
    window.dispatchEvent(new CustomEvent('msc:restart-tour'));
  }
  function advanceTour(userActionCompleted = false): void {
    if (!onboarding) return;
    const next = nextTourStep(onboarding.steps, tourIndex, userActionCompleted);
    if (next === tourIndex && tourIndex === onboarding.steps.length - 1) {
      saveLaunchState({ ...launchState, tourComplete: true });
    } else {
      tourIndex = next;
    }
  }
  function skipTour(): void {
    saveLaunchState({ ...launchState, tourComplete: true });
  }
</script>

<div class="screen help-screen">
  <ScreenHeader
    eyebrow="Server Handbook"
    title="Help and guides"
    description="Topics are loaded from the selected agent; this reader carries presentation only."
  />

  {#if !loaded}
    <p class="muted" role="status">Loading guide content…</p>
  {:else}
    <div class="guide-layout">
      <aside class="screen-card topic-list" aria-label="Handbook topics">
        <h3>Handbook</h3>
        {#each catalog.topics as item (item.id)}
          <button
            class:active={item.id === topicId}
            type="button"
            onclick={() => void selectTopic(item.id)}>{item.title}</button
          >
        {/each}
      </aside>
      <article class="screen-card topic-reader">
        {#if topic}
          <p class="eyebrow">{topic.category ?? topic.kind}</p>
          <h2>{topic.title}</h2>
          {#if topic.analogy}<p class="analogy">Think of it like this: {topic.analogy}</p>{/if}
          <div class="markdown" data-safe-markdown="true">
            {@html renderMarkdown(topic.markdown)}
          </div>
          {#if topic.relatedIds.length}
            <h3>Related topics</h3>
            <div class="related-topics">
              {#each topic.relatedIds as relatedId (relatedId)}
                <button type="button" onclick={() => void selectTopic(relatedId)}
                  >{relatedId}</button
                >
              {/each}
            </div>
          {/if}
        {:else}
          <h2>That topic is not available on this agent</h2>
          <p class="muted">The link is preserved so a newer agent can provide this topic later.</p>
          <code>{topicId}</code>
        {/if}
      </article>
    </div>

    <section class="screen-card">
      <div class="screen-card-header">
        <div>
          <p class="eyebrow">Concept Guide</p>
          <h3>{concept?.pages[guidePage]?.title ?? 'Guide unavailable'}</h3>
        </div>
        <span class="metric-label"
          >{concept ? `${guidePage + 1} / ${concept.pages.length}` : ''}</span
        >
      </div>
      {#if concept?.pages[guidePage]}
        <p>{concept.pages[guidePage].body}</p>
        <HelpLink helpId={concept.pages[guidePage].helpId} {hostId} {serverId} />
        <div class="screen-actions">
          <ActionButton
            kind="quiet"
            label="Previous page"
            disabled={guidePage === 0}
            onclick={() => (guidePage -= 1)}>Previous</ActionButton
          ><ActionButton
            label={guidePage === concept.pages.length - 1 ? 'Finish Concept Guide' : 'Next page'}
            onclick={() =>
              guidePage === concept!.pages.length - 1 ? finishConcept() : (guidePage += 1)}
            >Next</ActionButton
          >
        </div>
      {/if}
    </section>

    <section class="screen-card">
      <p class="eyebrow">Router guides</p>
      <div class="screen-grid">
        {#each routerGuides.guides as guide (guide.id)}<details>
            <summary>{guide.displayName}</summary>
            <ol>
              {#each guide.steps as step}<li>{step}</li>{/each}
            </ol>
          </details>{/each}
      </div>
    </section>

    {#if onboarding}
      <section class="screen-card first-launch" data-onboarding-stage={launchStage}>
        <div class="screen-card-header">
          <div>
            <p class="eyebrow">First launch</p>
            <h3>
              {launchStage === 'setup'
                ? 'Set up MSC'
                : launchStage === 'concept-guide'
                  ? 'Read the Concept Guide'
                  : launchStage === 'tour'
                    ? onboarding.steps[tourIndex]?.title
                    : 'Tour complete'}
            </h3>
          </div>
          <ActionButton kind="quiet" label={onboarding.reopen.label} onclick={restartTour}
            >{onboarding.reopen.label}</ActionButton
          >
        </div>
        {#if launchStage === 'setup'}
          <p class="muted">
            Finish the setup sheet before the Concept Guide and guided tour begin.
          </p>
          <ActionButton label="Finish setup" onclick={completeSetup}>Finish setup</ActionButton>
        {:else if launchStage === 'concept-guide'}
          <p class="muted">Read the Concept Guide above, then continue to the guided tour.</p>
          <ActionButton label="Continue to tour" onclick={finishConcept}>Continue</ActionButton>
        {:else if launchStage === 'tour'}
          {@const step = onboarding.steps[tourIndex]}
          <p>{step.body}</p>
          {#if step.anchor}<p class="anchor" data-onboarding-anchor={step.anchor}>
              Anchor: {step.anchor}
            </p>{/if}
          {#if !step.hideCard}<div class="tour-card">{step.actionLabel ?? 'Continue'}</div>{/if}
          <div class="screen-actions">
            <ActionButton kind="quiet" label={onboarding.skip.label} onclick={skipTour}
              >{onboarding.skip.label}</ActionButton
            ><ActionButton
              label={step.requiresUserAction ? 'I did that' : (step.actionLabel ?? 'Continue')}
              onclick={() => advanceTour(step.requiresUserAction)}
              >{step.requiresUserAction
                ? 'I did that'
                : (step.actionLabel ?? 'Continue')}</ActionButton
            >
          </div>
        {:else}
          <p class="muted">The Server Handbook stays available whenever you need it.</p>
          <ActionButton
            label="Open Handbook overview"
            onclick={() => void selectTopic('handbook.overview')}>Open Handbook</ActionButton
          >
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .guide-layout {
    display: grid;
    grid-template-columns: minmax(12rem, 0.4fr) minmax(0, 1fr);
    gap: 1rem;
  }
  .topic-list {
    display: grid;
    align-content: start;
    gap: 0.25rem;
  }
  .topic-list button,
  .related-topics button {
    border: 0;
    border-radius: var(--msc-radius-sm);
    padding: 0.5rem;
    color: var(--msc-muted);
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .topic-list button.active,
  .topic-list button:hover,
  .related-topics button:hover {
    color: var(--msc-text);
    background: rgba(143, 227, 207, 0.11);
  }
  .topic-reader h2 {
    margin-top: 0;
  }
  .analogy {
    color: var(--msc-accent) !important;
    font-style: italic;
  }
  .markdown :global(p),
  .markdown :global(li) {
    line-height: 1.6;
  }
  .markdown :global(a) {
    color: var(--msc-accent);
  }
  .related-topics {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }
  .screen-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 1rem;
  }
  .first-launch {
    scroll-margin-top: 1rem;
  }
  .anchor {
    color: var(--msc-subtle);
    font-size: 0.8rem;
  }
  .tour-card {
    margin: 0.75rem 0;
    padding: 0.75rem;
    border: 1px dashed var(--msc-border);
    border-radius: var(--msc-radius-sm);
    color: var(--msc-muted);
  }
  @media (max-width: 759px) {
    .guide-layout {
      grid-template-columns: 1fr;
    }
    .topic-list {
      max-height: 13rem;
      overflow: auto;
    }
  }
</style>
