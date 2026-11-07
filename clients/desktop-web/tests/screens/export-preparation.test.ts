import { expect, it } from 'vitest';
import { ApiError } from '../../src/lib/api/client';
import { publicationReviewOperation } from '../../src/lib/sections/components/content-model';
import type { ScreenApi } from '../../src/lib/sections/shared/types';

// Essential P24.36: reopening must not duplicate a running preparation, reuse
// another request's review, or cancel work this sheet did not start. Existing
// backend admission tests do not cover desktop conflict recovery. Immediate
// mocked terminal responses; no filesystem/network/timers, expected <5 ms total.
function fixture(target = 'server', type = 'content-publication-review') {
  const posted: string[] = [];
  const tracked: string[] = [];
  const api: ScreenApi = {
    async post<T>(path: string): Promise<T> {
      posted.push(path);
      if (posted.length === 1)
        throw new ApiError(409, {
          code: 'operation_conflict',
          message: 'Busy',
          details: { target: 'server', conflictingOperationId: 'previous' },
        });
      return { operationId: 'fresh' } as T;
    },
    async get<T>(path: string): Promise<T> {
      if (path === '/v1/operations/previous')
        return {
          id: 'previous',
          type,
          target,
          state: 'succeeded',
          result: { reviewId: 'old-review' },
        } as T;
      if (path === '/v1/operations/fresh')
        return {
          id: 'fresh',
          type,
          target: 'server',
          state: 'succeeded',
          result: { reviewId: 'new-review' },
        } as T;
      throw new Error(`Unexpected request: ${path}`);
    },
  };
  const run = () =>
    publicationReviewOperation(
      api,
      '/review',
      {},
      () => {},
      (id) => tracked.push(id),
      { serverId: 'server', isCurrent: () => true },
    );
  return { run, posted, tracked };
}
it('waits for the prior review and starts a fresh owned review without cancelling the prior work', async () => {
  const value = fixture();
  expect(await value.run()).toEqual({ reviewId: 'new-review' });
  expect(value.posted).toEqual(['/review', '/review']);
  expect(value.tracked).toEqual(['fresh']);
});
it('refuses to adopt a different server or another kind of task', async () => {
  for (const [target, type] of [
    ['other-server', 'content-publication-review'],
    ['server', 'server-stop'],
  ]) {
    const value = fixture(target, type);
    await expect(value.run()).rejects.toThrow('Another server task');
    expect(value.posted).toEqual(['/review']);
    expect(value.tracked).toEqual([]);
  }
});
