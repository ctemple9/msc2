import { chmodSync, cpSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const clientRoot = resolve(new URL('..', import.meta.url).pathname);
const workspaceRoot = resolve(clientRoot, '../..');
const agentName = process.platform === 'win32' ? 'msc.exe' : 'msc';
const source = join(workspaceRoot, 'target', 'debug', agentName);

const build = spawnSync('cargo', ['build', '-p', 'msc-agent'], {
  cwd: workspaceRoot,
  stdio: 'inherit',
});
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const destinationRoot = join(clientRoot, 'src-tauri', 'target');
const destination =
  process.platform === 'darwin'
    ? join(destinationRoot, 'Resources', 'agent', agentName)
    : process.platform === 'win32'
      ? join(destinationRoot, 'debug', 'agent', agentName)
      : join(destinationRoot, 'lib', 'msc2', 'agent', agentName);

mkdirSync(dirname(destination), { recursive: true });
cpSync(source, destination);
if (process.platform !== 'win32') chmodSync(destination, 0o755);
console.log(`staged current msc-agent at ${destination}`);
