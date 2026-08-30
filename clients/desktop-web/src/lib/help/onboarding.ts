import type { OnboardingStep } from './types';
import { KNOWN_TOUR_ANCHOR_IDS } from './tourAnchors';
import { writable } from 'svelte/store';

/** The current guided-tour card, shared with UI that needs tour-only limits. */
export const activeTourStep = writable<string | null>(null);

export type TourServerContext = {
  serverType: 'java' | 'bedrock';
  javaCategory: 'standard' | 'modded';
  javaFlavor: string;
  enableCrossPlay: boolean;
};

/** Current wizard choices used to omit tour cards that do not apply. */
export const tourServerContext = writable<TourServerContext | null>(null);

/** Becomes true after the wizard's real create operation reaches success. */
export const tourServerCreated = writable(false);

/**
 * MSC 2 doesn't yet cover every screen the oracle's tour walks through (no
 * AddServerWizardView port, no Packs tab -- see tourAnchors.ts). Keep only
 * steps that either need no anchor (welcome/done) or point at a real one in
 * this build, so the guided tour never spotlights UI that doesn't exist.
 */
export function applicableTourSteps(
  steps: readonly OnboardingStep[],
  context: TourServerContext | null = null,
): OnboardingStep[] {
  return steps.filter((step) => {
    if (step.anchor !== null && !KNOWN_TOUR_ANCHOR_IDS.has(step.anchor)) return false;
    if (step.id === 'xbox-broadcast' && context) {
      return (
        context.serverType === 'java' &&
        context.javaCategory === 'standard' &&
        context.javaFlavor !== 'vanilla' &&
        context.enableCrossPlay
      );
    }
    if (
      step.id === 'server-category' ||
      step.id === 'server-flavor' ||
      step.id === 'server-version'
    ) {
      return !context || context.serverType === 'java';
    }
    if (step.id === 'crossplay') {
      return (
        !context ||
        (context.serverType === 'java' &&
          context.javaCategory === 'standard' &&
          context.javaFlavor !== 'vanilla')
      );
    }
    if (step.id === 'add-ons' && context) {
      return context.serverType === 'java' && context.javaFlavor !== 'vanilla';
    }
    return true;
  });
}

export type FirstLaunchState = {
  setupComplete: boolean;
  tourComplete: boolean;
};

export type FirstLaunchStage = 'setup' | 'tour' | 'complete';

/** The presentation sequence is client-owned; the step text and ordering are agent data. */
export function firstLaunchStage(state: FirstLaunchState): FirstLaunchStage {
  if (!state.setupComplete) return 'setup';
  return state.tourComplete ? 'complete' : 'tour';
}

export function nextTourStep(
  steps: readonly OnboardingStep[],
  currentIndex: number,
  userActionCompleted = false,
): number {
  const current = steps[currentIndex];
  if (!current || (current.requiresUserAction && !userActionCompleted)) return currentIndex;
  return Math.min(currentIndex + 1, Math.max(0, steps.length - 1));
}
