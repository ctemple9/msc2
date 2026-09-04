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
    cargo build --release --no-default-features --target $rustTarget -p msc-agent
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
    if (Test-Path $archive) {
        Remove-Item -Force $archive
    }
    Compress-Archive -Path (Join-Path $packageRoot '*') -DestinationPath $archive
    Write-Output "built $archive"
}
finally {
    Pop-Location
}
