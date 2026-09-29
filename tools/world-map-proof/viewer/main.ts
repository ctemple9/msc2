import { VantageViewer } from '@thoughts-on-things/vantage-mc/three';
import './style.css';

const container = document.querySelector<HTMLDivElement>('#viewer');
if (!container) throw new Error('Viewer container missing');

VantageViewer.mount(container, {
  tile: '/terrain.vtile',
  textures: '/terrain.vtexarr',
}).catch((error) => {
  const message = document.createElement('p');
  message.textContent = `BDS terrain could not load: ${String(error)}`;
  document.body.append(message);
  console.error(error);
});
