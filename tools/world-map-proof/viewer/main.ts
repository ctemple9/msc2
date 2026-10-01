import { PlayerLayer, VantageViewer, type PlayerSnapshot } from '@thoughts-on-things/vantage-mc/three';
import './style.css';

const container = document.querySelector<HTMLDivElement>('#viewer');
const revisionInput = document.querySelector<HTMLInputElement>('#revision');
const replaceButton = document.querySelector<HTMLButtonElement>('#replace');
const status = document.querySelector<HTMLOutputElement>('#revision-status');
const playerStatus = document.querySelector<HTMLOutputElement>('#player-status');
const roster = document.querySelector<HTMLElement>('#player-roster');
const playerCount = document.querySelector<HTMLOutputElement>('#player-count');
const rosterEmpty = document.querySelector<HTMLParagraphElement>('#roster-empty');
const playerList = document.querySelector<HTMLUListElement>('#player-list');
const view2d = document.querySelector<HTMLButtonElement>('#view-2d');
const view3d = document.querySelector<HTMLButtonElement>('#view-3d');
const viewFly = document.querySelector<HTMLButtonElement>('#view-fly');
const coordinates = document.querySelector<HTMLOutputElement>('#map-coordinates');
const zoomOut = document.querySelector<HTMLButtonElement>('#zoom-out');
const zoomIn = document.querySelector<HTMLButtonElement>('#zoom-in');
const homeButton = document.querySelector<HTMLButtonElement>('#map-home');
const cameraButton = document.querySelector<HTMLButtonElement>('#map-camera');
const toolbarMessage = document.querySelector<HTMLOutputElement>('#toolbar-message');
if (!container || !revisionInput || !replaceButton || !status || !playerStatus
  || !roster || !playerCount || !rosterEmpty || !playerList || !view2d || !view3d
  || !viewFly || !coordinates || !zoomOut || !zoomIn || !homeButton || !cameraButton
  || !toolbarMessage) {
  throw new Error('Proof viewer controls missing');
}

