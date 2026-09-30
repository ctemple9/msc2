import { VantageViewer } from '@thoughts-on-things/vantage-mc/three';
import './style.css';

const container = document.querySelector<HTMLDivElement>('#viewer');
const revisionInput = document.querySelector<HTMLInputElement>('#revision');
const replaceButton = document.querySelector<HTMLButtonElement>('#replace');
const status = document.querySelector<HTMLOutputElement>('#revision-status');
if (!container || !revisionInput || !replaceButton || !status) {
  throw new Error('Proof viewer controls missing');
}

const viewer = await VantageViewer.mount(container, {
  tile: '/terrain.vtile',
  textures: '/terrain.vtexarr',
});
status.value = 'Initial saved terrain loaded';

replaceButton.addEventListener('click', async () => {
  const revision = revisionInput.value.trim();
  if (!/^[a-zA-Z0-9_-]+$/.test(revision)) {
    status.value = 'Use letters, numbers, _ or - for the revision';
    return;
  }
  replaceButton.disabled = true;
  status.value = `Loading saved terrain ${revision}…`;
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
    status.value = `Showing saved terrain ${revision}`;
  } catch (error) {
    status.value = `Could not load ${revision}: ${String(error)}`;
    console.error(error);
  } finally {
    replaceButton.disabled = false;
  }
});
