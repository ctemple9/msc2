<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    insetBounds,
    intersects,
    jumpHeightAt,
    jumpVelocity,
    MAX_OBSTACLES,
    MAX_SCORE,
    RUNNER_DESIGN_HEIGHT,
    RUNNER_PHYSICS,
    shouldAutoJumpNow,
    type RunnerBounds,
  } from './runningBannerGame';

  export let running: boolean;
  export let bannerColor: string;

  type ObstacleKind = 'creeper' | 'block' | 'stump';
  type Obstacle = {
    id: number;
    kind: ObstacleKind;
    x: number;
    width: number;
    height: number;
  };
  type Cloud = { x: number; y: number; opacity: number };

  const SCENE_HEIGHT = 30;
  const GROUND_HEIGHT = 8;
  const GRASS_HEIGHT = 2;
  const RUNNER_HEIGHT = 20;
  const RUNNER_BASE_X_RATIO = 0.18;
  const OBSTACLE_SPECS: Record<ObstacleKind, { width: number; height: number }> = {
    creeper: { width: 8, height: 14 },
    block: { width: 10, height: 10 },
    stump: { width: 12, height: 8 },
  };

  let container: HTMLDivElement;
  let viewportWidth = 400;
  let viewportHeight = SCENE_HEIGHT;
  let scale = viewportHeight / RUNNER_DESIGN_HEIGHT;
  let groundY = viewportHeight - GRASS_HEIGHT * scale;
  let runnerX = viewportWidth * RUNNER_BASE_X_RATIO;
  let runnerHeight = RUNNER_HEIGHT * scale;
  let runnerVertical = 0;
  let runnerVelocityY = 0;
  let isOnGround = true;
  let autoJumpEnabled = true;
  let isFlashing = false;
  let flashRemaining = 0;
  let score = 0;
  let scoreTick = 0;
  let elapsed = 0;
  let groundOffset = 0;
  let obstacleSpawnTimer = 0;
  let nextSpawnInterval = 3;
  let nextObstacleId = 1;
  let obstacles: Obstacle[] = [];
  let clouds: Cloud[] = [];
  let animationFrame: number | undefined;
  let lastFrameTime = 0;
  let accumulator = 0;
  let mounted = false;
  let lastRunning = false;
  let resizeObserver: ResizeObserver | undefined;

  $: scale = Math.max(0.5, viewportHeight / RUNNER_DESIGN_HEIGHT);
  $: groundY = viewportHeight - (GROUND_HEIGHT + GRASS_HEIGHT) * scale;
  $: runnerHeight = RUNNER_HEIGHT * scale;

  function randomBetween(min: number, max: number): number {
    return min + Math.random() * (max - min);
  }

  function updateSize(): void {
    if (!container) return;
    const rect = container.getBoundingClientRect();
    viewportWidth = Math.max(1, rect.width);
    viewportHeight = Math.max(1, rect.height);
    runnerX = viewportWidth * RUNNER_BASE_X_RATIO;
    if (isOnGround) runnerVertical = 0;
  }

  function resetRun(): void {
    obstacles = [];
    obstacleSpawnTimer = 0;
    nextSpawnInterval = randomBetween(2.8, 5.0);
    nextObstacleId = 1;
    autoJumpEnabled = true;
    isFlashing = false;
    flashRemaining = 0;
    score = 0;
    scoreTick = 0;
    elapsed = 0;
    groundOffset = 0;
    runnerVertical = 0;
    runnerVelocityY = 0;
    isOnGround = true;
    runnerX = viewportWidth * RUNNER_BASE_X_RATIO;
  }

  function resetClouds(): void {
    clouds = [
      { x: viewportWidth * 0.18, y: viewportHeight * 0.23, opacity: 0.25 },
      { x: viewportWidth * 0.53, y: viewportHeight * 0.16, opacity: 0.19 },
      { x: viewportWidth * 0.82, y: viewportHeight * 0.3, opacity: 0.29 },
    ];
  }

  function startLoop(): void {
    if (animationFrame !== undefined) return;
    resetRun();
    resetClouds();
    lastFrameTime = 0;
    accumulator = 0;
    animationFrame = requestAnimationFrame(frame);
  }

  function stopLoop(): void {
    if (animationFrame !== undefined) cancelAnimationFrame(animationFrame);
    animationFrame = undefined;
    lastFrameTime = 0;
    accumulator = 0;
  }

  function frame(now: number): void {
    if (!running) {
      stopLoop();
      return;
    }

    const rawDelta = lastFrameTime === 0 ? 0 : (now - lastFrameTime) / 1000;
    lastFrameTime = now;
    accumulator += Math.min(rawDelta, 0.1);

    const fixedDelta = 1 / 60;
    let steps = 0;
    while (accumulator >= fixedDelta && steps < 6) {
      step(fixedDelta);
      accumulator -= fixedDelta;
      steps += 1;
    }

    animationFrame = requestAnimationFrame(frame);
  }

  function step(delta: number): void {
    elapsed += delta;
    groundOffset -= RUNNER_PHYSICS.scrollSpeed * scale * delta;
    if (groundOffset <= -viewportWidth) groundOffset += viewportWidth;

    const cloudDelta = RUNNER_PHYSICS.scrollSpeed * 0.31 * scale * delta;
    clouds = clouds.map((cloud) => {
      const nextX = cloud.x - cloudDelta;
      return nextX < -30 * scale
        ? {
            ...cloud,
            x: viewportWidth + randomBetween(0, 20) * scale,
            y: viewportHeight * 0.25 + randomBetween(-3, 3) * scale,
          }
        : { ...cloud, x: nextX };
    });

    updateObstacles(delta);
    updateRunner(delta);
    maybeAutoJump();
    checkCollisions();
    updateScore(delta);

    if (isFlashing) {
      flashRemaining -= delta;
      if (flashRemaining <= 0) resetRun();
    }
  }

  function updateObstacles(delta: number): void {
    const dx = RUNNER_PHYSICS.scrollSpeed * scale * delta;
    obstacles = obstacles.filter((obstacle) => obstacle.x > -24 * scale);
    obstacles = obstacles.map((obstacle) => ({ ...obstacle, x: obstacle.x - dx }));
    obstacleSpawnTimer += delta;
    if (obstacleSpawnTimer < nextSpawnInterval) return;

    obstacleSpawnTimer = 0;
    nextSpawnInterval = randomBetween(2.8, 5.2);
    if (Math.random() >= 0.65) return;

    const kinds: ObstacleKind[] = ['creeper', 'block', 'stump'];
    const kind = kinds[Math.floor(Math.random() * kinds.length)];
    const spec = OBSTACLE_SPECS[kind];
    const nextObstacle = {
      id: nextObstacleId++,
      kind,
      x: viewportWidth + 14 * scale,
      width: spec.width * scale,
      height: spec.height * scale,
    } satisfies Obstacle;
    obstacles = [...obstacles, nextObstacle].slice(-MAX_OBSTACLES);
  }

  function runnerBounds(): RunnerBounds {
    return {
      minX: runnerX - 5 * scale,
      maxX: runnerX + 5 * scale,
      minY: groundY - runnerHeight - runnerVertical,
      maxY: groundY - runnerVertical,
    };
  }

  function obstacleBounds(obstacle: Obstacle): RunnerBounds {
    return {
      minX: obstacle.x,
      maxX: obstacle.x + obstacle.width,
      minY: groundY - obstacle.height,
      maxY: groundY,
    };
  }

  function updateRunner(delta: number): void {
    if (isOnGround) return;

    runnerVelocityY += RUNNER_PHYSICS.gravity * scale * delta;
    runnerVertical += runnerVelocityY * delta;
    if (runnerVertical > 0) return;

    runnerVertical = 0;
    runnerVelocityY = 0;
    isOnGround = true;
  }

  function maybeAutoJump(): void {
    if (!autoJumpEnabled || !isOnGround) return;
    const runner = runnerBounds();
    const nearest = obstacles
      .filter((obstacle) => obstacle.x + obstacle.width >= runner.minX)
      .sort((first, second) => first.x - second.x)[0];
    if (!nearest) return;
    if (shouldAutoJumpNow(runner, obstacleBounds(nearest), scale)) jump();
  }

  function checkCollisions(): void {
    if (isFlashing) return;
    const runner = insetBounds(runnerBounds(), 4 * scale, 3 * scale);
    for (const obstacle of obstacles) {
      if (intersects(runner, insetBounds(obstacleBounds(obstacle), 1 * scale, 1 * scale))) {
        isFlashing = true;
        flashRemaining = 0.3;
        break;
      }
    }
  }

  function updateScore(delta: number): void {
    scoreTick += delta;
    while (scoreTick >= 0.1) {
      scoreTick -= 0.1;
      score += 1;
      if (score >= MAX_SCORE) {
        resetRun();
        return;
      }
    }
  }

  function jump(): void {
    if (!isOnGround || isFlashing) return;
    isOnGround = false;
    runnerVelocityY = jumpVelocity(scale);
  }

  function handleJumpInput(): void {
    if (!running || isFlashing) return;
    autoJumpEnabled = false;
    jump();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key !== ' ' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    handleJumpInput();
  }

  function obstacleTransform(obstacle: Obstacle): string {
    return `translate(${obstacle.x} ${groundY - obstacle.height})`;
  }

  $: if (mounted && running !== lastRunning) {
    lastRunning = running;
    if (running) startLoop();
    else stopLoop();
  }

  onMount(() => {
    mounted = true;
    lastRunning = running;
    resizeObserver = new ResizeObserver(updateSize);
    resizeObserver.observe(container);
    updateSize();
    resetClouds();
    if (running) startLoop();
  });

  onDestroy(() => {
    mounted = false;
    stopLoop();
    resizeObserver?.disconnect();
  });
