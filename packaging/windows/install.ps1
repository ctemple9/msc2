param(
    [ValidateSet('User', 'Machine')]
    [string]$Scope = 'User'
)

$ErrorActionPreference = 'Stop'

function Fail([string]$Message) {
    Write-Error "msc 2 Windows headless installer: $Message"
    exit 1
}

function Assert-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Fail 'machine installation requires an elevated PowerShell window'
    }
}

if (-not $env:LOCALAPPDATA) {
    Fail 'LOCALAPPDATA is not set'
}
if ($Scope -eq 'Machine') {
    Assert-Administrator
    if (-not $env:ProgramFiles) {
        Fail 'ProgramFiles is not set'
    }
    $installDirectory = Join-Path $env:ProgramFiles 'MSC2\bin'
    $environmentTarget = [EnvironmentVariableTarget]::Machine
} else {
    $installDirectory = Join-Path $env:LOCALAPPDATA 'MSC2\bin'
    $environmentTarget = [EnvironmentVariableTarget]::User
}

$sourceBinary = Join-Path $PSScriptRoot 'msc.exe'
$installedBinary = Join-Path $installDirectory 'msc.exe'
$ownershipMarker = Join-Path $installDirectory '.msc2-owned'
if (-not (Test-Path -LiteralPath $sourceBinary -PathType Leaf)) {
    Fail "package binary is missing: $sourceBinary"
}

if (Test-Path -LiteralPath $ownershipMarker -PathType Leaf) {
    $marker = (Get-Content -LiteralPath $ownershipMarker -Raw).Trim()
    if ($marker -ne 'msc2-headless-archive') {
        Fail "installation directory has an unrecognized ownership marker: $installDirectory"
    }
} elseif (Test-Path -LiteralPath $installedBinary) {
    Fail "existing non-MSC executable at $installedBinary"
}

New-Item -ItemType Directory -Force -Path $installDirectory | Out-Null
Copy-Item -LiteralPath $sourceBinary -Destination $installedBinary -Force
[IO.File]::WriteAllText($ownershipMarker, ("msc2-headless-archive" + [Environment]::NewLine))

$oldPath = [Environment]::GetEnvironmentVariable('Path', $environmentTarget)
$entries = @()
if ($oldPath) {
    $entries = @($oldPath -split ';' | Where-Object { $_ -ne '' })
}
$alreadyPresent = $entries | Where-Object { $_.TrimEnd('\') -ieq $installDirectory.TrimEnd('\') }
if (-not $alreadyPresent) {
    $newPath = (($entries + $installDirectory) -join ';')
    [Environment]::SetEnvironmentVariable('Path', $newPath, $environmentTarget)
    $pathChange = 'The PATH entry was added for new shells.'
} else {
    $pathChange = 'The PATH entry was already present.'
}

Write-Output @"
MSC 2 Windows headless command installed for the $Scope PATH.

The executable is installed as $installedBinary and is available as msc.exe.
$pathChange
Refresh PATH or open a new PowerShell or Command Prompt window before using
msc.exe from a shell that was already open.

This installed the CLI only. It did not install, start, stop, or replace the
Windows Service. The service endpoint remains 127.0.0.1:48001.
"@
