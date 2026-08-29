<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { ScreenApi } from '../sections/shared/types';
  import ActionButton from '../components/ActionButton.svelte';
  import SetupIntro from './SetupIntro.svelte';
  import TourOverlay from './TourOverlay.svelte';
  import {
    applicableTourSteps,
    firstLaunchStage,
    nextTourStep,
    tourServerContext,
    type FirstLaunchState,
  } from './onboarding';
  import type { ConceptGuide, OnboardingGuide } from './types';

  export let api: ScreenApi;
  export let agentReady = false;

  let concept: ConceptGuide | null = null;
  let onboarding: OnboardingGuide | null = null;
  let ready = false;
  let conceptPage = 0;
  let tourIndex = 0;
  let state: FirstLaunchState = {
    setupComplete: false,
    conceptGuideSeen: false,
    tourComplete: false,
  };

  $: stage = firstLaunchStage(state);
  $: onboardingOpen =
    agentReady && ready && onboarding !== null && concept !== null && stage !== 'complete';
  $: tourSteps = onboarding ? applicableTourSteps(onboarding.steps, $tourServerContext) : [];

  function setPageScrollLocked(locked: boolean): void {
    if (typeof document === 'undefined' || !document.body) return;
    document.documentElement.classList.toggle('msc-onboarding-open', locked);
    document.body.classList.toggle('msc-onboarding-open', locked);
  }

  $: setPageScrollLocked(onboardingOpen);

  function readState(setupComplete: boolean): FirstLaunchState {
    if (!onboarding) return state;
    const conceptGuideSeen = localStorage.getItem('msc.concept-guide-seen') === 'true';
    const tourComplete = localStorage.getItem(onboarding.reopen.persistenceKey) === 'true';
    return {
      setupComplete,
      conceptGuideSeen,
      tourComplete,
    };
  }
  function writeState(next: FirstLaunchState): void {
    state = next;
    if (!onboarding) return;
    localStorage.setItem('msc.concept-guide-seen', String(next.conceptGuideSeen));
    localStorage.setItem(onboarding.reopen.persistenceKey, String(next.tourComplete));
  }
  function restart(): void {
    tourIndex = 0;
    writeState({ ...state, tourComplete: false });
  }
  function advance(userActionCompleted = false): void {
    if (!onboarding) return;
    if (tourSteps.length === 0) {
      writeState({ ...state, tourComplete: true });
      return;
    }
    const next = nextTourStep(tourSteps, tourIndex, userActionCompleted);
    if (next === tourIndex && tourIndex === tourSteps.length - 1) {
      writeState({ ...state, tourComplete: true });
    } else {
      tourIndex = next;
    }
  }

  onMount(() => {
    if (!agentReady) return;
    const load = async () => {
      try {
        const [loadedConcept, loadedOnboarding, hostSetup] = await Promise.all([
          api.get<ConceptGuide>('/v1/guides/concept-guide'),
          api.get<OnboardingGuide>('/v1/guides/onboarding'),
          api.get<{ complete: boolean }>('/v1/config/host-setup'),
        ]);
        concept = loadedConcept;
        onboarding = loadedOnboarding;
        state = readState(hostSetup.complete);
      } catch {
        // A pre-P11.24 agent does not serve guide data yet. The regular client
        // remains usable rather than inventing local onboarding prose.
      } finally {
        ready = true;
      }
    };
    void load();
    const onRestart = () => restart();
    window.addEventListener('msc:restart-tour', onRestart);
    return () => window.removeEventListener('msc:restart-tour', onRestart);
  });

  onDestroy(() => setPageScrollLocked(false));
</script>

{#if agentReady && ready && onboarding && concept}
  {#if stage === 'setup' || stage === 'concept-guide'}
    <div class="gate-backdrop" role="presentation">
      <section
        class="gate"
        aria-live="polite"
        aria-labelledby="first-launch-title"
        data-onboarding-stage={stage}
      >
        {#if stage === 'setup'}
          <SetupIntro {api} onComplete={() => writeState({ ...state, setupComplete: true })} />
        {:else}
          {@const page = concept.pages[conceptPage]}
          <p class="eyebrow">{page.eyebrow}</p>
          <h2 id="first-launch-title">{page.title}</h2>
          <p>{page.body}</p>
          <div class="actions">
            <ActionButton
              kind="quiet"
              label="Previous Concept Guide page"
              disabled={conceptPage === 0}
              onclick={() => (conceptPage -= 1)}>Previous</ActionButton
            ><ActionButton
              label={conceptPage === concept.pages.length - 1
                ? 'Continue to tour'
                : 'Next Concept Guide page'}
              onclick={() =>
                conceptPage === concept!.pages.length - 1
                  ? writeState({ ...state, conceptGuideSeen: true })
                  : (conceptPage += 1)}
              >{conceptPage === concept.pages.length - 1 ? 'Continue' : 'Next'}</ActionButton
            >
          </div>
        {/if}
      </section>
    </div>
  {:else if stage === 'tour'}
    <TourOverlay
      steps={tourSteps}
      stepIndex={tourIndex}
      skipLabel={onboarding.skip.label}
      onAdvance={advance}
      onSkip={() => writeState({ ...state, tourComplete: true })}
      onComplete={() => writeState({ ...state, tourComplete: true })}
    />
  {/if}
{/if}

<style>
  .gate-backdrop {
    position: fixed;
    z-index: 40;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(7, 12, 16, 0.76);
  }
  .gate {
    width: min(100%, 38rem);
    max-height: calc(100vh - 2rem);
    padding: 1.5rem;
    overflow-y: auto;
    border: 1px solid var(--msc-border);
    border-radius: var(--msc-radius-lg);
    background: var(--msc-surface-raised);
    box-shadow: var(--msc-shadow);
    scrollbar-width: none;
  }
  .gate::-webkit-scrollbar {
    display: none;
    width: 0;
  }
  :global(html.msc-onboarding-open),
  :global(body.msc-onboarding-open) {
    overflow: hidden;
  }
  .gate h2 {
    margin: 0;
  }
  .gate p {
    color: var(--msc-muted);
    line-height: 1.6;
  }
  .eyebrow {
    color: var(--msc-accent) !important;
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 1rem;
  }
</style>
