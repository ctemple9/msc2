<script lang="ts">
  // MSC 1 ServerHandbookView.swift / ConceptGuideView.swift /
  // RouterPortForwardGuideSheet.swift, rebuilt to the S0 disciplined system
  // (docs/msc2/antiAIslop.md) as one screen with a segmented switcher, since
  // MSC 2 has a single "Handbook" tab rather than three separate windows.
  // Content is served by the agent's help contract (GET /v1/help/*,
  // /v1/guides/*) -- this file carries presentation only, no hardcoded
  // divergent prose.
  import { onMount } from 'svelte';
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import SegmentedControl from '../../components/base/SegmentedControl.svelte';
  import {
    applicableTourSteps,
    firstLaunchStage,
    nextTourStep,
    type FirstLaunchState,
  } from '../../help/onboarding';
  import SetupIntro from '../../help/SetupIntro.svelte';
  import type {
    ConceptGuide,
    HelpCatalog,
    HelpTopic,
    OnboardingGuide,
    RouterGuideCatalog,
  } from '../../help/types';
  import type { ScreenProps } from '../shared/types';
  import { call } from '../shared/types';
  import ConceptGuidePanel from './ConceptGuidePanel.svelte';
  import HandbookBrowser from './HandbookBrowser.svelte';
  import RouterGuidePanel from './RouterGuidePanel.svelte';

  export let api: ScreenProps['api'] = undefined;
  export let hostId = 'local-agent';
  export let serverId = 'survival';

  type View = 'handbook' | 'concept' | 'router';
  const viewOptions: { value: View; label: string }[] = [
    { value: 'handbook', label: 'Handbook' },
    { value: 'concept', label: 'How MSC Works' },
    { value: 'router', label: 'Router Guide' },
  ];

  let activeView: View = 'handbook';
  let catalog: HelpCatalog = { topics: [] };
  let topic: HelpTopic | null = null;
  let topicId = '';
  let concept: ConceptGuide | null = null;
  let conceptPage = 0;
  let routerGuides: RouterGuideCatalog = { guides: [], troubleshooting: [], symptoms: [] };
  let onboarding: OnboardingGuide | null = null;
  let tourIndex = 0;
  let loaded = false;

  const defaultState: FirstLaunchState = {
    setupComplete: false,
    conceptGuideSeen: false,
    tourComplete: false,
  };
  let launchState = defaultState;

  $: tourSteps = onboarding ? applicableTourSteps(onboarding.steps) : [];
  $: launchStage = firstLaunchStage(launchState);

  function storageKey(name: string): string {
    return `msc.${name}`;
  }
  function readLaunchState(setupComplete: boolean): FirstLaunchState {
    if (typeof localStorage === 'undefined') return defaultState;
    const conceptGuideSeen = localStorage.getItem(storageKey('concept-guide-seen')) === 'true';
    const tourComplete = onboarding
      ? localStorage.getItem(onboarding.reopen.persistenceKey) === 'true'
      : false;
    return { setupComplete, conceptGuideSeen, tourComplete };
  }
  function saveLaunchState(next: FirstLaunchState): void {
    launchState = next;
    if (typeof localStorage === 'undefined' || !onboarding) return;
    localStorage.setItem(storageKey('concept-guide-seen'), String(next.conceptGuideSeen));
    localStorage.setItem(onboarding.reopen.persistenceKey, String(next.tourComplete));
  }

  onMount(async () => {
    catalog = await call(api, catalog, '/v1/help/catalog');
    concept = await call(api, concept, '/v1/guides/concept-guide');
    routerGuides = await call(api, routerGuides, '/v1/guides/router-catalog');
    onboarding = await call(api, onboarding, '/v1/guides/onboarding');
    const hostSetup = await call(api, { complete: false }, '/v1/config/host-setup');
    launchState = readLaunchState(hostSetup.complete);
    const requestedTopic =
      new URLSearchParams(window.location.search).get('topic') ?? 'handbook.overview';
    await selectTopic(requestedTopic);
    loaded = true;
  });

  async function selectTopic(id: string): Promise<void> {
    topicId = id;
    activeView = 'handbook';
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
    if (tourSteps.length === 0) {
      saveLaunchState({ ...launchState, tourComplete: true });
      return;
    }
    const next = nextTourStep(tourSteps, tourIndex, userActionCompleted);
    if (next === tourIndex && tourIndex === tourSteps.length - 1) {
      saveLaunchState({ ...launchState, tourComplete: true });
    } else {
      tourIndex = next;
    }
  }
  function skipTour(): void {
    saveLaunchState({ ...launchState, tourComplete: true });
  }