</script>

<div
  bind:this={container}
  class="running-banner-game"
  role="button"
  tabindex="0"
  aria-label="Running server mini-game. Click or press Space to jump."
  onpointerdown={handleJumpInput}
  onkeydown={handleKeydown}
>
  <svg
    viewBox={`0 0 ${viewportWidth} ${viewportHeight}`}
    preserveAspectRatio="none"
    aria-hidden="true"
  >
    <rect width={viewportWidth} height={viewportHeight} fill={bannerColor} />

    {#each clouds as cloud}
      <g transform={`translate(${cloud.x} ${cloud.y}) scale(${scale})`} opacity={cloud.opacity}>
        <rect x="0" y="3" width="10" height="5" fill="white" />
        <rect x="3" y="0" width="14" height="6" fill="white" />
        <rect x="10" y="2" width="8" height="5" fill="white" />
      </g>
    {/each}

    <g aria-hidden="true">
      <rect
        x={groundOffset}
        y={groundY}
        width={viewportWidth}
        height={GRASS_HEIGHT * scale}
        fill="rgba(255,255,255,0.18)"
      />
      <rect
        x={groundOffset}
        y={groundY + GRASS_HEIGHT * scale}
        width={viewportWidth}
        height={GROUND_HEIGHT * scale}
        fill="rgba(0,0,0,0.28)"
      />
      <rect
        x={groundOffset + viewportWidth}
        y={groundY}
        width={viewportWidth}
        height={GRASS_HEIGHT * scale}
        fill="rgba(255,255,255,0.18)"
      />
      <rect
        x={groundOffset + viewportWidth}
        y={groundY + GRASS_HEIGHT * scale}
        width={viewportWidth}
        height={GROUND_HEIGHT * scale}
        fill="rgba(0,0,0,0.28)"
      />
    </g>

    {#each obstacles as obstacle (obstacle.id)}
      <g transform={obstacleTransform(obstacle)}>
        {#if obstacle.kind === 'creeper'}
          <rect width={8 * scale} height={14 * scale} fill="#339933" />
          <rect y={10 * scale} width={2 * scale} height={2 * scale} fill="#0d470d" />
          <rect x={5 * scale} y={10 * scale} width={2 * scale} height={2 * scale} fill="#0d470d" />
          <rect x={2 * scale} y={8 * scale} width={4 * scale} height={2 * scale} fill="#0d470d" />
          <rect x={3 * scale} y={6 * scale} width={2 * scale} height={2 * scale} fill="#0d470d" />
          <rect width={3 * scale} height={4 * scale} fill="#1a661a" />
          <rect x={5 * scale} width={3 * scale} height={4 * scale} fill="#1a661a" />
        {:else if obstacle.kind === 'block'}
          <rect width={10 * scale} height={10 * scale} fill="#87878c" />
          <rect x={1 * scale} y={8 * scale} width={3 * scale} height={1 * scale} fill="#a3a3a8" />
          <rect x={5 * scale} y={8 * scale} width={2 * scale} height={1 * scale} fill="#5e5e63" />
          <rect x={1 * scale} y={4 * scale} width={2 * scale} height={2 * scale} fill="#45454a" />
          <rect x={6 * scale} y={4 * scale} width={2 * scale} height={2 * scale} fill="#a3a3a8" />
          <rect x={4 * scale} y={0} width={2 * scale} height={1 * scale} fill="#45454a" />
        {:else}
          <rect width={12 * scale} height={8 * scale} fill="#7a5129" />
          <rect x={2 * scale} y={6 * scale} width={8 * scale} height={2 * scale} fill="#8c6638" />
          <rect x={3 * scale} width={1 * scale} height={6 * scale} fill="#4d2e14" />
          <rect x={7 * scale} width={1 * scale} height={6 * scale} fill="#4d2e14" />
        {/if}
      </g>
    {/each}

    <g
      class:hit={isFlashing && Math.floor(flashRemaining / 0.05) % 2 === 0}
      transform={`translate(${runnerX} ${groundY - runnerVertical}) scale(${scale} -${scale})`}
    >
      <rect x="-4" y="17" width="9" height="3" fill="#140d08" />
      <rect x="-4" y="13" width="9" height="4" fill="#332114" />
      <rect x="-4" y="15" width="9" height="2" fill="#332114" />
      <rect x="-5" y="4" width="10" height="7" fill="#2659b8" />
      <rect x="-1" y="4" width="2" height="7" fill="#1a4299" />
      <rect x="-4" y="8" width="9" height="4" fill="#332114" />
      <rect x="-4" y="6" width="9" height="2" fill="#140d08" />
      <rect x="-3" y="8" width="2" height="2" fill="#d9bf99" />
      <rect x="1" y="8" width="2" height="2" fill="#d9bf99" />
      <rect x="-4" y="11" width="9" height="2" fill="#332114" />
      <rect x="-4" y="0" width="4" height="4" fill="#472f78" />
      <rect x="1" y="0" width="4" height="4" fill="#472f78" />
      {#if Math.floor(elapsed * 10) % 2 === 0 || !isOnGround}
        <rect x="-4" y="0" width="4" height="1" fill="#4d2e14" />
        <rect x="1" y="1" width="4" height="1" fill="#4d2e14" />
      {:else}
        <rect x="-4" y="1" width="4" height="1" fill="#4d2e14" />
        <rect x="1" y="0" width="4" height="1" fill="#4d2e14" />
      {/if}
    </g>

    <text
      x={viewportWidth - 6}
      y={10 * scale}
      text-anchor="end"
      fill="rgba(255,255,255,0.7)"
      font-family="ui-monospace, SFMono-Regular, Menlo, monospace"
      font-size={9 * scale}>{score}</text
    >
  </svg>
</div>

<style>
  .running-banner-game {
    flex: 1;
    min-width: 0;
    height: 30px;
    overflow: hidden;
    border-radius: 7px;
    cursor: pointer;
    outline: none;
    touch-action: manipulation;
  }
  .running-banner-game:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.65);
    outline-offset: 2px;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
    shape-rendering: crispEdges;
  }
  .hit {
    filter: sepia(1) saturate(8) hue-rotate(300deg);
  }
</style>
