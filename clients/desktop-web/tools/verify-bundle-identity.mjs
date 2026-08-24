import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const root = resolve(new URL('..', import.meta.url).pathname);
const dist = join(root, 'dist');
const index = join(dist, 'index.html');
const identity = join(dist, 'bundle-identity.json');
const marker = 'msc2-shared-client';

if (!statSync(dist, { throwIfNoEntry: false })?.isDirectory()) {
  throw new Error('dist/ is missing; run npm run build first');
}

const indexText = readFileSync(index, 'utf8');
const identityText = readFileSync(identity, 'utf8');
const assets = readdirSync(join(dist, 'assets'))
  .filter((entry) => entry.endsWith('.js'))
  .map((entry) => readFileSync(join(dist, 'assets', entry), 'utf8'));

if (
  !indexText.includes('/assets/') ||
  !identityText.includes(marker) ||
  !assets.join('\n').includes(marker)
) {
  throw new Error('production output does not contain the shared bundle identity');
}

console.log(`OK: ${marker} is present in the production bundle and static identity asset`);
