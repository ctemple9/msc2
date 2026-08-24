import type { OnboardingStep } from './types';

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
