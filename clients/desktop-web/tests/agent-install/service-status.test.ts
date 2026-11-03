import { afterEach, describe, expect, it, vi } from 'vitest';
import { createServiceStatusMonitor } from '../../src/lib/sections/setup/service-status';
import type { AgentServiceStatus } from '../../src/lib/platform';

const running: AgentServiceStatus = {
  available: true,
  platform: 'windows',
  serviceName: 'msc-agent',
  state: 'running',
  detail: 'Running',
};

afterEach(() => vi.useRealTimers());

describe('agent service status refresh', () => {
  it('recovers from a rejected check without requiring the page to reopen', async () => {
    vi.useFakeTimers();
    const read = vi
      .fn()
      .mockRejectedValueOnce(new Error('query failed'))
      .mockResolvedValue(running);
    const update = vi.fn();
    const failed = vi.fn();
    const monitor = createServiceStatusMonitor(read, update, failed);
    monitor.configure('local:ready');
    await vi.advanceTimersByTimeAsync(0);
    expect(failed).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(5000);
    expect(update).toHaveBeenCalledWith(running);
    monitor.dispose();
  });

  it('reports a hung check without piling up native queries, then accepts recovery', async () => {
    vi.useFakeTimers();
    let resolve!: (status: AgentServiceStatus) => void;
    const read = vi
      .fn<() => Promise<AgentServiceStatus>>()
      .mockImplementationOnce(() => new Promise<AgentServiceStatus>((done) => (resolve = done)))
      .mockResolvedValue(running);
    const update = vi.fn();
    const failed = vi.fn();
    const monitor = createServiceStatusMonitor(read, update, failed);
    monitor.configure('local:ready');
    monitor.configure(undefined);
    monitor.configure('local:ready');
    await vi.advanceTimersByTimeAsync(10_000);
    expect(failed).toHaveBeenCalledOnce();
    void monitor.refresh();
    await vi.advanceTimersByTimeAsync(20_000);
    expect(read).toHaveBeenCalledOnce();
    resolve(running);
    await vi.advanceTimersByTimeAsync(0);
    expect(update).toHaveBeenCalledWith(running);
    monitor.dispose();
  });

  it('discards replies from a prior page activation and stops refreshing when hidden', async () => {
    vi.useFakeTimers();
    let resolve!: (status: AgentServiceStatus) => void;
    const read = vi.fn(() => new Promise<AgentServiceStatus>((done) => (resolve = done)));
    const update = vi.fn();
    const monitor = createServiceStatusMonitor(read, update, vi.fn());
    monitor.configure('local:ready');
    monitor.configure(undefined);
    resolve(running);
    await vi.advanceTimersByTimeAsync(20_000);
    expect(update).not.toHaveBeenCalled();
    expect(read).toHaveBeenCalledOnce();
    monitor.dispose();
  });
});
