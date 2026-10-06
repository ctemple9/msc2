<script lang="ts">
  import { onMount } from 'svelte';
  import Select from '../../components/base/Select.svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { getCurrentWindow, LogicalPosition } from '@tauri-apps/api/window';
  import type { WorldSource } from '@thoughts-on-things/vantage-mc/core';
  import type {
    PlayerLayer,
    PlayerSnapshot,
    VantageViewer,
  } from '@thoughts-on-things/vantage-mc/three';
  import type { Schema, ScreenApi } from '../shared/types';
  import { pollOperation } from './model';

  export let api: ScreenApi | undefined;
  export let serverId: string;
  export let worldName: string;
  export let onClose: () => void;

  type Dimension = Schema['WorldMapDimensionDTO'];
  type LiveFeed = Schema['WorldMapPlayersResponseDTO'];
  type LivePlayer = Schema['WorldMapPlayerDTO'];
  let dimensions: Dimension[] = [];
  let selectedDimension = '';
  let serverType = '';
  let status = 'Checking saved terrain…';
  let busy = true;
  let refreshing = false;
  let canvas: HTMLDivElement;
  let viewer: VantageViewer | undefined;
  let alive = true;
  let loadGeneration = 0;
  let frameId = 0;
  let coords = 'XYZ —';
  let flying = false;
  let topDown = false;
  let depthY = 83;
  let depthMin = 2;
  let depthMax = 126;
  let spawn: { x: number; y: number; z: number } | undefined;
  let message = '';
  let messageTimer: ReturnType<typeof setTimeout> | undefined;
  let desktopLook = false;
  let lookClick: { x: number; y: number } | undefined;
  let lastPointer: { x: number; y: number } | undefined;
  let warping = false;
  let ignoreWarpUntil = 0;
  let resyncPointer = false;
  let playerLayer: PlayerLayer | undefined;
  let livePlayers: LivePlayer[] = [];
  let playerFeedStatus = 'Connecting to live player feed…';
  let followedId: string | undefined;
  let changingPlayerDimension = false;
  let playerPoll: ReturnType<typeof setInterval> | undefined;
  let playerRequest = 0;
  let lastAppliedPlayerRequest = 0;
  let playerRequestsInFlight = 0;
  let savedHeightAt: VantageViewer['controls']['heightAt'] | undefined;

  function bedrockTileStatus(
    displayName: string,
    stats: { loaded: number; loading: number; total: number; lowres?: number },
  ): string {
    if (stats.total === 0) return `No saved terrain in ${displayName}`;
    const loading = stats.loading > 0 ? ` · ${stats.loading} loading` : '';
    const overview = stats.lowres ? ` · ${stats.lowres} overview tiles` : '';
    return `Saved ${displayName} terrain · ${stats.loaded} detailed / ${stats.total} saved tiles${overview}${loading}`;
  }

  $: selected = dimensions.find((dimension) => dimension.id === selectedDimension);

  function say(value: string): void {
    message = value;
    if (messageTimer) clearTimeout(messageTimer);
    messageTimer = setTimeout(() => (message = ''), 4000);
  }

  function disposeViewer(): void {
    stopFollowing();
    releaseDesktopLook();
    playerLayer?.dispose();
    playerLayer = undefined;
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
    status = serverType === 'bedrock'
      ? 'Preparing saved Bedrock terrain and verified textures…'
      : `Preparing ${entry.displayName} terrain and Minecraft textures…`;
    let opening: VantageViewer | undefined;
    let bedrockStats: { loaded: number; loading: number; total: number; lowres?: number } | undefined;
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
      const { VantageViewer: Viewer, PlayerLayer: Layer } =
        await import('@thoughts-on-things/vantage-mc/three');
      if (!alive || generation !== loadGeneration) return;
      opening = new Viewer(canvas, {
        players: { enabled: false },
        urlState: false,
        // The Bedrock agent serializes tile exports while extending its shared
        // texture atlas. Match that limit instead of queueing six blocked HTTP
        // requests that all appear to be rendering at once.
        ...(serverType === 'bedrock'
          ? {
              streaming: {
                concurrency: 1,
                maxBytes: 512 * 1024 * 1024,
                maxTiles: 120,
                // The whole-world overview replaces remembered screenshots.
                mapMemory: 0,
              },
            }
          : {}),
      });
      if (serverType === 'bedrock') {
        opening.on('stats', (stats) => {
          if (!alive || generation !== loadGeneration) return;
          bedrockStats = stats;
          status = bedrockTileStatus(entry.displayName, stats);
        });
      }
      const source: WorldSource = {
        label: `${worldName} · ${entry.displayName}`,
        manifest,
        fetch: read,
      };
      await opening.load({ world: source });
      if (!alive || generation !== loadGeneration) return;
      viewer = opening;
      opening = undefined;
      const range = viewer.sliceRange;
      depthMin = Math.ceil(range.min + 2);
      depthMax = Math.max(depthMin, Math.floor(range.max));
      if (dimension === 'minecraft:the_nether') {
        depthMax = Math.max(depthMin, Math.min(126, depthMax));
      }
      depthY = depthMax;
      playerLayer = new Layer({ scene: viewer.scene, camera: viewer.camera });
      applyPlayers();
      viewer.controls.addEventListener('start', () => {
        if (followedId) stopFollowing();
      });
      const worldSpawn = manifest.spawn;
      spawn = worldSpawn && [worldSpawn.x, worldSpawn.y, worldSpawn.z].every(Number.isFinite)
        ? worldSpawn : undefined;
      status = serverType === 'bedrock'
        ? bedrockStats
          ? bedrockTileStatus(entry.displayName, bedrockStats)
          : `Rendering saved ${entry.displayName} terrain tiles…`
        : `Saved ${entry.displayName} terrain`;
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
      const moved = playerLayer?.update(performance.now());
      if (followedId && playerLayer) {
        const position = playerLayer.positionOf(followedId);
        if (position) {
          position.y += 0.9;
          if (viewer.controls.position.distanceToSquared(position) > 0.000001) {
            viewer.controls.position.copy(position);
            viewer.invalidate();
          }
        }
      } else if (moved) {
        viewer.invalidate();
      }
      const point =
        viewer.controls.mode === 'fly' ? viewer.camera.position : viewer.controls.position;
      coords = `XYZ ${Math.floor(point.x)}, ${Math.floor(point.y)}, ${Math.floor(point.z)}`;
      flying = viewer.isFlying;
      topDown = !flying && viewer.tilt <= 0.001;
    }
    frameId = requestAnimationFrame(updateToolbar);
  }

  function setDepth(y: number): void {
    if (!viewer) return;
    stopFollowing();
    // Surface-following would lift the focus back above the selected cave level.
    holdFocusHeight();
    depthY = Math.max(depthMin, Math.min(depthMax, Math.round(y)));
    viewer.setSlice(depthY);
    viewer.controls.position.y = depthY;
    viewer.invalidate();
  }

  function leaveFly(): void {
    releaseDesktopLook();
    viewer?.setFlyMode(false);
  }

  function stopFollowing(): void {
    followedId = undefined;
    playerLayer?.setFollowed(null);
    if (viewer && savedHeightAt !== undefined) {
      viewer.controls.heightAt = savedHeightAt;
      savedHeightAt = undefined;
    }
    viewer?.invalidate();
  }

  function holdFocusHeight(): void {
    if (!viewer || savedHeightAt !== undefined) return;
    savedHeightAt = viewer.controls.heightAt;
    viewer.controls.heightAt = null;
  }

  function applyPlayers(): void {
    if (!playerLayer) return;
    const snapshot: PlayerSnapshot = {
      source: 'host',
      updated: Date.now(),
      players: livePlayers.map((player) => ({
        uuid: player.id,
        name: player.name,
        dimension: player.dimension,
        x: player.x,
        y: player.y,
        z: player.z,
        yaw: player.yaw,
        pitch: player.pitch,
        foreign: player.dimension !== selectedDimension,
        stale: false,
      })),
    };
    playerLayer.setSnapshot(snapshot, performance.now());
    viewer?.invalidate();
  }

  async function refreshPlayers(): Promise<void> {
    if (!api || playerRequestsInFlight >= 2) return;
    const request = ++playerRequest;
    playerRequestsInFlight += 1;
    try {
      const feed = await api.get<LiveFeed>('/v1/worlds/map/players');
      if (!alive || request <= lastAppliedPlayerRequest) return;
      lastAppliedPlayerRequest = request;
      if (feed.serverId !== serverId) {
        livePlayers = [];
        playerFeedStatus = 'Select this server to see live players';
        stopFollowing();
        applyPlayers();
        return;
      }
      if (!feed.fresh) {
        livePlayers = [];
        playerFeedStatus =
          feed.status === 'server-stopped' ? 'Server stopped' : 'Live positions unavailable';
        stopFollowing();
      } else {
        livePlayers = feed.players;
        playerFeedStatus = `${feed.players.length} live player${feed.players.length === 1 ? '' : 's'}`;
        const followed = feed.players.find((player) => player.id === followedId);
        if (followedId && !followed) stopFollowing();
        if (followed && followed.dimension !== selectedDimension && !changingPlayerDimension) {
          const target = dimensions.find((entry) => entry.id === followed.dimension);
          if (target?.state === 'ready') {
            changingPlayerDimension = true;
            void loadDimension(target.id)
              .then(() => {
                if (alive) void followPlayer(followed.id);
              })
              .finally(() => (changingPlayerDimension = false));
          } else {
            stopFollowing();
            say('That player entered a dimension without saved terrain');
          }
        }
      }
      applyPlayers();
    } catch {
      if (!alive || request <= lastAppliedPlayerRequest) return;
      lastAppliedPlayerRequest = request;
      livePlayers = [];
      playerFeedStatus = 'Live player feed unavailable';
      stopFollowing();
      applyPlayers();
    } finally {
      playerRequestsInFlight -= 1;
    }
  }

  async function focusPlayer(player: LivePlayer, follow: boolean): Promise<void> {
    if (player.dimension !== selectedDimension) {
      const target = dimensions.find((entry) => entry.id === player.dimension);
      if (target?.state !== 'ready') {
        say('That dimension has no saved terrain yet');
        return;
      }
      await loadDimension(target.id);
    }
    if (
      !livePlayers.some((entry) => entry.id === player.id && entry.dimension === selectedDimension)
    )
      return;
    if (!viewer || !playerLayer) return;
    leaveFly();
    stopFollowing();
    viewer.setSlice(null);
    const target =
      playerLayer.positionOf(player.id) ??
      viewer.controls.position.clone().set(player.x, player.y, player.z);
    target.y += 0.9;
    const toward = target.clone().sub(viewer.camera.position);
    const distance = Math.max(toward.length(), 0.001);
    const state = {
      position: target,
      distance: viewer.controls.distance > 260 ? 140 : viewer.controls.distance,
      rotation: Math.atan2(toward.x, -toward.z),
      angle: Math.acos(Math.max(-1, Math.min(1, -toward.y / distance))),
    };
    viewer.controls.setMode('map');
    if (follow) {
      holdFocusHeight();
      followedId = player.id;
      playerLayer.setFollowed(player.id);
      viewer.controls.setView(state);
    } else {
      viewer.controls.animateTo(state);
    }
    viewer.invalidate();
  }

  async function followPlayer(id: string): Promise<void> {
    const player = livePlayers.find((entry) => entry.id === id);
    if (player) await focusPlayer(player, true);
  }

  function toggleFly(): void {
    if (!viewer) return;
    stopFollowing();
    viewer.setSlice(null);
    const controls = viewer.controls;
    const focus = controls.position.clone();
    const groundY = controls.heightAt?.(focus.x, focus.z) ?? focus.y;
    viewer.toggleFly();
    if (!viewer.isFlying) {
      releaseDesktopLook();
      return;
    }
    // A streamed world opens in a high overview. Keeping that eye position
    // when entering Fly starts hundreds of blocks above the terrain, unlike
    // the close-up proof tile. Begin near the map focus at player eye height.
    if (controls.position.y - groundY > 32) {
      controls.position.set(focus.x, groundY + 2, focus.z);
    }
    controls.angle = Math.PI / 2;
    viewer.invalidate();
    say('Click the map to look · WASD move · Space up · Shift down');
  }

  function releaseDesktopLook(): void {
    if (!desktopLook) return;
    desktopLook = false;
    lookClick = undefined;
    lastPointer = undefined;
    warping = false;
    ignoreWarpUntil = 0;
    resyncPointer = false;
    void getCurrentWindow()
      .setCursorVisible(true)
      .catch(() => {});
  }

  async function centerDesktopPointer(): Promise<void> {
    if (!desktopLook || warping) return;
    warping = true;
    const rect = canvas.getBoundingClientRect();
    try {
      const windowHandle = getCurrentWindow();
      const center = { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
      // macOS Tauri measures cursor positions from the content area already.
      // Adding the title-bar inset here moves the cursor away from the map
      // center and lets the warp appear as an unintended camera turn.
      lastPointer = center;
      ignoreWarpUntil = performance.now() + 120;
      resyncPointer = true;
      await windowHandle.setCursorPosition(new LogicalPosition(center.x, center.y));
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
      try {
        await getCurrentWindow().setCursorVisible(false);
        if (!viewer?.isFlying) {
          await getCurrentWindow().setCursorVisible(true);
          return;
        }
        desktopLook = true;
        await centerDesktopPointer();
      } catch {
        releaseDesktopLook();
        say('Mouse capture is unavailable in this desktop window. Drag to look.');
      }
    }, 80);
  }

  function moveDesktopLook(event: PointerEvent): void {
    if (!desktopLook || !viewer?.isFlying || viewer.controls.isPointerLocked) return;
    if (warping || performance.now() < ignoreWarpUntil) return;
    if (resyncPointer) {
      lastPointer = { x: event.clientX, y: event.clientY };
      resyncPointer = false;
      return;
    }
    if (!lastPointer) {
      lastPointer = { x: event.clientX, y: event.clientY };
      return;
    }
    const dx = event.clientX - lastPointer.x;
    const dy = event.clientY - lastPointer.y;
    lastPointer = { x: event.clientX, y: event.clientY };
    const controls = viewer.controls;
    const sensitivity = 1.5 / Math.max(canvas.clientHeight, 1);
    controls.rotation += dx * sensitivity;
    controls.angle = Math.max(0.02, Math.min(Math.PI - 0.02, controls.angle - dy * sensitivity));
    viewer.invalidate();
    const rect = canvas.getBoundingClientRect();
    if (
      event.clientX < rect.left + 48 ||
      event.clientX > rect.right - 48 ||
      event.clientY < rect.top + 48 ||
      event.clientY > rect.bottom - 48
    ) {
      void centerDesktopPointer();
    }
  }

  function goHome(): void {
    if (!viewer || !spawn) return;
    stopFollowing();
    leaveFly();
    viewer.setSlice(null);
    depthY = depthMax;
    viewer.controls.animateTo({
      position: viewer.controls.position.clone().set(spawn.x + 0.5, spawn.y, spawn.z + 0.5),
      distance: 140,
    });
    viewer.invalidate();
  }

  async function refreshTerrain(): Promise<void> {
    if (!api || refreshing || !viewer || !selectedDimension) return;
    const dimension = selectedDimension;
    refreshing = true;
    say('Capturing current terrain…');
    try {
      const started = await api.post<{ operationId: string }>('/v1/worlds/map/refresh', {});
      const operation = await pollOperation(api, started.operationId);
      if (!alive || dimension !== selectedDimension) return;
      if (operation?.state !== 'succeeded') {
        say(operation?.error?.message ?? 'Terrain refresh failed.');
        return;
      }
      await loadDimension(dimension);
      if (alive) say('Current terrain loaded');
    } catch (error) {
      if (alive) say(error instanceof Error ? error.message : 'Terrain refresh failed.');
    } finally {
      if (alive) refreshing = false;
    }
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
    void refreshPlayers();
    playerPoll = setInterval(() => void refreshPlayers(), 1000);
    frameId = requestAnimationFrame(updateToolbar);
    return () => {
      alive = false;
      ++playerRequest;
      ++loadGeneration;
      cancelAnimationFrame(frameId);
      if (messageTimer) clearTimeout(messageTimer);
      if (playerPoll) clearInterval(playerPoll);
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
    {#if dimensions.length > 0}
      <div class="map-actions">
        <button
          type="button"
          class="refresh"
          disabled={!viewer || refreshing}
          onclick={refreshTerrain}>{refreshing ? 'Refreshing…' : 'Refresh terrain'}</button
        >
        <div class="dimension-picker">
          <span>Dimension</span>
          <Select
            ariaLabel="Dimension"
            width="215px"
            value={selectedDimension}
            options={dimensions.map((dimension) => ({
              value: dimension.id,
              label: `${dimension.displayName}${dimension.state === 'ready' ? '' : ' · unavailable'}`,
            }))}
            onchange={(value) => void loadDimension(value)}
          />
        </div>
      </div>
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
    <details class="players-panel" open>
      <summary class="players-heading">
        <span class="panel-caret" aria-hidden="true">▾</span>
        <strong>Players</strong><span>{livePlayers.length}</span>
      </summary>
      {#if livePlayers.length === 0}
        <p>{playerFeedStatus}</p>
      {:else}
        <ul>
          {#each livePlayers as player (player.id)}
            <li>
              <div class="player-name">
                <span>{player.name}</span>
                {#if player.dimension !== selectedDimension}<small
                    >{dimensions.find((entry) => entry.id === player.dimension)?.displayName ??
                      player.dimension}</small
                  >{/if}
              </div>
              <div class="player-actions">
                <button
                  type="button"
                  disabled={!viewer}
                  aria-label={`Fly to ${player.name}`}
                  onclick={() => void focusPlayer(player, false)}>Fly</button
                >
                <button
                  type="button"
                  disabled={!viewer}
                  aria-label={`${followedId === player.id ? 'Stop following' : 'Follow'} ${player.name}`}
                  aria-pressed={followedId === player.id}
                  onclick={() =>
                    followedId === player.id ? stopFollowing() : void focusPlayer(player, true)}
                  >{followedId === player.id ? 'Following' : 'Follow'}</button
                >
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </details>
    {#if desktopLook}
      <div class="look-indicator" role="status">Mouse look active · Esc releases pointer</div>
    {/if}
    <details class="map-caption" open>
      <summary aria-label="Terrain status" title="Show or hide terrain status">
        <span class="terrain-caret" aria-hidden="true">▴</span>
        <span class="terrain-caption-text" role="status">
          {status}{message ? ` · ${message}` : ''}
        </span>
      </summary>
    </details>
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
          stopFollowing();
          leaveFly();
          if (viewer) {
            // Cancel any pending camera tilt and use a vertical view now.
            // The animated flatten path could still show block side faces
            // after the toolbar had already switched to 2D.
            const controls = viewer.controls;
            if (selectedDimension === 'minecraft:the_nether') {
              setDepth(controls.position.y > 120 ? 83 : controls.position.y);
            } else {
              setDepth(viewer.slice ?? depthMax);
            }
            controls.setView({
              position: controls.position.clone(),
              distance: controls.distance,
              angle: 0,
              rotation: 0,
            });
            viewer.invalidate();
          }
        }}>2D</button
      >
      <button
        type="button"
        class:chosen={!topDown && !flying}
        disabled={!viewer}
        aria-pressed={!topDown && !flying}
        onclick={() => {
          stopFollowing();
          leaveFly();
          viewer?.setSlice(null);
          viewer?.setTilt(0.42);
        }}>3D</button
      >
      <button
        type="button"
        class:chosen={flying}
        disabled={!viewer}
        aria-pressed={flying}
        onclick={toggleFly}>Fly</button
      >
      {#if viewer && topDown && !flying}
        <label class="map-depth">
          Depth Y
          <input
            type="range"
            min={depthMin}
            max={depthMax}
            step="1"
            value={depthY}
            oninput={(event) => setDepth(Number(event.currentTarget.value))}
          />
          <output>{depthY}</output>
        </label>
      {/if}
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
  .map-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .refresh {
    padding: 7px 10px;
    color: #d7dce1;
    background: #242428;
    border: 1px solid #48484e;
    border-radius: 5px;
    font: inherit;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
  }
  .refresh:disabled {
    opacity: 0.45;
    cursor: default;
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
    width: 100%;
    height: 100%;
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
  .players-panel:not([open]) {
    width: fit-content;
  }
  .players-panel summary,
  .map-caption summary {
    list-style: none;
    cursor: pointer;
  }
  .players-panel summary::-webkit-details-marker,
  .map-caption summary::-webkit-details-marker {
    display: none;
  }
  .panel-caret {
    display: inline-block;
    transition: transform 120ms ease;
  }
  details:not([open]) > summary .panel-caret {
    transform: rotate(-90deg);
  }
  .players-panel strong {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .players-heading {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .players-heading strong {
    flex: 1;
  }
  .players-heading span {
    color: #aeb4bb;
    font-size: 11px;
  }
  .players-panel p {
    margin: 8px 0 0;
    color: #aeb4bb;
    font-size: 11px;
    line-height: 1.4;
  }
  .players-panel ul {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    max-height: 45vh;
    overflow-y: auto;
  }
  .players-panel li {
    padding: 8px 0;
    border-top: 1px solid #3a3a40;
  }
  .player-name {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
  }
  .player-name small {
    color: #aeb4bb;
    font-size: 10px;
  }
  .player-actions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .player-actions button {
    background: #242428;
    border: 1px solid #48484e;
    border-radius: 4px;
    color: #f1f1f2;
    font: inherit;
    font-size: 11px;
    padding: 4px 7px;
    cursor: pointer;
  }
  .player-actions button[aria-pressed='true'] {
    background: #244b71;
    border-color: #568fc5;
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
  .map-caption .terrain-caret {
    display: flex;
    width: 20px;
    min-height: 20px;
    align-items: center;
    justify-content: center;
  }
  .map-caption[open] .terrain-caret,
  .map-caption:not([open]) .terrain-caption-text {
    display: none;
  }
  .look-indicator {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 12px;
    background: #141417;
    color: #f1f1f2;
    border: 1px solid #3a3a40;
    border-radius: 5px;
    font-size: 11px;
    pointer-events: none;
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
  .map-depth {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    font-size: 11px;
    color: #c3cad1;
  }
  .map-depth input {
    width: 90px;
  }
  .map-depth output {
    min-width: 22px;
    font-variant-numeric: tabular-nums;
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
