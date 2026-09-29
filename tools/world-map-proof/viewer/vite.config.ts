import { defineConfig } from 'vite';

const output = process.env.MSC_WORLD_MAP_PROOF_OUTPUT;
if (!output) throw new Error('Set MSC_WORLD_MAP_PROOF_OUTPUT to the private generated tile directory');

export default defineConfig({
  publicDir: output,
  server: { fs: { strict: true } },
});
