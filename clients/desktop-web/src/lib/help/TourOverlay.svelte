<script lang="ts">
  // Real anchored spotlight tour, replacing FirstLaunchGate's old plain-card
  // placeholder tour stage. Ports MSC 1's OnboardingOverlayView shape and
  // behavior (spotlight cutout on the real control, dim everywhere else,
  // full-screen welcome/done cards, the "Got it" hide-card pattern for a
  // multi-field form step, non-blocking dim so the real target stays
  // clickable) onto web: a single fixed-position box-shadow spotlight
  // instead of SwiftUI's four-rect dim layer, and no dedicated
  // "Show tip"/"Skip tour" floating pill row keyed to a sheet frame -- both
  // are simplifications of the *mechanism*, not the behavior.
  //
  // docs/msc2/antiAIslop.md: no colored icon-in-circle for the welcome/done
  // cards, no accent-colored glow ring (a plain white ring reads the
  // highlight without spending the accent budget), buttons are the shared
  // Button component (solid neutral fill, no gradient/capsule).
  import { onDestroy, onMount } from 'svelte';
  import Button from '../components/base/Button.svelte';
  import { anchorFrames, ONBOARDING_ANCHOR_ACTION_EVENT, remeasureAll } from './tourAnchors';
  import { activeTourStep, tourServerCreated } from './onboarding';
  import type { OnboardingStep } from './types';

  export let steps: readonly OnboardingStep[] = [];
  export let stepIndex: number;
  export let skipLabel = 'Skip tour';
  export let onAdvance: (userActionCompleted?: boolean) => void;
  export let onSkip: () => void;
  export let onComplete: () => void = () => onSkip();

  const PAD = 10;
  const MARGIN = 16;
  const CARD_WIDTH = 320;
  const CARD_EST_HEIGHT = 190;
  const ACTION_ANCHORS: Readonly<Record<string, string>> = {
    'manage-servers': 'ob_manage_servers',
    'create-server': 'ob_create_server',
    'choose-path': 'ob_wizard_continue',
    'server-settings': 'ob_wizard_continue',
    'network-continue': 'ob_wizard_continue',
    'first-world': 'ob_wizard_continue',
    'add-ons': 'ob_wizard_continue',
  };
  const REVIEW_STEP_IDS = new Set(['server-settings', 'first-world', 'add-ons', 'create']);

  let cardHidden = false;
  let lastIndex = -1;
  let createCompletionHandled = false;
  $: if (stepIndex !== lastIndex) {
    cardHidden = false;
    lastIndex = stepIndex;
    createCompletionHandled = false;
  }

  let reduceMotion = false;
  onMount(() => {
    const mql = window.matchMedia('(prefers-reduced-motion: reduce)');
    reduceMotion = mql.matches;
    const onChange = (event: MediaQueryListEvent) => (reduceMotion = event.matches);
    mql.addEventListener('change', onChange);
    return () => mql.removeEventListener('change', onChange);
  });

  onMount(() => {
    const onAnchorAction = (event: Event) => {
      const anchorId = (event as CustomEvent<{ anchorId?: string }>).detail?.anchorId;
      if (step && ACTION_ANCHORS[step.id] === anchorId) {
        onAdvance(true);
      }
    };
    window.addEventListener(ONBOARDING_ANCHOR_ACTION_EVENT, onAnchorAction);
    return () => window.removeEventListener(ONBOARDING_ANCHOR_ACTION_EVENT, onAnchorAction);
  });

  $: activeTourStep.set(step?.id ?? null);
  // Wait for the real asynchronous create operation before ending this pass of
  // the tour, so the success page is visible and later setup cards do not take
  // over while the wizard is still creating the server.
  $: if (step?.id === 'create' && $tourServerCreated && !createCompletionHandled) {
    createCompletionHandled = true;
    onComplete();
  }
  onDestroy(() => activeTourStep.set(null));

  // Catches spotlights going stale when a layout change moves an anchor without
  // resizing it -- e.g. Sheet.svelte's vertically-centered scrim shifting every
  // child upward as the create-server form grows the sheet taller. ResizeObserver
  // (tourAnchors.ts) only fires on the tracked element's own size changing, so
  // this re-reads real positions every frame for as long as the tour is showing.
  onMount(() => {
    let frame = requestAnimationFrame(function tick() {
      remeasureAll();
      frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  });

  let viewportW = typeof window !== 'undefined' ? window.innerWidth : 1024;
  let viewportH = typeof window !== 'undefined' ? window.innerHeight : 768;
  function onResize(): void {
    viewportW = window.innerWidth;
    viewportH = window.innerHeight;
  }

  $: step = steps[stepIndex];
  // Choose Path describes the selection cards, but Continue is the actual
  // action that advances this card, so spotlight that real control instead.
  // Review Your Settings is a deliberate pause rather than a spotlight: its
  // oracle anchor has no one-to-one MSC 2 element, and the user needs the
  // unobstructed sheet to inspect the accumulated choices.
  $: spotlightAnchor =
    step?.id === 'choose-path'
      ? 'ob_wizard_continue'
      : step && REVIEW_STEP_IDS.has(step.id)
        ? null
        : step?.anchor;
  $: hasAnchor = spotlightAnchor !== null && spotlightAnchor !== undefined;
  $: anchorRect = hasAnchor && spotlightAnchor ? $anchorFrames[spotlightAnchor] : undefined;
  $: resolved = hasAnchor && !!anchorRect;
  $: blocking = !hasAnchor || !resolved;
  $: totalSteps = steps.length;

  $: spot =
    resolved && anchorRect
      ? {
          top: anchorRect.top - PAD,
          left: anchorRect.left - PAD,
          width: anchorRect.width + PAD * 2,
          height: anchorRect.height + PAD * 2,
        }
      : null;

  $: cardPos = (() => {
    if (!spot) return null;
    const spaceBelow = viewportH - (spot.top + spot.height + MARGIN);
    const spaceAbove = spot.top - MARGIN;
    const placeBelow = spaceBelow >= Math.min(CARD_EST_HEIGHT, spaceAbove);
    let top = placeBelow
      ? spot.top + spot.height + MARGIN
      : Math.max(MARGIN, spot.top - MARGIN - CARD_EST_HEIGHT);
    top = Math.max(MARGIN, Math.min(top, viewportH - CARD_EST_HEIGHT - MARGIN));
    let left = spot.left + spot.width / 2 - CARD_WIDTH / 2;
    left = Math.max(MARGIN, Math.min(left, viewportW - CARD_WIDTH - MARGIN));
    return { top, left };
  })();
</script>

<svelte:window on:resize={onResize} />

{#if step && !(REVIEW_STEP_IDS.has(step.id) && cardHidden)}
  <div class="tour-root" role="dialog" aria-modal="true" aria-live="polite">
    {#if !cardHidden}
      {#if blocking}
        <div class="dim-block" aria-hidden="true"></div>
      {:else if spot}
        <div
          class="spotlight"
          class:no-motion={reduceMotion}
          style="top:{spot.top}px; left:{spot.left}px; width:{spot.width}px; height:{spot.height}px;"
          aria-hidden="true"
        ></div>
      {/if}
    {/if}

    {#if cardHidden}
      <div class="hidden-pills">
        <Button variant="secondary" size="sm" onclick={() => (cardHidden = false)}>Show tip</Button>
        <Button variant="secondary" size="sm" onclick={onSkip}>{skipLabel}</Button>
      </div>
    {:else if REVIEW_STEP_IDS.has(step.id)}
      <div class="bookend-card">
        <h2>{step.title}</h2>
        <p>{step.body}</p>
        {#if step.id === 'server-settings'}
          <p class="hint review-hint">Click Continue once you have reviewed your settings.</p>
        {:else if step.id === 'add-ons'}
          <p class="hint review-hint">
            Nothing is required for a basic server. Feel free to browse or add files, then click
            Okay and Continue when you are ready.
          </p>
        {:else if step.id === 'first-world'}
          <p class="hint review-hint">
            Choose your world settings, then click Okay and Continue when you are ready.
          </p>
        {:else}
          <p class="hint review-hint">
            Review the summary and display name, then click Okay and Create Server when you are
            ready.
          </p>
        {/if}
        <div class="actions center">
          <Button variant="secondary" size="sm" onclick={onSkip}>{skipLabel}</Button>
          <Button variant="primary" onclick={() => (cardHidden = true)}>Okay</Button>
        </div>
      </div>
    {:else if !hasAnchor}
      <div class="bookend-card">
        <h2>{step.title}</h2>
        <p>{step.body}</p>
        <div class="actions center">
          {#if stepIndex === 0}
            <Button variant="secondary" size="sm" onclick={onSkip}>{skipLabel}</Button>
          {/if}
          <Button variant="primary" onclick={() => onAdvance(false)}
            >{step.actionLabel ?? 'Continue'}</Button
          >
        </div>
      </div>
    {:else if !resolved}
      <div class="bookend-card">
        <h2>{step.title}</h2>
        <p>{step.body}</p>
        <div class="actions center">
          <Button variant="secondary" size="sm" onclick={onSkip}>{skipLabel}</Button>
        </div>
      </div>
    {:else if cardPos}
      <div
        class="anchored-card"
        class:no-motion={reduceMotion}
        style="top:{cardPos.top}px; left:{cardPos.left}px; width:{CARD_WIDTH}px;"
      >
        <div class="progress">
          <span style="width:{((stepIndex + 1) / totalSteps) * 100}%"></span>
        </div>
        <h3>{step.title}</h3>
        <p>{step.body}</p>
        {#if step.id === 'continue-details'}
          <div class="actions">
            <Button variant="secondary" size="sm" onclick={onSkip}>Finish tour</Button>
            <Button variant="primary" size="sm" onclick={() => onAdvance(false)}>Continue</Button>
          </div>
        {:else if step.id === 'manage-servers' || step.id === 'create-server' || step.id === 'network-continue'}
          <div class="hint-row">
            <p class="hint">
              Click {step.id === 'manage-servers'
                ? 'Manage…'
                : step.id === 'create-server'
                  ? 'Add Server…'
                  : 'Continue'} to continue.
            </p>
          </div>
        {:else if step.id === 'choose-path'}
          <div class="hint-row">
            <p class="hint">Start Fresh is selected. Click Continue to continue.</p>
          </div>
        {:else if step.requiresUserAction}
          <!-- Checked before hideCard: this client confirms a required action by an
               explicit "I did that" (nextTourStep's userActionCompleted flag) rather than
               intercepting the real control's own click handler, so a step that is both
               hideCard and requiresUserAction (the create-server step) still has a way to
               advance -- hiding the card alone would otherwise dead-end the tour. -->
          <div class="hint-row">
            <p class="hint">Complete the highlighted action, then confirm below.</p>
            <Button variant="primary" size="sm" onclick={() => onAdvance(true)}>I did that</Button>
          </div>
        {:else if step.hideCard}
          <div class="hint-row">
            <p class="hint">Then continue the tour whenever you're ready.</p>
            <Button variant="primary" size="sm" onclick={() => (cardHidden = true)}>Got it</Button>
          </div>
        {:else}
          <div class="actions">
            <Button variant="secondary" size="sm" onclick={onSkip}>{skipLabel}</Button>
            <Button variant="primary" size="sm" onclick={() => onAdvance(false)}
              >{step.actionLabel ?? 'Next'}</Button
            >
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .tour-root {
    position: fixed;
    inset: 0;
    z-index: 500;
    pointer-events: none;
  }
  .dim-block {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.72);
    pointer-events: auto;
  }
  .spotlight {
    position: fixed;
    border: 2px solid rgba(255, 255, 255, 0.85);
    border-radius: var(--msc2-radius-2);
    box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.72);
    pointer-events: none;
    transition:
      top 200ms ease,
      left 200ms ease,
      width 200ms ease,
      height 200ms ease;
  }
  .spotlight.no-motion {
    transition: none;
  }
  .bookend-card,
  .anchored-card,
  .hidden-pills {
    pointer-events: auto;
  }
  .bookend-card {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(420px, calc(100vw - 48px));
    box-sizing: border-box;
    padding: 28px;
    border-radius: var(--msc2-radius-3);
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline);
    text-align: center;
  }
  .anchored-card {
    position: fixed;
    box-sizing: border-box;
    padding: 16px;
    border-radius: var(--msc2-radius-2);
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline);
    box-shadow: var(--msc2-shadow-float);
    transition:
      top 200ms ease,
      left 200ms ease;
  }
  .anchored-card.no-motion {
    transition: none;
  }
  .hidden-pills {
    position: fixed;
    top: 16px;
    right: 16px;
    display: flex;
    gap: 8px;
  }
  h2,
  h3 {
    margin: 0 0 8px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  h2 {
    font-size: 19px;
  }
  h3 {
    font-size: 15px;
  }
  p {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--msc2-text-secondary);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
  .actions.center {
    justify-content: center;
  }
  .hint-row {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    margin-top: 14px;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .progress {
    height: 3px;
    margin-bottom: 10px;
    border-radius: 2px;
    background: var(--msc2-hairline-subtle);
    overflow: hidden;
  }
  .progress span {
    display: block;
    height: 100%;
    background: var(--msc2-text-primary);
  }
</style>
