import { defineConfig } from 'vite';
import { execFile } from 'node:child_process';
import { resolve } from 'node:path';
import { promisify } from 'node:util';

const run = promisify(execFile);
const cli = resolve(process.cwd(), '../../../target/debug/msc');

const output = process.env.MSC_WORLD_MAP_PROOF_OUTPUT;
if (!output) throw new Error('Set MSC_WORLD_MAP_PROOF_OUTPUT to the private generated tile directory');

export default defineConfig({
  publicDir: output,
  server: { fs: { strict: true } },
  plugins: [{
    name: 'local-map-player-proof',
    configureServer(server) {
      server.middlewares.use('/live-players.json', async (_request, response) => {
        response.setHeader('Cache-Control', 'no-store');
        response.setHeader('Content-Type', 'application/json');
        try {
          // The CLI performs MSC's local authorization exchange. Browser code
          // never receives an agent credential or calls the agent directly.
          const { stdout } = await run(cli, ['--json', 'world', 'map-players'], {
            timeout: 3000,
            maxBuffer: 128 * 1024,
          });
          response.end(stdout);
        } catch {
          response.statusCode = 503;
          response.end(JSON.stringify({ source: 'bds-behavior-pack', fresh: false, status: 'agent-unavailable', players: [] }));
        }
      });
    },
  }],
});
