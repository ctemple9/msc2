export const RUNNER_DESIGN_HEIGHT = 50;
export const MAX_SCORE = 9999;
export const MAX_OBSTACLES = 32;

export const RUNNER_PHYSICS = {
  gravity: -900,
  jumpPeak: 36,
  scrollSpeed: 90,
  previewPadding: 0.2,
  verticalClearance: 3,
} as const;

export type RunnerBounds = {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
};

export function jumpVelocity(scale = 1): number {
  return Math.sqrt(2 * Math.abs(RUNNER_PHYSICS.gravity) * RUNNER_PHYSICS.jumpPeak) * scale;
}

export function totalAirTime(): number {
  return (2 * jumpVelocity()) / Math.abs(RUNNER_PHYSICS.gravity);
}

/** Vertical distance above the ground during a ballistic jump. */
export function jumpHeightAt(time: number, scale = 1): number {
  const gravity = RUNNER_PHYSICS.gravity * scale;
  const velocity = jumpVelocity(scale);
  const airTime = totalAirTime();
  const t = Math.max(0, Math.min(time, airTime));
  return velocity * t + 0.5 * gravity * t * t;
}

/**
 * Mirrors MSC1's frame-aware auto-jump check. It tests the runner's front,
 * middle, and back against the whole obstacle overlap window instead of using
 * one fragile distance threshold for every obstacle shape.
 */
export function shouldAutoJumpNow(
  runner: RunnerBounds,
  obstacle: RunnerBounds,
  scale = 1,
): boolean {
  if (obstacle.maxX <= runner.minX || obstacle.minX <= runner.maxX) return false;

  const speed = RUNNER_PHYSICS.scrollSpeed * scale;
  const airTime = totalAirTime();
  const previewWindow = airTime + RUNNER_PHYSICS.previewPadding;
  const horizontalInset = 2 * scale;
  const requiredHeight = runner.maxY - obstacle.minY + RUNNER_PHYSICS.verticalClearance * scale;
  const sampleXs = [
    runner.maxX - horizontalInset,
    (runner.minX + runner.maxX) * 0.5,
    runner.minX + horizontalInset,
  ];

  let earliestRelevantTime = Number.POSITIVE_INFINITY;

  for (const sampleX of sampleXs) {
    const tStart = Math.max(0, (obstacle.minX - sampleX) / speed);
    const tEnd = Math.min(airTime, (obstacle.maxX - sampleX) / speed);
    if (tEnd < 0 || tStart > airTime || tStart > tEnd) continue;

    earliestRelevantTime = Math.min(earliestRelevantTime, tStart);
    const tMid = (tStart + tEnd) * 0.5;
    const jumpHeights = [
      jumpHeightAt(tStart, scale),
      jumpHeightAt(tMid, scale),
      jumpHeightAt(tEnd, scale),
    ];

    if (jumpHeights.some((height) => height <= requiredHeight)) return false;
  }

  return earliestRelevantTime <= previewWindow;
}

export function insetBounds(bounds: RunnerBounds, dx: number, dy: number): RunnerBounds {
  return {
    minX: bounds.minX + dx,
    maxX: bounds.maxX - dx,
    minY: bounds.minY + dy,
    maxY: bounds.maxY - dy,
  };
}

export function intersects(first: RunnerBounds, second: RunnerBounds): boolean {
  return (
    first.minX < second.maxX &&
    first.maxX > second.minX &&
    first.minY < second.maxY &&
    first.maxY > second.minY
  );
}
