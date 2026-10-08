# Used by the confirmed full-removal worker after graceful Minecraft shutdown.
$ErrorActionPreference = 'Stop'
& sc.exe stop com.ctemple.msc2.agent
if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 1062) { exit $LASTEXITCODE }
$deadline = (Get-Date).AddSeconds(45)
do {
    $service = Get-Service -Name com.ctemple.msc2.agent -ErrorAction Stop
    try { $stopped = $service.Status -eq 'Stopped' }
    finally { $service.Dispose() }
    if ($stopped) { break }
    Start-Sleep -Milliseconds 250
} while ((Get-Date) -lt $deadline)
if (-not $stopped) { throw 'Agent did not stop; data was retained.' }
& sc.exe delete com.ctemple.msc2.agent
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Deletion may wait for a service-management window's open handles. Do not
# remove metadata/data until SCM confirms the registration is actually gone.
$deadline = (Get-Date).AddSeconds(15)
do {
    & sc.exe query com.ctemple.msc2.agent | Out-Null
    if ($LASTEXITCODE -eq 1060) { break }
    if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 1072) { exit $LASTEXITCODE }
    Start-Sleep -Milliseconds 250
} while ((Get-Date) -lt $deadline)
if ($LASTEXITCODE -ne 1060) {
    throw 'The agent service is pending deletion. Close service-management windows or restart Windows, then retry full removal. Data and metadata were retained.'
}
$metadata = 'C:\ProgramData\MSC2\Services\com.ctemple.msc2.agent.metadata'
if (Test-Path -LiteralPath $metadata) { Remove-Item -LiteralPath $metadata -Force }
