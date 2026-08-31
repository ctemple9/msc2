import type { SectionDescriptor } from './types';

const PRELOAD_TABS_KEY = 'msc2.preload-tabs';
const PRELOAD_DELAY_MS = 750;

/** Preloading is a client preference; an unset preference keeps the fast path on. */
export function readTabPreloadPreference(): boolean {
  if (typeof localStorage === 'undefined') return true;
  return localStorage.getItem(PRELOAD_TABS_KEY) !== 'false';
}

export function setTabPreloadPreference(enabled: boolean): void {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(PRELOAD_TABS_KEY, String(enabled));
}

/**
 * Start loading tab code after the first screen has had time to paint. The
 * returned cancellation function is used when the preference or host changes.
 * Imports are intentionally caught: a failed background fetch must remain a
 * normal retry on the next tab click.
 */
export function scheduleTabPreload(descriptors: readonly SectionDescriptor[]): () => void {
  let cancelled = false;
  const preload = (): void => {
    if (cancelled) return;
    for (const descriptor of descriptors) {
      void descriptor.load().catch(() => undefined);
    }
  };

  if (typeof window === 'undefined') return () => undefined;
  const timeout = window.setTimeout(preload, PRELOAD_DELAY_MS);
  return () => {
    cancelled = true;
    window.clearTimeout(timeout);
  };
}
