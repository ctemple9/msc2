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

Write-Host "staged Windows release agent from $source"
