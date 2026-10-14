import { describe, expect, it } from 'vitest';
import { ServerStatusMonitor, waitForRunningState } from '../../src/lib/lifecycle/serverLifecycle';

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

describe('continuous server status reconciliation', () => {
  it('observes a completed shutdown after the initial wait has expired', async () => {
    let clock = 0;
    let displayed: { running: boolean } = await waitForRunningState(
      async () => ({ running: true }),
      false,
      {
        timeoutMs: 10_000,
        intervalMs: 1000,
        now: () => clock,
        sleep: async (milliseconds) => {
          clock += milliseconds;
        },
      },
    );
    expect(displayed.running).toBe(true);
    const monitor = new ServerStatusMonitor<{ running: boolean }>();
    const apply = (next: { running: boolean }) => {
      displayed = next;
    };
    const failed = () => {
      throw new Error('unexpected status failure');
    };
    await monitor.poll(async () => ({ running: true }), apply, failed);
    await monitor.poll(async () => ({ running: false }), apply, failed);
    expect(displayed.running).toBe(false);
  });

  it('coalesces slow reads and ignores a response from a previous host context', async () => {
    const monitor = new ServerStatusMonitor<{ running: boolean }>();
    const observed: boolean[] = [];
    let finishOld!: (value: { running: boolean }) => void;
    const oldRead = new Promise<{ running: boolean }>((resolve) => {
      finishOld = resolve;
    });
    let reads = 0;
    const read = () => {
      reads += 1;
      return oldRead;
    };
    const apply = (next: { running: boolean }) => observed.push(next.running);
    const failed = () => {
      throw new Error('unexpected status failure');
    };
    const first = monitor.poll(read, apply, failed);
    const duplicate = monitor.poll(read, apply, failed);
    expect(duplicate).toBe(first);
    await Promise.resolve();
    expect(reads).toBe(1);
    monitor.reset();
    await monitor.poll(async () => ({ running: false }), apply, failed);
    finishOld({ running: true });
    await first;
    expect(observed).toEqual([false]);
  });

  it('reports a failed read and resumes reconciliation on the next poll', async () => {
    const monitor = new ServerStatusMonitor<{ running: boolean }>();
    const failures: unknown[] = [];
    const observed: boolean[] = [];
    const failure = new Error('agent temporarily unavailable');
    const apply = (next: { running: boolean }) => observed.push(next.running);
    const failed = (error: unknown) => failures.push(error);
    await monitor.poll(
      async () => {
        throw failure;
      },
      apply,
      failed,
    );
    await monitor.poll(async () => ({ running: false }), apply, failed);
    expect(failures).toEqual([failure]);
    expect(observed).toEqual([false]);
  });
});
