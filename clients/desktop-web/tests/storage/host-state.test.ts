import { describe, expect, it } from 'vitest';
import { StorageSession, type StorageBridge, type StorageSnapshot } from '../../src/lib/platform/storage';

// Native caches/requests can complete after host removal. Prior host tests do
// not exercise this new seam. Explicit promises, no timers/I/O, expected <1 ms.
describe('storage host ownership', () => {
  it('rejects a late result after invalidation and a reply for another computer', async () => {
    let resolve!: (value: StorageSnapshot | null) => void;
    const bridge = {
      cached: () => new Promise<StorageSnapshot | null>((done) => { resolve = done; }),
    } as unknown as StorageBridge;
    const session = new StorageSession(bridge);
    const late = session.cached('agent-one');
    session.invalidate('agent-one');
    resolve(null);
    await expect(late).rejects.toThrow('authorization changed');
    const wrong = session.cached('agent-one');
    resolve({ hostId: 'agent-two' } as StorageSnapshot);
    await expect(wrong).rejects.toThrow('another computer');
  });
});
