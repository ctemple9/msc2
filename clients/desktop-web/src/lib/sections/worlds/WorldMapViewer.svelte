<script lang="ts">
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { getCurrentWindow, LogicalPosition } from '@tauri-apps/api/window';
  import type { WorldSource } from '@thoughts-on-things/vantage-mc/core';
  import type { VantageViewer } from '@thoughts-on-things/vantage-mc/three';
  import type { Schema, ScreenApi } from '../shared/types';

  export let api: ScreenApi | undefined;
  export let serverId: string;
  export let worldName: string;
  export let onClose: () => void;

  type Dimension = Schema['WorldMapDimensionDTO'];
  let dimensions: Dimension[] = [];
  let selectedDimension = '';
  let serverType = '';
  let status = 'Checking saved terrain…';
  let busy = true;
  let canvas: HTMLDivElement;
  let viewer: VantageViewer | undefined;
  let alive = true;
  let loadGeneration = 0;
  let frameId = 0;
  let coords = 'XYZ —';
  let flying = false;
  let topDown = false;
  let spawn: { x: number; y: number; z: number } | undefined;
  let message = '';
  let messageTimer: ReturnType<typeof setTimeout> | undefined;
  let desktopLook = false;
  let lookClick: { x: number; y: number } | undefined;
  let warping = false;
  let ignoreWarp = false;

  $: selected = dimensions.find((dimension) => dimension.id === selectedDimension);

  function say(value: string): void {
    message = value;
    if (messageTimer) clearTimeout(messageTimer);
    messageTimer = setTimeout(() => (message = ''), 4000);
  }

  function disposeViewer(): void {
    releaseDesktopLook();
    viewer?.dispose();
    viewer = undefined;
  }

  function asBuffer(bytes: Uint8Array): ArrayBuffer {
    const buffer = new ArrayBuffer(bytes.byteLength);
    new Uint8Array(buffer).set(bytes);
    return buffer;
  }

  function artifactPath(dimension: string, path: string): string {
    const params = new URLSearchParams({ dimension, path });
    return `/v1/worlds/map/terrain?${params.toString()}`;
  }

  async function loadDimension(dimension: string): Promise<void> {
    selectedDimension = dimension;
    const generation = ++loadGeneration;
    disposeViewer();
    spawn = undefined;
    const entry = dimensions.find((item) => item.id === dimension);
    if (!entry) return;
    if (entry.state !== 'ready') {
      status = entry.reason || 'No saved terrain has been generated in this dimension yet.';
      busy = false;
      return;
    }
    if (!api?.getBytes) {
      status = 'Connect to an MSC agent to load saved terrain.';
      busy = false;
      return;
    }
    busy = true;
    status = `Loading ${entry.displayName}…`;
    let opening: VantageViewer | undefined;
    try {
      const read = async (path: string, signal?: AbortSignal): Promise<ArrayBuffer> => {
        if (signal?.aborted) throw new DOMException('Cancelled', 'AbortError');
        const bytes = await api!.getBytes!(artifactPath(dimension, path));
        if (signal?.aborted || !alive || generation !== loadGeneration) {
          throw new DOMException('Cancelled', 'AbortError');
        }
        return asBuffer(bytes);
      };
      const manifest = JSON.parse(new TextDecoder().decode(await read('manifest.json'))) as {
        spawn?: { x: number; y: number; z: number };
      };
      if (!alive || generation !== loadGeneration) return;
      const source: WorldSource = {
        label: `${worldName} · ${entry.displayName}`,
        manifest,
        fetch: read,
      };
      const { VantageViewer: Viewer } = await import('@thoughts-on-things/vantage-mc/three');
      if (!alive || generation !== loadGeneration) return;
      opening = new Viewer(canvas, { players: { enabled: false }, urlState: false });
      await opening.load({ world: source });
      if (!alive || generation !== loadGeneration) return;
      viewer = opening;
      opening = undefined;
      spawn =
        manifest.spawn &&
        [manifest.spawn.x, manifest.spawn.y, manifest.spawn.z].every(Number.isFinite)
          ? manifest.spawn
          : undefined;
      status = `Saved ${entry.displayName} terrain`;
    } catch (error) {
      if (alive && generation === loadGeneration) {
        status = error instanceof Error ? error.message : 'Saved terrain could not be loaded.';
      }
    } finally {
      opening?.dispose();
      if (alive && generation === loadGeneration) busy = false;
    }
  }

  async function loadDimensions(): Promise<void> {
    if (!api) {
      status = 'Connect to an MSC agent to view saved terrain.';
      busy = false;
      return;
    }
    try {
      const catalog = await api.get<Schema['WorldMapDimensionsResponseDTO']>(
        '/v1/worlds/map/dimensions',
      );
      if (!alive) return;
      if (catalog.serverId !== serverId) {
        status = 'Select this server in MSC to view its active world.';
        busy = false;
        return;
      }
      serverType = catalog.serverType;
      dimensions = catalog.dimensions;
      if (serverType === 'bedrock') {
        status =
          'Bedrock terrain is available in the proof viewer; integration into MSC is still in progress.';
        busy = false;
        return;
      }
      const preferred =
        dimensions.find(
          (dimension) => dimension.id === 'minecraft:overworld' && dimension.state === 'ready',
        ) ??
        dimensions.find((dimension) => dimension.state === 'ready') ??
        dimensions[0];
      if (preferred) await loadDimension(preferred.id);
      else {
        status = 'This world has no saved dimensions yet.';
        busy = false;
      }
    } catch (error) {
      if (alive) {
        status = error instanceof Error ? error.message : 'Dimensions could not be loaded.';
        busy = false;
      }
    }
  }

  function updateToolbar(): void {
    if (!alive) return;
    if (viewer) {
      const point =
        viewer.controls.mode === 'fly' ? viewer.camera.position : viewer.controls.position;
      coords = `XYZ ${Math.floor(point.x)}, ${Math.floor(point.y)}, ${Math.floor(point.z)}`;
      flying = viewer.isFlying;
      topDown = !flying && viewer.tilt <= 0.08;
    }
    frameId = requestAnimationFrame(updateToolbar);
  }

  function leaveFly(): void {
    releaseDesktopLook();
    viewer?.setFlyMode(false);
  }

  function releaseDesktopLook(): void {
    if (!desktopLook) return;
    desktopLook = false;
    lookClick = undefined;
    warping = false;
    ignoreWarp = false;
    void getCurrentWindow().setCursorVisible(true).catch(() => {});
  }

  async function centerDesktopPointer(): Promise<void> {
    if (!desktopLook || warping) return;
    warping = true;
    const rect = canvas.getBoundingClientRect();
    try {
      ignoreWarp = true;
      await getCurrentWindow().setCursorPosition(
        new LogicalPosition(rect.left + rect.width / 2, rect.top + rect.height / 2),
      );
    } catch {
      releaseDesktopLook();
      say('Mouse capture is unavailable in this desktop window. Drag to look.');
    } finally {
      warping = false;
    }
  }

  function beginDesktopLook(event: PointerEvent): void {
    if (!isTauri() || !viewer?.isFlying || desktopLook || event.button !== 0) return;
    if (!lookClick || Math.hypot(event.clientX - lookClick.x, event.clientY - lookClick.y) > 3)
      return;
    lookClick = undefined;
    // WebKit in the desktop window does not grant the browser's pointer lock.
    // Allow that request to complete first, then use window cursor control if needed.
    setTimeout(async () => {
      if (!viewer?.isFlying || viewer.controls.isPointerLocked || desktopLook) return;
      desktopLook = true;
      try {
        await getCurrentWindow().setCursorVisible(false);
        if (!desktopLook) {
          await getCurrentWindow().setCursorVisible(true);
          return;
        }
        await centerDesktopPointer();
        if (desktopLook) say('Mouse look active · Esc releases pointer');
      } catch {
        releaseDesktopLook();
        say('Mouse capture is unavailable in this desktop window. Drag to look.');
      }
    }, 80);
  }

  function moveDesktopLook(event: PointerEvent): void {
    if (!desktopLook || !viewer?.isFlying || viewer.controls.isPointerLocked) return;
    const rect = canvas.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;
    if (
      ignoreWarp &&
      Math.abs(event.clientX - centerX) < 2 &&
      Math.abs(event.clientY - centerY) < 2
    ) {
      ignoreWarp = false;
      return;
    }
    if (warping) return;
    const dx = event.clientX - centerX;
    const dy = event.clientY - centerY;
    if (Math.abs(dx) < 1 && Math.abs(dy) < 1) return;
    const controls = viewer.controls;
    const sensitivity = 1.5 / Math.max(canvas.clientHeight, 1);
    controls.rotation += dx * sensitivity;
    controls.angle = Math.max(0.02, Math.min(Math.PI - 0.02, controls.angle - dy * sensitivity));
    viewer.invalidate();
    void centerDesktopPointer();
  }

  function goHome(): void {
    if (!viewer || !spawn) return;
    leaveFly();
    viewer.controls.animateTo({
      position: viewer.controls.position.clone().set(spawn.x + 0.5, spawn.y, spawn.z + 0.5),
      distance: 140,
    });
    viewer.invalidate();
  }

  function capture(): void {
    if (!viewer) return;
    const link = document.createElement('a');
    link.href = viewer.screenshot();
    link.download = `msc-world-${new Date().toISOString().replace(/[:.]/g, '-')}.png`;
    link.click();
    say('Screenshot saved');
  }

  onMount(() => {
    alive = true;
    const onMove = (event: PointerEvent) => moveDesktopLook(event);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') releaseDesktopLook();
    };
    window.addEventListener('pointermove', onMove, true);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('blur', releaseDesktopLook);
    void loadDimensions();
    frameId = requestAnimationFrame(updateToolbar);
    return () => {
      alive = false;
      ++loadGeneration;
      cancelAnimationFrame(frameId);
      if (messageTimer) clearTimeout(messageTimer);
      window.removeEventListener('pointermove', onMove, true);
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('blur', releaseDesktopLook);
      disposeViewer();
    };
  });