const viewer = await VantageViewer.mount(container, {
  tile: '/terrain.vtile',
  textures: '/terrain.vtexarr',
});
status.value = 'Initial saved terrain loaded';
const startingView = {
  position: viewer.controls.position.clone(),
  distance: viewer.controls.distance,
  rotation: viewer.controls.rotation,
  angle: viewer.controls.angle,
  floorY: viewer.controls.floorY,
};
let homeView = startingView;
let homeIsSpawn = false;
let tileOrigin: [number, number] | null = null;
void fetch('/viewer-world.json', { cache: 'no-store' }).then(async (response) => {
  if (!response.ok) return;
  const metadata = await response.json() as {
    spawn?: { x: number; y: number; z: number } | null;
    chunkOrigin?: [number, number];
  };
  const spawn = metadata.spawn;
  const origin = metadata.chunkOrigin;
  if (origin && origin.length === 2 && origin.every(Number.isFinite)) tileOrigin = origin;
  if (!spawn || !origin || ![spawn.x, spawn.y, spawn.z].every(Number.isFinite)) return;
  if (spawn.x < origin[0] * 16 || spawn.x >= (origin[0] + 4) * 16
      || spawn.z < origin[1] * 16 || spawn.z >= (origin[1] + 4) * 16) return;
  const position = startingView.position.clone().set(spawn.x + 0.5, spawn.y, spawn.z + 0.5);
  homeView = { ...startingView, position, floorY: spawn.y };
  homeIsSpawn = true;
  homeButton.title = 'Return to world spawn';
}).catch(() => { /* Older proof tiles have no spawn metadata. */ });
homeButton.title = 'Return to the loaded tile (spawn outside this tile)';
const navigation = document.querySelector<HTMLElement>('#map-toolbar');
navigation?.addEventListener('pointerdown', (event) => event.stopPropagation());
function notifyToolbar(message: string) {
  toolbarMessage.value = message;
  window.setTimeout(() => {
    if (toolbarMessage.value === message) toolbarMessage.value = '';
  }, 3500);
}
function leaveFly() {
  if (viewer.controls.mode === 'fly') viewer.setFlyMode(false);
}
view2d.addEventListener('click', () => {
  stopFollowing();
  leaveFly();
  viewer.flatten();
  viewer.invalidate();
});
view3d.addEventListener('click', () => {
  stopFollowing();
  leaveFly();
  viewer.setTilt(0.42);
  viewer.invalidate();
});
viewFly.addEventListener('click', () => {
  stopFollowing();
  viewer.toggleFly();
  viewer.invalidate();
  if (viewer.isFlying) notifyToolbar('Click the map to look around · WASD move · Space up · Shift down');
});
zoomOut.addEventListener('click', () => { viewer.controls.zoom(-1); viewer.invalidate(); });
zoomIn.addEventListener('click', () => { viewer.controls.zoom(1); viewer.invalidate(); });
homeButton.addEventListener('click', () => {
  stopFollowing();
  leaveFly();
  viewer.controls.animateTo(homeView);
  viewer.invalidate();
  if (!homeIsSpawn) notifyToolbar('Spawn is outside this loaded tile; returned to the tile view');
});
cameraButton.addEventListener('click', () => {
  const link = document.createElement('a');
  link.href = viewer.screenshot();
  link.download = `msc-world-${new Date().toISOString().replace(/[:.]/g, '-')}.png`;
  link.click();
  notifyToolbar('Screenshot saved');
});
function updateToolbar() {
  const controls = viewer.controls;
  const point = controls.mode === 'fly' ? viewer.camera.position : controls.position;
  const outsideTile = tileOrigin && (point.x < tileOrigin[0] * 16
    || point.x >= (tileOrigin[0] + 4) * 16 || point.z < tileOrigin[1] * 16
    || point.z >= (tileOrigin[1] + 4) * 16);
  coordinates.value = `XYZ ${Math.floor(point.x)}, ${Math.floor(point.y)}, ${Math.floor(point.z)}${outsideTile ? ' · outside tile' : ''}`;
  coordinates.classList.toggle('outside-tile', Boolean(outsideTile));
  viewFly.classList.toggle('active', controls.mode === 'fly');
  view3d.classList.toggle('active', controls.mode === 'map' && controls.angle > 0.08);
  view2d.classList.toggle('active', controls.mode === 'map' && controls.angle <= 0.08);
  viewFly.setAttribute('aria-pressed', String(controls.mode === 'fly'));
  view3d.setAttribute('aria-pressed', String(controls.mode === 'map' && controls.angle > 0.08));
  view2d.setAttribute('aria-pressed', String(controls.mode === 'map' && controls.angle <= 0.08));
  requestAnimationFrame(updateToolbar);
}
requestAnimationFrame(updateToolbar);
const players = new PlayerLayer({ scene: viewer.scene, camera: viewer.camera });
let displayedRoster = '';
let followingId: string | null = null;
let savedHeightAt: typeof viewer.controls.heightAt = null;
let hasHeightOverride = false;
type Feed = {
  fresh: boolean;
  status: string;
  sampledAtMs?: number;
  players: Array<{
    id: string; name: string; dimension: string;
    x: number; y: number; z: number; yaw: number; pitch: number;
  }>;
};
let lastSequence = '';
function centerOf(position: typeof viewer.controls.position) {
  const centered = position.clone();
  centered.y += 0.9;
  return centered;
}
function cameraStateFor(target: typeof viewer.controls.position, distanceLimit?: number) {
  // Player roster actions are map navigation even if the viewer was left in
  // free-flight mode, where controls.position means the camera eye, not its
  // look-at pivot.
  if (viewer.controls.mode !== 'map') viewer.controls.setMode('map');
  const towardTarget = target.clone().sub(viewer.camera.position);
  const eyeDistance = Math.max(towardTarget.length(), 0.001);
  const distance = distanceLimit ?? viewer.controls.distance;
  const rotation = Math.atan2(towardTarget.x, -towardTarget.z);
  // MapControls' angle is measured from straight down: a player below the
  // camera has a negative eye-to-target Y delta, so negate it here.
  const angle = Math.acos(Math.max(-1, Math.min(1, -towardTarget.y / eyeDistance)));
  return { position: target, distance, rotation, angle };
}
function holdCameraAtPlayerHeight() {
  if (!hasHeightOverride) {
    savedHeightAt = viewer.controls.heightAt;
    hasHeightOverride = true;
  }
  // MapControls otherwise moves the pivot back toward terrain height on each
  // update, which can push an elevated player away from the viewport center.
  viewer.controls.heightAt = null;
}
function restoreTerrainHeight() {
  if (!hasHeightOverride) return;
  viewer.controls.heightAt = savedHeightAt;
  savedHeightAt = null;
  hasHeightOverride = false;
}
function stopFollowing() {
  followingId = null;
  players.setFollowed(null);
  restoreTerrainHeight();
}
function renderRoster(current: PlayerSnapshot['players']) {
  playerCount.value = String(current.length);
  rosterEmpty.hidden = current.length > 0;
  const signature = current.map((player) => `${player.uuid}\0${player.name}`).join('\n');
  if (signature === displayedRoster) {
    playerList.querySelectorAll<HTMLButtonElement>('[data-follow]').forEach((button) => {
      const active = button.dataset.follow === followingId;
      button.textContent = active ? 'Following' : 'Follow';
      button.classList.toggle('following', active);
      button.setAttribute('aria-pressed', String(active));
    });
    return;
  }
  displayedRoster = signature;
  playerList.replaceChildren();
  for (const player of current) {
    const item = document.createElement('li');
    item.className = 'player-row';
    const name = document.createElement('span');
    name.textContent = player.name;
    const actions = document.createElement('span');
    actions.className = 'player-actions';
    const fly = document.createElement('button');
    fly.type = 'button';
    fly.textContent = 'Fly';
    fly.setAttribute('aria-label', `Fly to ${player.name}`);
    fly.addEventListener('click', () => {
      stopFollowing();
      const position = players.positionOf(player.uuid);
      if (!position) return;
      const target = centerOf(position);
      holdCameraAtPlayerHeight();
      const state = cameraStateFor(target, viewer.controls.distance > 260 ? 140 : undefined);
      viewer.controls.animateTo(state);
      viewer.invalidate();
      renderRoster(players.players.filter((entry) => entry.dimension === 'minecraft:overworld'));
    });
    const follow = document.createElement('button');
    follow.type = 'button';
    follow.dataset.follow = player.uuid;
    follow.addEventListener('click', () => {
      if (followingId === player.uuid) {
        stopFollowing();
      } else {
        stopFollowing();
        followingId = player.uuid;
        const target = players.positionOf(followingId);
        if (target) {
          const centered = centerOf(target);
          holdCameraAtPlayerHeight();
          viewer.controls.setView(cameraStateFor(centered));
        }
      }
      players.setFollowed(followingId);
      renderRoster(players.players.filter((entry) => entry.dimension === 'minecraft:overworld'));
    });
    actions.append(fly, follow);
    item.append(name, actions);
    playerList.append(item);
  }
  renderRoster(current);
}
roster.addEventListener('pointerdown', (event) => event.stopPropagation());
viewer.controls.addEventListener('start', () => {
  if (followingId === null) {
    restoreTerrainHeight();
    return;
  }
  stopFollowing();
  renderRoster(players.players.filter((entry) => entry.dimension === 'minecraft:overworld'));
});
async function refreshPlayers() {
  try {
    const response = await fetch('/live-players.json', { cache: 'no-store' });
    if (!response.ok) throw new Error('agent unavailable');
    const feed = await response.json() as Feed;
    if (!feed.fresh) {
      players.setSnapshot({ source: 'host', updated: 0, players: [] }, performance.now());
      playerStatus.value = `Player feed unavailable · ${feed.status}`;
      lastSequence = '';
      stopFollowing();
      renderRoster([]);
      viewer.invalidate();
      return;
    }
    const next: PlayerSnapshot = {
      source: 'host',
      updated: feed.sampledAtMs ?? 0,
      players: feed.players.map((player) => ({
        uuid: player.id,
        name: player.name,
        x: player.x, y: player.y, z: player.z,
        yaw: player.yaw, pitch: player.pitch,
        dimension: player.dimension,
        foreign: player.dimension !== 'minecraft:overworld',
        stale: false,
      })),
    };
    const signature = JSON.stringify(next);
    if (signature !== lastSequence) {
      players.setSnapshot(next, performance.now());
      lastSequence = signature;
    }
    const localPlayers = next.players.filter((player) => !player.foreign);
    if (followingId && !localPlayers.some((player) => player.uuid === followingId)) {
      followingId = null;
      players.setFollowed(null);
    }
    playerStatus.value = `${next.players.length} live player${next.players.length === 1 ? '' : 's'}`;
    renderRoster(localPlayers);
  } catch {
    players.setSnapshot({ source: 'host', updated: 0, players: [] }, performance.now());
    playerStatus.value = 'Player feed unavailable · agent connection lost';
    lastSequence = '';
    stopFollowing();
    renderRoster([]);
  }
  viewer.invalidate();
}
void refreshPlayers();
setInterval(() => void refreshPlayers(), 1000);
function animatePlayers(now: number) {
  let changed = players.update(now);
  if (followingId) {
    const target = players.positionOf(followingId);
    if (!target) {
      stopFollowing();
      renderRoster(players.players.filter((entry) => entry.dimension === 'minecraft:overworld'));
    } else {
      viewer.controls.position.copy(centerOf(target));
      changed = true;
    }
  }
  if (changed) viewer.invalidate();
  requestAnimationFrame(animatePlayers);
}
requestAnimationFrame(animatePlayers);

