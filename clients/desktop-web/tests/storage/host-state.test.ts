import { describe, expect, it } from 'vitest';
import {
  StorageSession,
  type StorageBridge,
  type StorageSnapshot,
} from '../../src/lib/platform/storage';

// Native caches/requests can complete after host removal. Prior host tests do
// not exercise this new seam. Explicit promises, no timers/I/O, expected <1 ms.
describe('storage host ownership', () => {
  it('rejects a late result after invalidation and a reply for another computer', async () => {
    let resolve!: (value: StorageSnapshot | null) => void;
    const bridge = {
      cached: () =>
        new Promise<StorageSnapshot | null>((done) => {
          resolve = done;
        }),
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

// Essential: the reported 436 GB disk appeared as 406.1 GB because binary counts
// carried decimal labels. Fixed input only, no machine queries/timing, <1 ms.
it('formats capacity and file sizes with decimal system storage units', async () => {
  const { storageBytesLabel } = await import('../../src/lib/sections/app-settings/storage-model');
  expect(storageBytesLabel(436_001_021_952)).toBe('436.0 GB');
  expect(storageBytesLabel(124_400_000)).toBe('124.4 MB');
});