</script>

<section class="map-shell" aria-label={`Saved terrain for ${worldName}`}>
  <header class="map-header">
    <div class="map-title">
      <button type="button" class="back" onclick={onClose}>← Worlds</button>
      <span class="divider" aria-hidden="true"></span>
      <strong>{worldName}</strong>
      <span class="saved">Saved terrain</span>
    </div>
    {#if serverType === 'java' && dimensions.length > 0}
      <label class="dimension-picker">
        <span>Dimension</span>
        <select
          value={selectedDimension}
          onchange={(event) => void loadDimension(event.currentTarget.value)}
        >
          {#each dimensions as dimension (dimension.id)}
            <option value={dimension.id}
              >{dimension.displayName}{dimension.state === 'ready' ? '' : ' · unavailable'}</option
            >
          {/each}
        </select>
      </label>
    {/if}
  </header>

  <div class="map-stage">
    <div
      bind:this={canvas}
      class="map-canvas"
      role="application"
      aria-label="World map camera"
      onpointerdown={(event) => {
        lookClick = { x: event.clientX, y: event.clientY };
      }}
      onpointerup={beginDesktopLook}
    ></div>
    {#if !viewer}
      <div class="map-state" role="status">
        <strong>{busy ? 'Preparing map' : (selected?.displayName ?? 'Map unavailable')}</strong>
        <p>{status}</p>
      </div>
    {/if}
    <aside class="players-panel">
      <strong>Players</strong>
      <p>Live positions will appear here when the player feed is connected.</p>
    </aside>
    <div class="map-caption" role="status">{status}{message ? ` · ${message}` : ''}</div>
    <nav
      class="map-toolbar"
      aria-label="Map controls"
      onpointerdown={(event) => event.stopPropagation()}
    >
      <button
        type="button"
        class:chosen={topDown && !flying}
        disabled={!viewer}
        aria-pressed={topDown && !flying}
        onclick={() => {
          leaveFly();
          viewer?.flatten();
        }}>2D</button
      >
      <button
        type="button"
        class:chosen={!topDown && !flying}
        disabled={!viewer}
        aria-pressed={!topDown && !flying}
        onclick={() => {
          leaveFly();
          viewer?.setTilt(0.42);
        }}>3D</button
      >
      <button
        type="button"
        class:chosen={flying}
        disabled={!viewer}
        aria-pressed={flying}
        onclick={() => {
          viewer?.toggleFly();
          if (viewer?.isFlying) {
            viewer.controls.angle = Math.PI / 2 - 0.2;
            viewer.invalidate();
            say('Click the map to look · WASD move · Space up · Shift down');
          } else releaseDesktopLook();
        }}>Fly</button
      >
      <output class="coordinates">{coords}</output>
      <button
        type="button"
        disabled={!viewer}
        title="Zoom out"
        aria-label="Zoom out"
        onclick={() => viewer?.zoomBy(-1)}>−</button
      >
      <button
        type="button"
        disabled={!viewer}
        title="Zoom in"
        aria-label="Zoom in"
        onclick={() => viewer?.zoomBy(1)}>+</button
      >
      <button
        type="button"
        disabled={!viewer || !spawn}
        title={spawn ? 'Return to world spawn' : 'Spawn coordinates unavailable'}
        onclick={goHome}>Home</button
      >
      <button type="button" disabled={!viewer} title="Save a screenshot" onclick={capture}
        >Camera</button
      >
    </nav>
  </div>
</section>

<style>
  .map-shell {
    min-height: 620px;
    height: calc(100vh - 184px);
    display: flex;
    flex-direction: column;
    background: var(--msc2-bg-content, #1c1c21);
    border: 1px solid var(--msc2-border, #34343a);
    border-radius: 10px;
    overflow: hidden;
  }
  .map-header {
    min-height: 54px;
    padding: 8px 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    background: #141417;
    color: #f1f1f2;
  }
  .map-title {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .map-title strong {
    font-size: 14px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .back {
    border: 0;
    background: none;
    color: #b9c1ca;
    font: inherit;
    cursor: pointer;
    white-space: nowrap;
  }
  .back:hover {
    color: #fff;
  }
  .divider {
    width: 1px;
    height: 18px;
    background: #3a3a40;
  }
  .saved {
    color: #92979d;
    font-size: 11px;
    white-space: nowrap;
  }
  .dimension-picker {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #aeb4bb;
    font-size: 11px;
  }
  .dimension-picker select {
    max-width: 215px;
    padding: 7px 10px;
    color: #f1f1f2;
    background: #242428;
    border: 1px solid #48484e;
    border-radius: 5px;
    font: inherit;
  }
  .map-stage {
    position: relative;
    flex: 1;
    min-height: 0;
    background: #1c1c21;
  }
  .map-canvas {
    position: absolute;
    inset: 0;
  }
  .map-canvas :global(canvas) {
    display: block;
  }
  .map-state {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    max-width: 380px;
    padding: 20px 24px;
    background: #141417;
    color: #f1f1f2;
    text-align: center;
    border: 1px solid #3a3a40;
    border-radius: 7px;
  }
  .map-state strong {
    font-size: 15px;
    font-weight: 600;
  }
  .map-state p {
    margin: 8px 0 0;
    color: #aeb4bb;
    font-size: 12px;
    line-height: 1.5;
  }
  .players-panel {
    position: absolute;
    right: 16px;
    top: 16px;
    width: 224px;
    padding: 12px 14px;
    background: #141417;
    color: #f1f1f2;
    border: 1px solid #3a3a40;
    border-radius: 7px;
  }
  .players-panel strong {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .players-panel p {
    margin: 8px 0 0;
    color: #aeb4bb;
    font-size: 11px;
    line-height: 1.4;
  }
  .map-caption {
    position: absolute;
    left: 16px;
    bottom: 16px;
    max-width: 260px;
    padding: 8px 10px;
    background: #141417;
    color: #aeb4bb;
    font-size: 11px;
    border-radius: 5px;
  }
  .map-toolbar {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px;
    background: #141417;
    border: 1px solid #3a3a40;
    border-radius: 7px;
    white-space: nowrap;
  }
  .map-toolbar button {
    min-width: 36px;
    height: 31px;
    padding: 0 8px;
    background: #242428;
    color: #d7dce1;
    border: 1px solid #48484e;
    border-radius: 4px;
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }
  .map-toolbar button:hover:not(:disabled) {
    background: #303038;
  }
  .map-toolbar button.chosen {
    color: #fff;
    background: #284c72;
    border-color: #4888c7;
  }
  .map-toolbar button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .coordinates {
    min-width: 155px;
    padding: 0 8px;
    color: #c3cad1;
    text-align: center;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  @media (max-width: 850px) {
    .map-shell {
      min-height: 520px;
    }
    .map-toolbar {
      left: 12px;
      right: 12px;
      transform: none;
      overflow-x: auto;
    }
    .players-panel {
      width: 170px;
    }
    .map-caption {
      bottom: 62px;
    }
  }
</style>