replaceButton.addEventListener('click', async () => {
  const revision = revisionInput.value.trim();
  if (!/^[a-zA-Z0-9_-]+$/.test(revision)) {
    status.value = 'Use letters, numbers, _ or - for the revision';
    return;
  }
  replaceButton.disabled = true;
  status.value = `Loading saved terrain ${revision}…`;
  const started = performance.now();
  try {
    // Fetch both files before replacing the scene. An incomplete export leaves
    // the visible tile intact and can be retried with the same name.
    const base = `/revisions/${revision}`;
    const [tileResponse, texturesResponse] = await Promise.all([
      fetch(`${base}/terrain.vtile`, { cache: 'no-store' }),
      fetch(`${base}/terrain.vtexarr`, { cache: 'no-store' }),
    ]);
    if (!tileResponse.ok || !texturesResponse.ok) {
      throw new Error(`revision files unavailable (${tileResponse.status}, ${texturesResponse.status})`);
    }
    const [tile, textures] = await Promise.all([
      tileResponse.arrayBuffer(),
      texturesResponse.arrayBuffer(),
    ]);
    const controls = viewer.controls;
    const view = {
      position: controls.position.clone(),
      distance: controls.distance,
      rotation: controls.rotation,
      angle: controls.angle,
      floorY: controls.floorY,
    };
    await viewer.load({ tile, textures });
    controls.setView(view);
    viewer.invalidate();
    status.value = `Showing saved terrain ${revision} · load ${Math.round(performance.now() - started)} ms`;
  } catch (error) {
    status.value = `Could not load ${revision}: ${String(error)}`;
    console.error(error);
  } finally {
    replaceButton.disabled = false;
  }
});
