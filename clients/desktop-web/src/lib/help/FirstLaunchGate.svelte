<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { ScreenApi } from '../sections/shared/types';
  import SetupIntro from './SetupIntro.svelte';
  import TourOverlay from './TourOverlay.svelte';
  import {
    applicableTourSteps,
    firstLaunchStage,
    nextTourStep,
    tourServerContext,
    type FirstLaunchState,
  } from './onboarding';
  import type { OnboardingGuide } from './types';

  export let api: ScreenApi;
  export let agentReady = false;

  let onboarding: OnboardingGuide | null = null;
  let ready = false;
  let tourIndex = 0;
  let state: FirstLaunchState = {
    setupComplete: false,
    tourComplete: false,
  };

  $: stage = firstLaunchStage(state);
  $: onboardingOpen = agentReady && ready && onboarding !== null && stage !== 'complete';
  $: tourSteps = onboarding ? applicableTourSteps(onboarding.steps, $tourServerContext) : [];

  function setPageScrollLocked(locked: boolean): void {
    if (typeof document === 'undefined' || !document.body) return;
    document.documentElement.classList.toggle('msc-onboarding-open', locked);
    document.body.classList.toggle('msc-onboarding-open', locked);
  }

  $: setPageScrollLocked(onboardingOpen);

  function readState(setupComplete: boolean): FirstLaunchState {
    if (!onboarding) return state;
    const tourComplete = localStorage.getItem(onboarding.reopen.persistenceKey) === 'true';
    return {
      setupComplete,
      tourComplete,
    };
  }
  function writeState(next: FirstLaunchState): void {
    state = next;
    if (!onboarding) return;
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
        const [loadedOnboarding, hostSetup] = await Promise.all([
          api.get<OnboardingGuide>('/v1/guides/onboarding'),
          api.get<{ complete: boolean }>('/v1/config/host-setup'),
        ]);
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

{#if agentReady && ready && onboarding}
  {#if stage === 'setup'}
    <div class="gate-backdrop" role="presentation">
      <section
        class="gate"
        aria-live="polite"
        aria-labelledby="first-launch-title"
        data-onboarding-stage={stage}
      >
        <SetupIntro {api} onComplete={() => writeState({ ...state, setupComplete: true })} />
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
</style>
