import type { OnboardingStep } from './types';
import { KNOWN_TOUR_ANCHOR_IDS } from './tourAnchors';
import { writable } from 'svelte/store';

/** The current guided-tour card, shared with UI that needs tour-only limits. */
export const activeTourStep = writable<string | null>(null);

/**
 * MSC 2 doesn't yet cover every screen the oracle's tour walks through (no
 * AddServerWizardView port, no Packs tab -- see tourAnchors.ts). Keep only
 * steps that either need no anchor (welcome/done) or point at a real one in
 * this build, so the guided tour never spotlights UI that doesn't exist.
 */
export function applicableTourSteps(steps: readonly OnboardingStep[]): OnboardingStep[] {
  return steps.filter((step) => step.anchor === null || KNOWN_TOUR_ANCHOR_IDS.has(step.anchor));
}

export type FirstLaunchState = {
  setupComplete: boolean;
  conceptGuideSeen: boolean;
  tourComplete: boolean;
};

export type FirstLaunchStage = 'setup' | 'concept-guide' | 'tour' | 'complete';

/** The presentation sequence is client-owned; the step text and ordering are agent data. */
export function firstLaunchStage(state: FirstLaunchState): FirstLaunchStage {
  if (!state.setupComplete) return 'setup';
  if (!state.conceptGuideSeen) return 'concept-guide';
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
