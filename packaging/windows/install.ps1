param(
    [ValidateSet('User', 'Machine')]
    [string]$Scope = 'Machine',
    [PSCredential]$ServiceCredential
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
Assert-Administrator
if ($Scope -eq 'Machine') {
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
$sourceVantage = Join-Path $PSScriptRoot 'vantage.exe'
$sourceBedrockMap = Join-Path $PSScriptRoot 'bedrock-map.exe'
$sourceVantageLicense = Join-Path $PSScriptRoot 'VANTAGE-LICENSE.txt'
$captureSource = Join-Path $PSScriptRoot 'map-capture/0.2.0'
$captureNames = @('helpers.json', 'LICENSE', 'DEPENDENCIES.md', 'msc-map-capture-fabric-0.2.0.jar', 'msc-map-capture-forge-0.2.0.jar', 'msc-map-capture-neoforge-0.2.0.jar')
foreach ($path in @((Join-Path $PSScriptRoot 'map-capture'), $captureSource) + @($captureNames | ForEach-Object { Join-Path $captureSource $_ })) {
    if (-not (Test-Path -LiteralPath $path) -or ((Get-Item -LiteralPath $path).Attributes -band [IO.FileAttributes]::ReparsePoint)) { Fail "capture payload is missing or linked: $path" }
}
$installedBinary = Join-Path $installDirectory 'msc.exe'
$installedVantage = Join-Path $installDirectory 'vantage.exe'
$installedBedrockMap = Join-Path $installDirectory 'bedrock-map.exe'
$ownershipMarker = Join-Path $installDirectory '.msc2-owned'
if (-not (Test-Path -LiteralPath $sourceBinary -PathType Leaf)) {
    Fail "package binary is missing: $sourceBinary"
}
if (-not (Test-Path -LiteralPath $sourceVantage -PathType Leaf)) {
    Fail "Vantage renderer is missing: $sourceVantage"
}
if (-not (Test-Path -LiteralPath $sourceBedrockMap -PathType Leaf)) {
    Fail "Bedrock terrain exporter is missing: $sourceBedrockMap"
}
if (-not (Test-Path -LiteralPath $sourceVantageLicense -PathType Leaf)) {
    Fail "Vantage license is missing: $sourceVantageLicense"
}

$serviceName = 'com.ctemple.msc2.agent'
$existingService = & sc.exe qc $serviceName 2>$null
$servicePresent = $LASTEXITCODE -eq 0
if ($servicePresent -and ($existingService -join "`n") -notlike "*$installedBinary*") {
    Fail 'the installed agent service belongs to another MSC installation'
}

if (Test-Path -LiteralPath $ownershipMarker -PathType Leaf) {
    $marker = (Get-Content -LiteralPath $ownershipMarker -Raw).Trim()
    if ($marker -ne 'msc2-headless-archive') {
        Fail "installation directory has an unrecognized ownership marker: $installDirectory"
    }
} elseif (Test-Path -LiteralPath $installedBinary) {
    Fail "existing non-MSC executable at $installedBinary"
}

# Inspect every existing helper path before stopping services or replacing binaries.
$captureDestination = Join-Path $installDirectory 'map-capture/0.2.0'
foreach ($path in @((Join-Path $installDirectory 'map-capture'), $captureDestination)) {
    $item = Get-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    if ($null -ne $item) {
        if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { Fail "installed capture helper directory is not a real directory: $path" }
    }
}
foreach ($name in $captureNames) {
    $path = Join-Path $captureDestination $name
    $item = Get-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
    if ($null -ne $item) {
        if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { Fail "installed capture helper file is not a regular file: $path" }
    }
}

$credential = $ServiceCredential
if (-not $credential) {
    $credential = Get-Credential -Message 'Enter the Windows account and password that will own MSC 2 servers'
}
if (-not $credential) {
    Fail 'a service account credential is required'
}
$account = $credential.UserName
if ($servicePresent -and (Test-Path -LiteralPath $installedBinary -PathType Leaf)) {
    & $installedBinary service stop --service-name $serviceName
    if ($LASTEXITCODE -ne 0) {
        Fail 'could not stop the existing agent before upgrade'
    }
}

New-Item -ItemType Directory -Force -Path $installDirectory | Out-Null
Copy-Item -LiteralPath $sourceBinary -Destination $installedBinary -Force
Copy-Item -LiteralPath $sourceVantage -Destination $installedVantage -Force
Copy-Item -LiteralPath $sourceBedrockMap -Destination $installedBedrockMap -Force
Copy-Item -LiteralPath $sourceVantageLicense -Destination (Join-Path $installDirectory 'VANTAGE-LICENSE.txt') -Force
New-Item -ItemType Directory -Force -Path $captureDestination | Out-Null
foreach ($name in $captureNames) { Copy-Item -LiteralPath (Join-Path $captureSource $name) -Destination (Join-Path $captureDestination $name) -Force }
[IO.File]::WriteAllText($ownershipMarker, ("msc2-headless-archive" + [Environment]::NewLine))

$dataDirectory = Join-Path $env:ProgramData 'MSC2'
New-Item -ItemType Directory -Force -Path $dataDirectory | Out-Null
& icacls.exe $dataDirectory /grant "${account}:(OI)(CI)M" | Out-Null
if ($LASTEXITCODE -ne 0) {
    Fail "could not grant $account access to $dataDirectory"
}
$passwordPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($credential.Password)
try {
    $env:MSC2_WINDOWS_SERVICE_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($passwordPointer)
    & $installedBinary service install --service-name $serviceName `
        --binary-path $installedBinary --working-directory $dataDirectory `
        --log-path (Join-Path $dataDirectory 'agent.log') `
        --run-user $account --expected-port 48001 `
        --env "MSC2_DATA_DIR=$dataDirectory"
    if ($LASTEXITCODE -ne 0) {
        Fail 'Windows Service registration failed'
    }
} finally {
    Remove-Item Env:\MSC2_WINDOWS_SERVICE_PASSWORD -ErrorAction SilentlyContinue
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($passwordPointer)
}
& $installedBinary service start --service-name $serviceName
if ($LASTEXITCODE -ne 0) {
    Fail 'Windows Service registration succeeded but startup failed'
}

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

The Windows Service is enabled at boot and running as $account.
Use msc.exe status agent, msc.exe stop agent, and msc.exe start agent for local control.
"@
