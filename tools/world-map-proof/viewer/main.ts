import { PlayerLayer, VantageViewer, type PlayerSnapshot } from '@thoughts-on-things/vantage-mc/three';
import './style.css';

const container = document.querySelector<HTMLDivElement>('#viewer');
const revisionInput = document.querySelector<HTMLInputElement>('#revision');
const replaceButton = document.querySelector<HTMLButtonElement>('#replace');
const status = document.querySelector<HTMLOutputElement>('#revision-status');
const playerStatus = document.querySelector<HTMLOutputElement>('#player-status');
if (!container || !revisionInput || !replaceButton || !status || !playerStatus) {
  throw new Error('Proof viewer controls missing');
}

const viewer = await VantageViewer.mount(container, {
  tile: '/terrain.vtile',
  textures: '/terrain.vtexarr',
});
status.value = 'Initial saved terrain loaded';
const players = new PlayerLayer({ scene: viewer.scene, camera: viewer.camera });
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
async function refreshPlayers() {
  try {
    const response = await fetch('/live-players.json', { cache: 'no-store' });
    if (!response.ok) throw new Error('agent unavailable');
    const feed = await response.json() as Feed;
    if (!feed.fresh) {
      players.setSnapshot({ source: 'host', updated: 0, players: [] }, performance.now());
      playerStatus.value = `Player feed unavailable · ${feed.status}`;
      lastSequence = '';
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
    playerStatus.value = `${next.players.length} live Bedrock player${next.players.length === 1 ? '' : 's'}`;
  } catch {
    players.setSnapshot({ source: 'host', updated: 0, players: [] }, performance.now());
    playerStatus.value = 'Player feed unavailable · agent connection lost';
    lastSequence = '';
  }
  viewer.invalidate();
}
void refreshPlayers();
setInterval(() => void refreshPlayers(), 1000);
function animatePlayers(now: number) {
  if (players.update(now)) viewer.invalidate();
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
