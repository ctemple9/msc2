import { cpSync, mkdirSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const clientRoot = fileURLToPath(new URL('..', import.meta.url));
const dist = join(clientRoot, 'dist');
const destination = resolve(clientRoot, '../../crates/msc-agent/web-ui');

rmSync(destination, { recursive: true, force: true });
mkdirSync(destination, { recursive: true });
cpSync(dist, destination, { recursive: true });

console.log('packaged the production Svelte output for msc-agent');
