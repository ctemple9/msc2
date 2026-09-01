import { describe, expect, it } from 'vitest';
import {
  intersects,
  jumpHeightAt,
  jumpVelocity,
  MAX_OBSTACLES,
  MAX_SCORE,
  shouldAutoJumpNow,
  totalAirTime,
} from '../../src/lib/components/shell/runningBannerGame';

describe('running banner game physics', () => {
  const runner = { minX: 100, maxX: 110, minY: 10, maxY: 30 };

  it('keeps long-running state bounded', () => {
    expect(MAX_SCORE).toBe(9999);
    expect(MAX_OBSTACLES).toBe(32);
  });

  it('matches MSC1 jump height and airtime', () => {
    expect(jumpVelocity()).toBeCloseTo(254.56, 1);
    expect(totalAirTime()).toBeCloseTo(0.566, 2);
    expect(jumpHeightAt(totalAirTime() / 2)).toBeCloseTo(36, 1);
  });

  it('jumps early enough to clear a normal obstacle', () => {
    expect(shouldAutoJumpNow(runner, { minX: 120, maxX: 128, minY: 22, maxY: 30 })).toBe(true);
  });

  it('does not jump when the obstacle is too far away or too tall', () => {
    expect(shouldAutoJumpNow(runner, { minX: 180, maxX: 188, minY: 22, maxY: 30 })).toBe(false);
    expect(shouldAutoJumpNow(runner, { minX: 120, maxX: 128, minY: 0, maxY: 30 })).toBe(false);
  });

  it('uses intersecting bounds for forgiving collision decisions', () => {
    expect(intersects(runner, { minX: 109, maxX: 118, minY: 20, maxY: 30 })).toBe(true);
    expect(intersects(runner, { minX: 111, maxX: 118, minY: 20, maxY: 30 })).toBe(false);
  });
});
