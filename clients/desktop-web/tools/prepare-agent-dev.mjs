import { chmodSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const clientRoot = resolve(new URL('..', import.meta.url).pathname);
const workspaceRoot = resolve(clientRoot, '../..');
const agentName = process.platform === 'win32' ? 'msc.exe' : 'msc';
const arguments_ = process.argv.slice(2);
const unsupportedArguments = arguments_.filter((argument) => argument !== '--release');
if (unsupportedArguments.length > 0) {
  fail(`unsupported argument(s): ${unsupportedArguments.join(', ')}`);
}

const profile = arguments_.includes('--release') ? 'release' : 'debug';
const cargoProfileArguments = profile === 'release' ? ['--release'] : [];
const source = join(workspaceRoot, 'target', profile, agentName);
const destinationRoot = join(clientRoot, 'src-tauri', 'target');
const packageAgentDirectory = join(destinationRoot, 'package', 'agent');

const applianceChecksums = {
  'vmlinuz-kata': '85ac495fce6bb6ee01206c8e022b65acad45ca3fcc2729ba377af33943c8b05e',
  'appliance-initramfs.gz': '4a67a927c406ff45fa64ad00dc1b541a13d8b7bb0a1d40258697c28731166bb2',
};

const version = verifyVersions();
const build = spawnSync('cargo', ['build', '-p', 'msc-agent', ...cargoProfileArguments], {
  cwd: workspaceRoot,
  stdio: 'inherit',
});
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}
if (!existsSync(source)) {
  fail(`expected ${profile} msc-agent executable is missing: ${source}`);
}

const destination =
  process.platform === 'darwin'
    ? join(destinationRoot, 'Resources', 'agent', agentName)
    : join(destinationRoot, profile, 'agent', agentName);

stageFile(source, destination);
stageFile(source, join(packageAgentDirectory, agentName));
console.log(`staged ${profile} msc-agent ${version} at ${destination}`);

if (process.platform === 'darwin') {
  stageMacosSidecar();
}

function stageMacosSidecar() {
  // The verified Intel pair is part of the MSC 2 repository so a macOS
  // developer build does not depend on an external MSC 1 checkout. Keep the
  // environment variable as an explicit override for release or replacement
  // appliance inputs.
  const applianceDirectory =
    process.env.MSC2_BEDROCK_APPLIANCE_DIR ||
    join(workspaceRoot, 'sidecar', 'bedrock', 'Resources');

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
  verifySidecarEntitlement(builtSidecar);
  stageFile(builtSidecar, join(devSidecarDirectory, 'BedrockSidecar'));
  stageFile(builtSidecar, join(packageSidecarDirectory, 'BedrockSidecar'));
  for (const name of Object.keys(applianceChecksums)) {
    stageFile(join(applianceDirectory, name), join(devSidecarDirectory, name));
    stageFile(join(applianceDirectory, name), join(packageSidecarDirectory, name));
  }
  console.log(`staged Intel BedrockSidecar and appliance resources at ${devSidecarDirectory}`);
}

function verifySidecarEntitlement(sidecarPath) {
  const verification = spawnSync('codesign', ['-d', '--entitlements', ':-', sidecarPath], {
    encoding: 'utf8',
  });
  const output = `${verification.stdout}${verification.stderr}`;
  if (
    verification.status !== 0 ||
    !output.includes('com.apple.security.virtualization') ||
    !output.includes('<true/>')
  ) {
    fail(
      `BedrockSidecar was built without com.apple.security.virtualization entitlement: ${sidecarPath}`,
    );
  }
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

function verifyVersions() {
  const packageVersion = readJsonVersion(join(clientRoot, 'package.json'));
  const tauriVersion = readJsonVersion(join(clientRoot, 'src-tauri', 'tauri.conf.json'));
  const shellVersion = readCargoPackageVersion(join(clientRoot, 'src-tauri', 'Cargo.toml'));
  const agentVersion = readCargoPackageVersion(
    join(workspaceRoot, 'crates', 'msc-agent', 'Cargo.toml'),
  );
  const versions = [
    ['desktop package', packageVersion],
    ['Tauri config', tauriVersion],
    ['Tauri shell', shellVersion],
    ['agent', agentVersion],
  ];

  if (new Set(versions.map(([, value]) => value)).size !== 1) {
    fail(`version mismatch: ${versions.map(([name, value]) => `${name}=${value}`).join(', ')}`);
  }

  return packageVersion;
}

function readJsonVersion(path) {
  try {
    const manifest = JSON.parse(readFileSync(path, 'utf8'));
    if (typeof manifest.version !== 'string') {
      fail(`version is missing from ${path}`);
    }
    return manifest.version;
  } catch (error) {
    fail(`could not read version from ${path}: ${error.message}`);
  }
}

function readCargoPackageVersion(path) {
  const manifest = requireFile(path).toString('utf8');
  const match = manifest.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!match) {
    fail(`version is missing from ${path}`);
  }
  return match[1];
}

function fail(message) {
  console.error(`MSC 2 agent packaging failed: ${message}`);
  process.exit(1);
}
