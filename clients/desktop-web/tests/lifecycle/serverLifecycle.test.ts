import { describe, expect, it } from 'vitest';
import { waitForRunningState } from '../../src/lib/lifecycle/serverLifecycle';

describe('server lifecycle status polling', () => {
  it('waits for stop to be reported instead of trusting the first response', async () => {
    const snapshots = [{ running: true }, { running: true }, { running: false }];
    const seen: boolean[] = [];

    const result = await waitForRunningState(
      async () => {
        const snapshot = snapshots.shift();
        if (!snapshot) throw new Error('read called too many times');
        seen.push(snapshot.running);
        return snapshot;
      },
      false,
      { intervalMs: 1, sleep: async () => undefined },
    );

    expect(seen).toEqual([true, true, false]);
    expect(result.running).toBe(false);
  });

  it('returns the latest state if the agent exceeds the polling timeout', async () => {
    let clock = 0;
    const result = await waitForRunningState(async () => ({ running: false }), true, {
      intervalMs: 10,
      timeoutMs: 20,
      now: () => clock,
      sleep: async (milliseconds) => {
        clock += milliseconds;
      },
    });

    expect(result.running).toBe(false);
    expect(clock).toBe(20);
  });
});
