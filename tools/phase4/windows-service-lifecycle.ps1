param(
    [Parameter(Mandatory = $false)]
    [string]$ServerDir = $env:MSC2_PHASE4_PAPER_SERVER,
    [string]$RunRoot = "$env:ProgramData\MSC2\phase4\windows-service-lifecycle",
    [switch]$KeepArtifacts
)

$ErrorActionPreference = "Stop"

function Require-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "this check must run from an elevated PowerShell session so it can register a Windows Service"
    }
}

function Require-Tool([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "missing required tool: $Name"
    }
}

function Get-FreePort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = ($listener.LocalEndpoint).Port
    $listener.Stop()
    return $port
}

function Wait-HttpReady([string]$BaseUrl, [int]$TimeoutSeconds = 45) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $response = Invoke-WebRequest -Uri "$BaseUrl/v1/health" -UseBasicParsing -TimeoutSec 2
            if ($response.StatusCode -eq 200) {
                return
            }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "agent did not become healthy at $BaseUrl"
}

function Wait-ServerRunning([string]$Msc, [string]$BaseUrl, [string]$Token, [int]$TimeoutSeconds = 45) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $json = & $Msc --base-url $BaseUrl --token $Token --json status
        if ($LASTEXITCODE -ne 0) {
            throw "status command failed"
        }
        if ((ConvertFrom-Json $json).running) {
            return
        }
        Start-Sleep -Milliseconds 250
    }
    throw "server never reached running state"
}

function Wait-ConsoleReady([string]$Msc, [string]$BaseUrl, [string]$Token, [string]$ServerName, [int]$TimeoutSeconds = 45) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $json = & $Msc --base-url $BaseUrl --token $Token --json console tail --server $ServerName --lines 80
        if ($LASTEXITCODE -ne 0) {
            throw "console tail command failed"
        }
        $lines = ConvertFrom-Json $json
        if ($lines | Where-Object { $_.text -like '*Done (*! For help, type "help"*' }) {
            return
        }
        Start-Sleep -Milliseconds 250
    }
    throw "server never emitted the ready line"
}

function Wait-ConsoleContains([string]$Msc, [string]$BaseUrl, [string]$Token, [string]$ServerName, [string]$Needle, [int]$TimeoutSeconds = 20) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $json = & $Msc --base-url $BaseUrl --token $Token --json console tail --server $ServerName --lines 80
        if ($LASTEXITCODE -ne 0) {
            throw "console tail command failed"
        }
        $lines = ConvertFrom-Json $json
        if ($lines | Where-Object { $_.text -like "*$Needle*" }) {
            return
        }
        Start-Sleep -Milliseconds 250
    }
    throw "console tail never observed: $Needle"
}

function New-ServiceHostScript([string]$Path) {
    $script = @'
param(
    [string]$ServiceName,
    [string]$MscPath,
    [string]$BindAddress,
    [string]$Token,
    [string]$JournalDir,
    [string]$WorkingDirectory,
    [string]$LogPath
)

$ErrorActionPreference = "Stop"
$crashLogPath = "$LogPath.crash"

try {

Add-Type -ReferencedAssemblies @("System.ServiceProcess", "System.Core") -TypeDefinition @"
using System;
using System.Diagnostics;
using System.IO;
using System.ServiceProcess;

public class MscAgentWindowsService : ServiceBase
{
    private readonly string mscPath;
    private readonly string bindAddress;
    private readonly string token;
    private readonly string journalDir;
    private readonly string workingDirectory;
    private readonly string logPath;
    private Process child;
    private StreamWriter logWriter;

    public MscAgentWindowsService(string serviceName, string mscPath, string bindAddress, string token, string journalDir, string workingDirectory, string logPath)
    {
        ServiceName = serviceName;
        AutoLog = true;
        CanStop = true;
        this.mscPath = mscPath;
        this.bindAddress = bindAddress;
        this.token = token;
        this.journalDir = journalDir;
        this.workingDirectory = workingDirectory;
        this.logPath = logPath;
    }

    protected override void OnStart(string[] args)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(logPath));
        Directory.CreateDirectory(journalDir);
        Directory.CreateDirectory(workingDirectory);

        logWriter = new StreamWriter(new FileStream(logPath, FileMode.Append, FileAccess.Write, FileShare.ReadWrite));
        logWriter.AutoFlush = true;
        WriteLog("starting msc serve on " + bindAddress);

        var startInfo = new ProcessStartInfo(mscPath, "serve --bind " + bindAddress)
        {
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            WorkingDirectory = workingDirectory,
        };
        startInfo.EnvironmentVariables["MSC2_TEST_BOOTSTRAP_TOKEN"] = token;
        startInfo.EnvironmentVariables["MSC2_OPERATION_JOURNAL_DIR"] = journalDir;

        child = new Process();
        child.StartInfo = startInfo;
        child.EnableRaisingEvents = true;
        child.OutputDataReceived += (sender, eventArgs) =>
        {
            if (eventArgs.Data != null)
            {
                WriteLog("[stdout] " + eventArgs.Data);
            }
        };
        child.ErrorDataReceived += (sender, eventArgs) =>
        {
            if (eventArgs.Data != null)
            {
                WriteLog("[stderr] " + eventArgs.Data);
            }
        };
        child.Exited += (sender, eventArgs) =>
        {
            WriteLog("msc serve exited with code " + child.ExitCode);
        };

        child.Start();
        child.BeginOutputReadLine();
        child.BeginErrorReadLine();
    }

    protected override void OnStop()
    {
        try
        {
            if (child != null && !child.HasExited)
            {
                WriteLog("stopping msc serve child");
                child.Kill();
                child.WaitForExit(15000);
            }
        }
        finally
        {
            if (logWriter != null)
            {
                logWriter.Dispose();
                logWriter = null;
            }
        }
    }

    private void WriteLog(string line)
    {
        if (logWriter != null)
        {
            logWriter.WriteLine(DateTime.UtcNow.ToString("o") + " " + line);
        }
    }

    public static void RunService(string serviceName, string mscPath, string bindAddress, string token, string journalDir, string workingDirectory, string logPath)
    {
        ServiceBase.Run(new MscAgentWindowsService(serviceName, mscPath, bindAddress, token, journalDir, workingDirectory, logPath));
    }
}
"@

