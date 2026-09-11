param(
    [ValidateSet('User', 'Machine')]
    [string]$Scope = 'User'
)

$ErrorActionPreference = 'Stop'

function Fail([string]$Message) {
    Write-Error "msc 2 Windows headless uninstaller: $Message"
    exit 1
}

function Assert-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Fail 'machine removal requires an elevated PowerShell window'
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

$installedBinary = Join-Path $installDirectory 'msc.exe'
$ownershipMarker = Join-Path $installDirectory '.msc2-owned'
if (-not (Test-Path -LiteralPath $ownershipMarker -PathType Leaf)) {
    if (Test-Path -LiteralPath $installedBinary) {
        Fail "refusing to remove an executable without MSC ownership marker: $installedBinary"
    }
    Write-Output "No MSC 2 $Scope headless command installation was found."
    exit 0
}
if ((Get-Content -LiteralPath $ownershipMarker -Raw).Trim() -ne 'msc2-headless-archive') {
    Fail "installation directory has an unrecognized ownership marker: $installDirectory"
}

$oldPath = [Environment]::GetEnvironmentVariable('Path', $environmentTarget)
$entries = @()
if ($oldPath) {
    $entries = @($oldPath -split ';' | Where-Object { $_ -ne '' })
}
$remainingEntries = @($entries | Where-Object { $_.TrimEnd('\') -ine $installDirectory.TrimEnd('\') })
if ($remainingEntries.Count -ne $entries.Count) {
    [Environment]::SetEnvironmentVariable('Path', ($remainingEntries -join ';'), $environmentTarget)
    $pathChange = 'The MSC-owned PATH entry was removed for new shells.'
} else {
    $pathChange = 'The MSC-owned PATH entry was not present.'
}

Remove-Item -LiteralPath $installedBinary, $ownershipMarker -Force -ErrorAction SilentlyContinue
if (Test-Path -LiteralPath $installDirectory -PathType Container) {
    $remaining = Get-ChildItem -LiteralPath $installDirectory -Force
    if (-not $remaining) {
        Remove-Item -LiteralPath $installDirectory -Force
    }
}

Write-Output @"
MSC 2 Windows headless command removed for the $Scope PATH.
$pathChange
Refresh PATH or open a new PowerShell or Command Prompt window before using
the changed command discovery state.

The Windows Service and managed server data were retained.
"@
