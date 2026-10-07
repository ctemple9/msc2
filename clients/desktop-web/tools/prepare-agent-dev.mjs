import { chmodSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const clientRoot = fileURLToPath(new URL('..', import.meta.url));
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
const vantageName = process.platform === 'win32' ? 'vantage.exe' : 'vantage';
const bedrockMapName = process.platform === 'win32' ? 'bedrock-map.exe' : 'bedrock-map';

const applianceChecksums = {
  'vmlinuz-kata': '85ac495fce6bb6ee01206c8e022b65acad45ca3fcc2729ba377af33943c8b05e',
  'appliance-initramfs.gz': 'd8e0ce4b9a2b9f752e2a482cf52feebaccaac4d892c619f41fa4011d68ac1291',
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
stageVantage();
stageBedrockMap();
if (process.platform === 'win32' && profile === 'release') {
  const helperRoot = join(workspaceRoot, 'packaging', 'windows', 'lifecycle-helper');
  const helper = spawnSync(
    'cargo',
    ['build', '--locked', '--release', '--manifest-path', join(helperRoot, 'Cargo.toml')],
    {
      cwd: workspaceRoot,
      stdio: 'inherit',
      env: { ...process.env, MSC2_MSI_PAYLOAD_DIR: packageAgentDirectory },
    },
  );
  if (helper.status !== 0) fail('could not build the native Windows MSI lifecycle helper');
  stageFile(
    join(helperRoot, 'target', 'release', 'msc2_service_lifecycle.dll'),
    join(destinationRoot, 'package', 'msi', 'msc2-service-lifecycle.dll'),
  );
}

if (process.platform === 'darwin' && process.arch === 'x64') {
  stageMacosSidecar();
} else if (process.platform === 'darwin') {
  // Apple Silicon can run the desktop agent and Java servers, but cannot run
  // the Intel-only Bedrock VM. Remove stale resources before packaging so an
  // arm64 build can never accidentally ship the Intel sidecar.
  rmSync(join(destinationRoot, 'Resources', 'agent', 'sidecar'), {
    recursive: true,
    force: true,
  });
  rmSync(join(packageAgentDirectory, 'sidecar'), { recursive: true, force: true });
  console.log(`staged ${process.arch} macOS agent without Bedrock sidecar`);
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

function stageVantage() {
  const vantagePlatform = {
    'darwin:x64': 'macos-x86_64',
    'darwin:arm64': 'macos-aarch64',
    'linux:x64': 'linux-x86_64',
    'linux:arm64': 'linux-aarch64',
    'win32:x64': 'windows-x86_64',
  }[`${process.platform}:${process.arch}`];
  if (!vantagePlatform) {
    fail(`Vantage has no pinned release binary for ${process.platform}/${process.arch}`);
  }

  const python = process.platform === 'win32' ? 'python' : 'python3';
  const stager = join(workspaceRoot, 'tools', 'release', 'stage-vantage.py');
  const staged = spawnSync(
    python,
    [stager, '--platform', vantagePlatform, '--output-dir', packageAgentDirectory],
    { cwd: workspaceRoot, stdio: 'inherit' },
  );
  if (staged.status !== 0) {
    fail(`could not stage the pinned Vantage ${vantagePlatform} renderer`);
  }

  const sourceVantage = join(packageAgentDirectory, vantageName);
  const sourceLicense = join(packageAgentDirectory, 'VANTAGE-LICENSE.txt');
  const runtimeDirectory =
    process.platform === 'darwin'
      ? join(destinationRoot, 'Resources', 'agent')
      : join(destinationRoot, profile, 'agent');
  stageFile(sourceVantage, join(runtimeDirectory, vantageName));
  stageFile(sourceLicense, join(runtimeDirectory, 'VANTAGE-LICENSE.txt'));
  console.log(`staged Vantage ${vantagePlatform} beside the ${profile} agent`);
}

function stageBedrockMap() {
  const manifest = join(workspaceRoot, 'tools', 'world-map-proof', 'Cargo.toml');
  const built = spawnSync(
    'cargo',
    // Terrain rendering is CPU intensive; ship the same optimized exporter
    // used by the browser proof even when the desktop/agent are debug builds.
    ['build', '--locked', '--release', '--manifest-path', manifest],
    { cwd: workspaceRoot, stdio: 'inherit' },
  );
  if (built.status !== 0) fail('could not build the Bedrock terrain exporter');
  const binary = join(
    workspaceRoot,
    'tools',
    'world-map-proof',
    'target',
    'release',
    process.platform === 'win32' ? 'msc-world-map-proof.exe' : 'msc-world-map-proof',
  );
  if (!existsSync(binary)) fail(`Bedrock terrain exporter is missing: ${binary}`);
  const runtimeDirectory =
    process.platform === 'darwin'
      ? join(destinationRoot, 'Resources', 'agent')
      : join(destinationRoot, profile, 'agent');
  stageFile(binary, join(packageAgentDirectory, bedrockMapName));
  stageFile(binary, join(runtimeDirectory, bedrockMapName));
  console.log(`staged Bedrock terrain exporter beside the ${profile} agent`);
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
  if (process.platform !== 'win32' && destinationPath.endsWith('/vantage')) {
    chmodSync(destinationPath, 0o755);
  }
  if (process.platform !== 'win32' && destinationPath.endsWith('/bedrock-map')) {
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