[MscAgentWindowsService]::RunService($ServiceName, $MscPath, $BindAddress, $Token, $JournalDir, $WorkingDirectory, $LogPath)

} catch {
    $crashDir = Split-Path -Parent $crashLogPath
    if ($crashDir -and -not (Test-Path $crashDir)) {
        New-Item -ItemType Directory -Path $crashDir -Force | Out-Null
    }
    "$(Get-Date -Format o) FATAL in service host script:" | Add-Content -Path $crashLogPath
    ($_ | Out-String) | Add-Content -Path $crashLogPath
    ($_.ScriptStackTrace | Out-String) | Add-Content -Path $crashLogPath
    throw
}
'@
    Set-Content -Path $Path -Value $script -Encoding UTF8
}

function Resume-Checkpoint([string]$CheckpointPath) {
    $checkpoint = Get-Content -Path $CheckpointPath -Raw | ConvertFrom-Json
    $msc = $checkpoint.MscPath
    $baseUrl = $checkpoint.BaseUrl
    $token = $checkpoint.Token
    $serviceName = $checkpoint.ServiceName
    $serverName = $checkpoint.ServerName

    try {
        $service = Get-Service -Name $serviceName -ErrorAction Stop
        if ($service.Status -ne "Running") {
            throw "Windows Service $serviceName is not running after sign-out"
        }
        Wait-HttpReady -BaseUrl $baseUrl -TimeoutSeconds 45
        Wait-ServerRunning -Msc $msc -BaseUrl $baseUrl -Token $token -TimeoutSeconds 45
        Write-Host "sign-out checkpoint: service survived and API/server are reachable"

        & $msc --base-url $baseUrl --token $token server stop $serverName | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "server stop failed during checkpoint resume"
        }
        Stop-Service -Name $serviceName -ErrorAction Stop
        sc.exe delete $serviceName | Out-Null

        if (-not $KeepArtifacts) {
            Remove-Item -LiteralPath $checkpoint.RunDir -Recurse -Force -ErrorAction SilentlyContinue
        }
        Remove-Item -LiteralPath $CheckpointPath -Force -ErrorAction SilentlyContinue
        Write-Host "windows service lifecycle check complete"
    } catch {
        throw
    }
}

if ($PSVersionTable.PSEdition -eq "Core" -and -not $IsWindows) {
    throw "this check only runs on Windows"
}

Require-Admin
Require-Tool cargo
Require-Tool sc.exe

$checkpointPath = Join-Path $RunRoot "checkpoint.json"
if (Test-Path $checkpointPath) {
    Resume-Checkpoint -CheckpointPath $checkpointPath
    exit 0
}

if ([string]::IsNullOrWhiteSpace($ServerDir)) {
    throw "-ServerDir is required or MSC2_PHASE4_PAPER_SERVER must be set"
}
if (-not (Test-Path (Join-Path $ServerDir "server.properties"))) {
    throw "server directory is missing server.properties: $ServerDir"
}

