import { cpSync, existsSync, mkdirSync, rmSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const clientRoot = fileURLToPath(new URL('..', import.meta.url));
const dist = join(clientRoot, 'dist');
const destination = resolve(clientRoot, '../../crates/msc-agent/web-ui');

if (!statSync(dist, { throwIfNoEntry: false })?.isDirectory()) {
  throw new Error('dist/ is missing; run npm run build before staging the agent bundle');
}
if (!existsSync(join(dist, 'index.html')) || !existsSync(join(dist, 'bundle-identity.json'))) {
  throw new Error('dist/ is missing its production entry point or bundle identity');
}

rmSync(destination, { recursive: true, force: true });
mkdirSync(destination, { recursive: true });
cpSync(dist, destination, { recursive: true });

console.log('packaged the production Svelte output for msc-agent');
