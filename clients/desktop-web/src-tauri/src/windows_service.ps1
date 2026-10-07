$ErrorActionPreference = 'Stop'
$password = $null

function Invoke-Agent([string[]] $commandArgs) {
    $output = & $agent @commandArgs 2>&1
    if ($LASTEXITCODE -ne 0) { throw ($output | Out-String).Trim() }
}

function Invoke-ServiceController([string[]] $commandArgs) {
    $output = & "$env:SystemRoot\System32\sc.exe" @commandArgs 2>&1
    if ($LASTEXITCODE -ne 0) { throw ($output | Out-String).Trim() }
    return ($output | Out-String).Trim()
}

try {
    if ($operation -eq 'install') {
        if (Get-CimInstance -ClassName Win32_Service -Filter "Name='com.ctemple.msc2.agent'") {
            throw 'The local service already exists. Use Setup to replace its owned package; first-launch registration cannot overwrite it.'
        }
        Add-Type -TypeDefinition $nativeSource
        $password = [MscServiceNative]::PromptPassword($account, $owner)
        [MscServiceNative]::GrantServiceLogon($account)
        # The credential never crosses back into the unelevated desktop process.
        $env:MSC2_WINDOWS_SERVICE_PASSWORD = $password
        try { Invoke-Agent $installArgs }
        finally { Remove-Item Env:MSC2_WINDOWS_SERVICE_PASSWORD -ErrorAction SilentlyContinue }

        # The owner can query, start and stop this service without further elevation.
        $descriptor = Invoke-ServiceController @('sdshow', $serviceName)
        $descriptor = [MscServiceNative]::AllowOwnerControl($descriptor, $account)
        $null = Invoke-ServiceController @('sdset', $serviceName, $descriptor)
        Invoke-Agent @('service', 'start', '--service-name', $serviceName)
        $ready = $false
        for ($attempt = 0; $attempt -lt 60; $attempt++) {
            $state = Invoke-ServiceController @('queryex', $serviceName)
            if ($state -match 'STATE\s*:\s*4\s') { $ready = $true; break }
            if ($state -match 'STATE\s*:\s*1\s') {
                throw 'The MSC agent stopped during startup. Check the agent log and Windows service logon policy.'
            }
            Start-Sleep -Milliseconds 500
        }
        if (-not $ready) { throw 'The MSC agent did not finish starting within 30 seconds.' }
    } elseif ($operation -eq 'uninstall') {
        Invoke-Agent @('service', 'uninstall', '--service-name', $serviceName)
    } else {
        throw 'Unsupported Windows service operation.'
    }
    $result = @{ success = $true; message = $null }
} catch {
    $message = $_.Exception.GetBaseException().Message
    if (-not [string]::IsNullOrEmpty($password)) { $message = $message.Replace($password, '[redacted]') }
    $result = @{ success = $false; message = $message }
} finally {
    Remove-Item Env:MSC2_WINDOWS_SERVICE_PASSWORD -ErrorAction SilentlyContinue
    $password = $null
}
[IO.File]::WriteAllText($resultPath, ($result | ConvertTo-Json -Compress), (New-Object Text.UTF8Encoding($false)))
if (-not $result.success) { exit 1 }