New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$runId = Get-Date -Format "yyyyMMddHHmmss"
$runDir = Join-Path $RunRoot "run-$runId"
$logsDir = Join-Path $runDir "logs"
$journalDir = Join-Path $runDir "journal"
$stateDir = Join-Path $runDir "state"
$hostScript = Join-Path $runDir "msc-agent-service-host.ps1"
$serviceLog = Join-Path $logsDir "service-host.log"
$serviceName = "msc2-phase4-agent-$runId"
$serverName = "Phase4 Paper"
$port = Get-FreePort
$baseUrl = "http://127.0.0.1:$port"
$token = "msc2_phase4_windows_service_secret"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$msc = Join-Path $root "target\debug\msc.exe"

New-Item -ItemType Directory -Path $runDir, $logsDir, $journalDir, $stateDir -Force | Out-Null
New-ServiceHostScript -Path $hostScript

Push-Location $root
try {
    cargo build -p msc-agent | Out-Null
} finally {
    Pop-Location
}

$serviceBinary = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
$innerCommand = "`"$serviceBinary`" -NoProfile -ExecutionPolicy Bypass -File `"$hostScript`" -ServiceName `"$serviceName`" -MscPath `"$msc`" -BindAddress `"127.0.0.1:$port`" -Token `"$token`" -JournalDir `"$journalDir`" -WorkingDirectory `"$stateDir`" -LogPath `"$serviceLog`""
# powershell.exe launched directly as a service's ImagePath hangs indefinitely: the
# Service Control Manager gives the process no console and no redirected standard
# handles, and Windows PowerShell's interactive console-host initialization blocks
# forever trying to interact with a console that doesn't exist, so ServiceBase.Run()
# is never reached and the service times out after 30s with no error anywhere.
# Routing through a generated .cmd launcher with stdout/stderr redirected to a file
# gives the process valid handles and avoids the hang; confirmed directly against
# real sc.exe/SCM behavior on this host (a bare `New-Service -BinaryPathName
# "powershell.exe ..."` never reaches OnStart, the same command wrapped in
# `cmd.exe /c` with output redirected reaches OnStart immediately).
$rawOutputLog = "$serviceLog.raw"
$launcherPath = Join-Path $runDir "service-launcher.cmd"
$launcherContent = "@echo off`r`n$innerCommand > `"$rawOutputLog`" 2>&1`r`n"
Set-Content -Path $launcherPath -Value $launcherContent -Encoding ASCII -NoNewline
$cmdExe = "$env:SystemRoot\System32\cmd.exe"
$serviceCommand = "`"$cmdExe`" /c `"$launcherPath`""
$credential = Get-Credential -Message "Enter the installing-user credential for the Windows Service Log On As account"

try {
    New-Service -Name $serviceName -BinaryPathName $serviceCommand -Credential $credential -StartupType Automatic -DisplayName $serviceName | Out-Null
    Start-Service -Name $serviceName
    Wait-HttpReady -BaseUrl $baseUrl -TimeoutSeconds 45

    & $msc --base-url $baseUrl --token $token server import $ServerDir --name $serverName | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "server import failed"
    }
    & $msc --base-url $baseUrl --token $token server start $serverName | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "server start failed"
    }

    Wait-ServerRunning -Msc $msc -BaseUrl $baseUrl -Token $token -TimeoutSeconds 45
    Wait-ConsoleReady -Msc $msc -BaseUrl $baseUrl -Token $token -ServerName $serverName -TimeoutSeconds 45
    & $msc --base-url $baseUrl --token $token command --server $serverName "say phase4 windows service check" | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "server command failed"
    }
    Wait-ConsoleContains -Msc $msc -BaseUrl $baseUrl -Token $token -ServerName $serverName -Needle "phase4 windows service check" -TimeoutSeconds 20

    $processes = Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*paper.jar*' }
    if (-not $processes) {
        throw "could not find the Paper Java process to confirm the service-launched server exists"
    }
    Write-Host "client exit check: service and Paper server remain running with no active CLI client"
    Write-Host "job object precondition: Paper server launched through the Windows agent process path"

    [pscustomobject]@{
        RunDir = $runDir
        BaseUrl = $baseUrl
        Token = $token
        ServiceName = $serviceName
        ServerName = $serverName
        MscPath = $msc
    } | ConvertTo-Json | Set-Content -Path $checkpointPath -Encoding UTF8

    Write-Host "checkpoint recorded at $checkpointPath"
    Write-Host "sign out of Windows, sign back in, then rerun this exact command:"
    Write-Host "powershell -ExecutionPolicy Bypass -File tools/phase4/windows-service-lifecycle.ps1 -ServerDir `$env:MSC2_PHASE4_PAPER_SERVER"
} catch {
    try {
        & $msc --base-url $baseUrl --token $token server stop $serverName | Out-Null
    } catch {
    }
    try {
        Stop-Service -Name $serviceName -ErrorAction SilentlyContinue
    } catch {
    }
    try {
        sc.exe delete $serviceName | Out-Null
    } catch {
    }
    if (-not $KeepArtifacts) {
        Remove-Item -LiteralPath $runDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    throw
}
