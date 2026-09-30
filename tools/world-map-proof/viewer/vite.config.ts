import { defineConfig } from 'vite';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { resolve } from 'node:path';

const cli = resolve(process.cwd(), '../../../target/debug/msc');

const output = process.env.MSC_WORLD_MAP_PROOF_OUTPUT;
if (!output) throw new Error('Set MSC_WORLD_MAP_PROOF_OUTPUT to the private generated tile directory');

export default defineConfig({
  publicDir: output,
  server: { fs: { strict: true } },
  plugins: [{
    name: 'local-map-player-proof',
    configureServer(server) {
      let latest = '';
      let receivedAt = 0;
      let child: ChildProcessWithoutNullStreams | undefined;
      let retry: ReturnType<typeof setTimeout> | undefined;
      let closing = false;
      function connect() {
        if (closing) return;
        // Keep one authorized CLI connection rather than minting a new
        // five-minute credential for each browser poll.
        child = spawn(cli, ['--json', 'world', 'map-players', '--follow']);
        let buffer = '';
        child.stdout.setEncoding('utf8');
        child.stdout.on('data', (chunk: string) => {
          buffer += chunk;
          if (buffer.length > 128 * 1024) buffer = '';
          let newline = buffer.indexOf('\n');
          while (newline >= 0) {
            const line = buffer.slice(0, newline).trim();
            buffer = buffer.slice(newline + 1);
            try {
              const parsed = JSON.parse(line);
              if (typeof parsed.fresh === 'boolean' && Array.isArray(parsed.players)) {
                latest = line;
                receivedAt = Date.now();
              }
            } catch { /* Ignore partial or diagnostic lines. */ }
            newline = buffer.indexOf('\n');
          }
        });
        child.on('error', () => { latest = ''; });
        child.on('close', () => {
          latest = '';
          if (!closing) retry = setTimeout(connect, 10_000);
        });
      }
      connect();
      server.httpServer?.on('close', () => {
        closing = true;
        if (retry) clearTimeout(retry);
        child?.kill();
      });
      server.middlewares.use('/live-players.json', (_request, response) => {
        response.setHeader('Cache-Control', 'no-store');
        response.setHeader('Content-Type', 'application/json');
        if (latest && Date.now() - receivedAt < 3000) {
          response.end(latest);
        } else {
          response.end(JSON.stringify({ source: 'bds-behavior-pack', fresh: false, status: 'agent-unavailable', players: [] }));
        }
      });
    },
  }],
});
