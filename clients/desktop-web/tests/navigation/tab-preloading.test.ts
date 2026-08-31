import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  readTabPreloadPreference,
  scheduleTabPreload,
  setTabPreloadPreference,
} from '../../src/lib/navigation/tabPreloading';

describe('tab preloading preference', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('defaults to enabled and persists an explicit choice', () => {
    const storage = new Map<string, string>();
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
    });

    expect(readTabPreloadPreference()).toBe(true);
    setTabPreloadPreference(false);
    expect(readTabPreloadPreference()).toBe(false);
    setTabPreloadPreference(true);
    expect(readTabPreloadPreference()).toBe(true);
  });

  it('preloads after the initial paint delay and can be cancelled', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('window', {
      setTimeout,
      clearTimeout,
    });
    const load = vi.fn(() => Promise.resolve({ default: {} }));
    const cancel = scheduleTabPreload([
      { id: 'home', label: 'Home', segment: 'home', scope: 'server', load },
    ]);

    vi.advanceTimersByTime(749);
    expect(load).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(load).toHaveBeenCalledTimes(1);

    const secondLoad = vi.fn(() => Promise.resolve({ default: {} }));
    const cancelSecond = scheduleTabPreload([
      {
        id: 'players-online',
        label: 'Players',
        segment: 'players-online',
        scope: 'server',
        load: secondLoad,
      },
    ]);
    cancelSecond();
    vi.advanceTimersByTime(750);
    expect(secondLoad).not.toHaveBeenCalled();
    cancel();
    await Promise.resolve();
  });
});