</script>

<div class="help-screen">
  {#if !loaded}
    <p class="msc2-type-meta" role="status">Loading guide content…</p>
  {:else}
    <section class="zone">
      <div class="section-header">
        <div class="overline">
          <Icon name="note" size={13} />
          <span class="msc2-type-overline">Guides</span>
        </div>
        <SegmentedControl
          options={viewOptions}
          value={activeView}
          onchange={(value) => (activeView = value as View)}
        />
      </div>
      <Card padding="18px">
        {#if activeView === 'handbook'}
          <HandbookBrowser {catalog} {topic} {topicId} onSelect={(id) => void selectTopic(id)} />
        {:else if activeView === 'concept'}
          <ConceptGuidePanel
            {concept}
            bind:page={conceptPage}
            {hostId}
            {serverId}
            onFinish={finishConcept}
            onOpenHandbook={() => void selectTopic('handbook.overview')}
          />
        {:else}
          <RouterGuidePanel {api} catalog={routerGuides} />
        {/if}
      </Card>
    </section>

    {#if onboarding}
      <section class="zone">
        <div class="section-header">
          <div class="overline">
            <Icon name="seal-check" size={13} />
            <span class="msc2-type-overline">Onboarding</span>
          </div>
          <Button size="sm" variant="secondary" onclick={restartTour}
            >{onboarding.reopen.label}</Button
          >
        </div>
        <Card padding="18px" as="section">
          {#if launchStage === 'setup'}
            <SetupIntro
              compact
              headingId="handbook-first-launch-title"
              {api}
              onComplete={completeSetup}
            />
          {:else if launchStage === 'concept-guide'}
            <p class="msc2-type-body muted">
              Open How MSC Works above and read through it, then continue.
            </p>
            <div class="onboarding-actions">
              <Button size="sm" variant="secondary" onclick={() => (activeView = 'concept')}>
                Open How MSC Works
              </Button>
              <Button size="sm" variant="primary" onclick={finishConcept}>Continue</Button>
            </div>
          {:else if launchStage === 'tour'}
            {@const step = tourSteps[tourIndex]}
            {#if step}
              <h3 class="msc2-type-section">{step.title}</h3>
              <p class="msc2-type-body muted">{step.body}</p>
              {#if !step.hideCard}
                <div class="tour-card msc2-type-meta">{step.actionLabel ?? 'Continue'}</div>
              {/if}
              <div class="onboarding-actions">
                <Button size="sm" variant="secondary" onclick={skipTour}
                  >{onboarding.skip.label}</Button
                >
                <Button
                  size="sm"
                  variant="primary"
                  onclick={() => advanceTour(step.requiresUserAction)}
                >
                  {step.requiresUserAction ? 'I did that' : (step.actionLabel ?? 'Continue')}
                </Button>
              </div>
            {/if}
          {:else}
            <p class="msc2-type-body muted">
              The Server Handbook stays available whenever you need it.
            </p>
            <Button
              size="sm"
              variant="secondary"
              onclick={() => void selectTopic('handbook.overview')}
            >
              Open Handbook
            </Button>
          {/if}
        </Card>
      </section>
    {/if}
  {/if}
</div>

<style>
  .help-screen {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .muted {
    color: var(--msc2-text-secondary);
    margin: 0 0 10px;
  }
  .onboarding-actions {
    display: flex;
    gap: 8px;
  }
  .tour-card {
    margin: 0 0 10px;
    padding: 10px 12px;
    border: 1px dashed var(--msc2-hairline);
    border-radius: var(--msc2-radius-2);
    color: var(--msc2-text-secondary);
  }
  h3 {
    margin: 0 0 4px;
  }
</style>
