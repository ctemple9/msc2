param(
    [switch]$Synthetic
)

$ErrorActionPreference = 'Stop'

if (-not $Synthetic) {
    Write-Error 'Usage: pwsh -File tools/phase10/windows-smoke.ps1 -Synthetic'
    exit 2
}

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Push-Location $root
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'missing required tool: cargo'
    }

    Write-Host '== exercising the synthetic Windows Bedrock HTTP contract =='
    cargo nextest run -p msc-agent `
        --test bedrock_windows_routes `
        --test bedrock_windows_cli
    if ($LASTEXITCODE -ne 0) {
        throw "cargo nextest failed with exit code $LASTEXITCODE"
    }

    Write-Host 'P10.15 WINDOWS SYNTHETIC SMOKE PASSED'
}
finally {
    Pop-Location
}
