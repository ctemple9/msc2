import { chmodSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const clientRoot = resolve(new URL('..', import.meta.url).pathname);
const workspaceRoot = resolve(clientRoot, '../..');
const agentName = process.platform === 'win32' ? 'msc.exe' : 'msc';
const source = join(workspaceRoot, 'target', 'debug', agentName);
const destinationRoot = join(clientRoot, 'src-tauri', 'target');
const packageAgentDirectory = join(destinationRoot, 'package', 'agent');

const applianceChecksums = {
  'vmlinuz-kata': '85ac495fce6bb6ee01206c8e022b65acad45ca3fcc2729ba377af33943c8b05e',
  'appliance-initramfs.gz': '0865eb432f61249a5a2f76770e7c79e53cf803c5fa435d110ced03747da8a278',
};

const build = spawnSync('cargo', ['build', '-p', 'msc-agent'], {
  cwd: workspaceRoot,
  stdio: 'inherit',
});
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const destination =
  process.platform === 'darwin'
    ? join(destinationRoot, 'Resources', 'agent', agentName)
    : process.platform === 'win32'
      ? join(destinationRoot, 'debug', 'agent', agentName)
      : join(destinationRoot, 'debug', 'agent', agentName);

stageFile(source, destination);
stageFile(source, join(packageAgentDirectory, agentName));
console.log(`staged current msc-agent at ${destination}`);

if (process.platform === 'darwin') {
  stageMacosSidecar();
}

function stageMacosSidecar() {
  // The verified Intel pair is part of the MSC 2 repository so a macOS
  // developer build does not depend on an external MSC 1 checkout. Keep the
  // environment variable as an explicit override for release or replacement
  // appliance inputs.
  const applianceDirectory =
    process.env.MSC2_BEDROCK_APPLIANCE_DIR || join(workspaceRoot, 'sidecar', 'bedrock', 'Resources');

  for (const [name, checksum] of Object.entries(applianceChecksums)) {
    const path = join(applianceDirectory, name);
    if (!existsSync(path)) {
      fail(`Bedrock appliance resource is missing: ${path}`);
    }
    const actual = createHash('sha256').update(requireFile(path)).digest('hex');
    if (actual !== checksum) {
      fail(`Bedrock appliance checksum mismatch for ${name}: expected ${checksum}, got ${actual}`);
    }
  }

  const derivedData = join(destinationRoot, 'bedrock-sidecar-build');
  rmSync(derivedData, { recursive: true, force: true });
  const sidecarProject = join(workspaceRoot, 'sidecar', 'bedrock', 'BedrockSidecar.xcodeproj');
  const sidecarBuild = spawnSync(
    'xcodebuild',
    [
      '-project',
      sidecarProject,
      '-scheme',
      'BedrockSidecar',
      '-configuration',
      'Release',
      '-derivedDataPath',
      derivedData,
      'ARCHS=x86_64',
      'ONLY_ACTIVE_ARCH=NO',
      `MSC2_BEDROCK_APPLIANCE_DIR=${applianceDirectory}`,
    ],
    { cwd: workspaceRoot, stdio: 'inherit' },
  );
  if (sidecarBuild.status !== 0) {
    process.exit(sidecarBuild.status ?? 1);
  }

  const builtSidecar = join(derivedData, 'Build', 'Products', 'Release', 'BedrockSidecar');
  if (!existsSync(builtSidecar)) {
    fail(`BedrockSidecar build produced no executable at ${builtSidecar}`);
  }

  const devSidecarDirectory = join(destinationRoot, 'Resources', 'agent', 'sidecar');
  const packageSidecarDirectory = join(packageAgentDirectory, 'sidecar');
  stageFile(builtSidecar, join(devSidecarDirectory, 'BedrockSidecar'));
  stageFile(builtSidecar, join(packageSidecarDirectory, 'BedrockSidecar'));
  for (const name of Object.keys(applianceChecksums)) {
    stageFile(join(applianceDirectory, name), join(devSidecarDirectory, name));
    stageFile(join(applianceDirectory, name), join(packageSidecarDirectory, name));
  }
  console.log(`staged Intel BedrockSidecar and appliance resources at ${devSidecarDirectory}`);
}

function stageFile(sourcePath, destinationPath) {
  mkdirSync(dirname(destinationPath), { recursive: true });
  cpSync(sourcePath, destinationPath);
  if (process.platform !== 'win32' && destinationPath.endsWith('/msc')) {
    chmodSync(destinationPath, 0o755);
  }
  if (process.platform !== 'win32' && destinationPath.endsWith('/BedrockSidecar')) {
    chmodSync(destinationPath, 0o755);
  }
}

function requireFile(path) {
  try {
    return readFileSync(path);
  } catch (error) {
    fail(`Could not read Bedrock appliance resource ${path}: ${error.message}`);
  }
}

function fail(message) {
  console.error(`Bedrock packaging failed: ${message}`);
  process.exit(1);
}
