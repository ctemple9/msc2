[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$WorkspaceRoot,

    [Parameter(Mandatory = $true)]
    [string]$TargetTriple
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$source = Join-Path $WorkspaceRoot "target\$TargetTriple\release\msc.exe"
$tauriTarget = Join-Path $WorkspaceRoot 'clients\desktop-web\src-tauri\target'
$runtimeDestination = Join-Path $tauriTarget 'release\agent\msc.exe'
$packageDestination = Join-Path $tauriTarget 'package\agent\msc.exe'

if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
    throw "expected Windows release agent is missing: $source"
}

foreach ($destination in @($runtimeDestination, $packageDestination)) {
    $directory = Split-Path -Parent $destination
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    Copy-Item -LiteralPath $source -Destination $destination -Force
}

# The desktop uses the same terrain helpers as the standalone headless package.
$mapManifest = Join-Path $WorkspaceRoot 'tools/world-map-proof/Cargo.toml'
Push-Location $WorkspaceRoot
try {
    & cargo build --release --locked --target $TargetTriple --manifest-path $mapManifest
    if ($LASTEXITCODE -ne 0) {
        throw 'could not build the Windows Bedrock terrain exporter'
    }
    $mapSource = Join-Path $WorkspaceRoot "tools/world-map-proof/target/$TargetTriple/release/msc-world-map-proof.exe"
    if (-not (Test-Path -LiteralPath $mapSource -PathType Leaf)) {
        throw "expected Windows Bedrock terrain exporter is missing: $mapSource"
    }
    $packageDirectory = Split-Path -Parent $packageDestination
    & python (Join-Path $WorkspaceRoot 'tools/release/stage-vantage.py') --platform windows-x86_64 --output-dir $packageDirectory
    if ($LASTEXITCODE -ne 0) {
        throw 'could not stage the pinned Vantage Windows renderer'
    }
    Copy-Item -LiteralPath $mapSource -Destination (Join-Path $packageDirectory 'bedrock-map.exe') -Force
    $runtimeDirectory = Split-Path -Parent $runtimeDestination
    foreach ($name in @('bedrock-map.exe', 'vantage.exe', 'VANTAGE-LICENSE.txt')) {
        Copy-Item -LiteralPath (Join-Path $packageDirectory $name) -Destination (Join-Path $runtimeDirectory $name) -Force
    }
}
finally {
    Pop-Location
}

Write-Host "staged Windows release agent and terrain helpers from $WorkspaceRoot"
