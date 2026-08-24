<script lang="ts">
  import { onMount } from 'svelte';
  import type { ScreenApi } from '../sections/shared/types';
  import ActionButton from '../components/ActionButton.svelte';
  import SetupIntro from './SetupIntro.svelte';
  import { firstLaunchStage, nextTourStep, type FirstLaunchState } from './onboarding';
  import type { ConceptGuide, OnboardingGuide } from './types';

  export let api: ScreenApi;

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

  function readState(): FirstLaunchState {
    if (!onboarding) return state;
    return {
      setupComplete: localStorage.getItem('msc.setup-complete') === 'true',
      conceptGuideSeen: localStorage.getItem('msc.concept-guide-seen') === 'true',
      tourComplete: localStorage.getItem(onboarding.reopen.persistenceKey) === 'true',
    };
  }
  function writeState(next: FirstLaunchState): void {
    state = next;
    if (!onboarding) return;
    localStorage.setItem('msc.setup-complete', String(next.setupComplete));
    localStorage.setItem('msc.concept-guide-seen', String(next.conceptGuideSeen));
    localStorage.setItem(onboarding.reopen.persistenceKey, String(next.tourComplete));
  }
  function restart(): void {
    tourIndex = 0;
    writeState({ ...state, tourComplete: false });
  }
  function advance(userActionCompleted = false): void {
    if (!onboarding) return;
    const next = nextTourStep(onboarding.steps, tourIndex, userActionCompleted);
    if (next === tourIndex && tourIndex === onboarding.steps.length - 1) {
      writeState({ ...state, tourComplete: true });
    } else {
      tourIndex = next;
    }
  }

  onMount(() => {
    const load = async () => {
      try {
        [concept, onboarding] = await Promise.all([
          api.get<ConceptGuide>('/v1/guides/concept-guide'),
          api.get<OnboardingGuide>('/v1/guides/onboarding'),
        ]);
        state = readState();
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
</script>

{#if ready && onboarding && concept && stage !== 'complete'}
  <div class="gate-backdrop" role="presentation">
    <section
      class="gate"
      aria-live="polite"
      aria-labelledby="first-launch-title"
      data-onboarding-stage={stage}
    >
      {#if stage === 'setup'}
        <SetupIntro onComplete={() => writeState({ ...state, setupComplete: true })} />
      {:else if stage === 'concept-guide'}
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
      {:else}
        {@const step = onboarding.steps[tourIndex]}
        <p class="eyebrow">Guided tour · {tourIndex + 1} of {onboarding.steps.length}</p>
        <h2 id="first-launch-title">{step.title}</h2>
        <p>{step.body}</p>
        {#if step.anchor}<p class="anchor" data-onboarding-anchor={step.anchor}>
            Continue after completing the highlighted action.
          </p>{/if}
        {#if !step.hideCard}<div class="tour-card">{step.actionLabel ?? 'Continue'}</div>{/if}
        <div class="actions">
          <ActionButton
            kind="quiet"
            label={onboarding.skip.label}
            onclick={() => writeState({ ...state, tourComplete: true })}
            >{onboarding.skip.label}</ActionButton
          ><ActionButton
            label={step.requiresUserAction ? 'I did that' : (step.actionLabel ?? 'Continue')}
            onclick={() => advance(step.requiresUserAction)}
            >{step.requiresUserAction
              ? 'I did that'
              : (step.actionLabel ?? 'Continue')}</ActionButton
          >
        </div>
      {/if}
    </section>
  </div>
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
    padding: 1.5rem;
    border: 1px solid var(--msc-border);
    border-radius: var(--msc-radius-lg);
    background: var(--msc-surface-raised);
    box-shadow: var(--msc-shadow);
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
  .anchor {
    color: var(--msc-subtle) !important;
    font-size: 0.82rem;
  }
  .tour-card {
    margin-top: 0.75rem;
    padding: 0.75rem;
    border: 1px dashed var(--msc-border);
    border-radius: var(--msc-radius-sm);
  }
</style>
