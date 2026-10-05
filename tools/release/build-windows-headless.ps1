param(
    [string]$OutputRoot = 'target/release-artifacts'
)

$ErrorActionPreference = 'Stop'

$workspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$rustTarget = 'x86_64-pc-windows-msvc'
$agentPackage = Join-Path $workspaceRoot 'crates/msc-agent/Cargo.toml'
$sourceBinary = Join-Path $workspaceRoot "target/$rustTarget/release/msc.exe"
$packageRoot = Join-Path $workspaceRoot "$OutputRoot/.windows-package"

function Fail([string]$Message) {
    Write-Error "msc 2 Windows headless build: $Message"
    exit 1
}

$versionLine = Select-String -Path $agentPackage -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
$version = $versionLine.Matches[0].Groups[1].Value
if ([string]::IsNullOrWhiteSpace($version)) {
    Fail 'could not read the msc-agent version'
}

Push-Location $workspaceRoot
try {
    & python (Join-Path $workspaceRoot "tools/release/stage-map-capture.py") --build-only
    if ($LASTEXITCODE -ne 0) { Fail "could not build capture adapters" }
    if (-not $env:MSC2_MAP_CAPTURE_HELPERS) { $env:MSC2_MAP_CAPTURE_HELPERS = Join-Path $workspaceRoot "target/map-capture-helpers" }
    cargo build --release --no-default-features --target $rustTarget -p msc-agent
    if ($LASTEXITCODE -ne 0) { Fail "could not build the release agent" }
    cargo build --release --locked --target $rustTarget --manifest-path (Join-Path $workspaceRoot 'tools/world-map-proof/Cargo.toml')
    if ($LASTEXITCODE -ne 0) {
        Fail 'could not build the Bedrock terrain exporter'
    }
    if (-not (Test-Path -Path $sourceBinary -PathType Leaf)) {
        Fail "release binary is missing: $sourceBinary"
    }

    $platformDirectory = Join-Path $workspaceRoot "$OutputRoot/windows"
    $archive = Join-Path $workspaceRoot "$OutputRoot/msc2-headless-$version-windows-x86_64.zip"
    if (Test-Path $packageRoot) {
        Remove-Item -Recurse -Force $packageRoot
    }
    New-Item -ItemType Directory -Force -Path $platformDirectory, $packageRoot | Out-Null
    Copy-Item $sourceBinary (Join-Path $platformDirectory 'msc.exe')
    Copy-Item $sourceBinary (Join-Path $packageRoot 'msc.exe')
    & python (Join-Path $workspaceRoot 'tools/release/stage-map-capture.py') --output-dir $packageRoot
    if ($LASTEXITCODE -ne 0) { Fail 'could not stage capture adapters' }
    $bedrockMap = Join-Path $workspaceRoot "tools/world-map-proof/target/$rustTarget/release/msc-world-map-proof.exe"
    if (-not (Test-Path -LiteralPath $bedrockMap -PathType Leaf)) {
        Fail "Bedrock terrain exporter is missing: $bedrockMap"
    }
    Copy-Item $bedrockMap (Join-Path $packageRoot 'bedrock-map.exe')
    $vantageStager = Join-Path $workspaceRoot 'tools/release/stage-vantage.py'
    & python $vantageStager --platform windows-x86_64 --output-dir $packageRoot
    if ($LASTEXITCODE -ne 0) {
        Fail 'could not stage the pinned Vantage Windows renderer'
    }
    Set-Content -LiteralPath (Join-Path $packageRoot 'MSC2-VERSION') -Value $version
    Copy-Item (Join-Path $workspaceRoot 'packaging/windows/install.ps1') (Join-Path $packageRoot 'install.ps1')
    Copy-Item (Join-Path $workspaceRoot 'packaging/windows/uninstall.ps1') (Join-Path $packageRoot 'uninstall.ps1')
    Copy-Item (Join-Path $workspaceRoot 'docs/msc2/clients/headless-installation.md') (Join-Path $packageRoot 'HEADLESS-INSTALL.md')
    if (Test-Path $archive) {
        Remove-Item -Force $archive
    }
    Compress-Archive -Path (Join-Path $packageRoot '*') -DestinationPath $archive
    Write-Output "built $archive"
}
finally {
    Pop-Location
}
