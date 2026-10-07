# Used only by the separately confirmed full-removal worker, never ordinary MSI.
$ErrorActionPreference = 'Stop'
$root = 'C:\ProgramData\MSC2\Services'
if (-not (Test-Path -LiteralPath $root)) { exit 0 }
if (Get-Service -Name com.ctemple.msc2.agent -ErrorAction SilentlyContinue) { throw 'The local service still exists; retain its cache.' }
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class MscCleanupPaths {
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern SafeFileHandle CreateFile(string path, uint access, uint share,
        IntPtr security, uint disposition, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern bool GetFileInformationByHandle(SafeFileHandle handle, out Info info);
    [StructLayout(LayoutKind.Sequential)] struct Info {
        public uint attributes;
        public System.Runtime.InteropServices.ComTypes.FILETIME created, accessed, written;
        public uint volume, high, low, links, indexHigh, indexLow;
    }
    public static SafeFileHandle Pin(string path) {
        var handle = CreateFile(path, 0, 3, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero);
        Info info;
        if (handle.IsInvalid || !GetFileInformationByHandle(handle, out info) ||
            (info.attributes & 0x400) != 0 || (info.attributes & 0x10) == 0) {
            handle.Dispose(); throw new InvalidOperationException("Refusing an unpinned cleanup directory.");
        }
        return handle;
    }
}
'@
function Check-Path([string]$path, [bool]$protected = $true) {
    $full = [IO.Path]::GetFullPath($path)
    if ($full -ine $root -and -not $full.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Cleanup escaped the fixed service directory.' }
    if ((Get-Item -LiteralPath $full -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing a redirected cleanup path.' }
    if (-not $protected) { return }
    $acl = Get-Acl -LiteralPath $full
    if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin @('S-1-5-18','S-1-5-32-544')) { throw 'The service cache is not administrator-owned.' }
    foreach ($rule in $acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {
        $write = [Security.AccessControl.FileSystemRights]::Write -bor [Security.AccessControl.FileSystemRights]::Delete -bor [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership
        if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -notin @('S-1-5-18','S-1-5-32-544') -and ($rule.FileSystemRights -band $write)) { throw 'The service cache is writable outside administrators.' }
    }
}
$parent = 'C:\ProgramData\MSC2'
if ((Get-Item -LiteralPath $parent -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing a redirected service parent.' }
Check-Path $root $false
$parentHandle = [MscCleanupPaths]::Pin($parent)
$rootHandle = [MscCleanupPaths]::Pin($root)
$cache = Join-Path $root 'DesktopLifecycle'
$children = @(Get-ChildItem -LiteralPath $root -Force)
if ($children | Where-Object { $_.Name -ne 'DesktopLifecycle' -or -not $_.PSIsContainer }) { throw 'Unrecognized service metadata remains.' }
if (Test-Path -LiteralPath $cache) {
    $cacheHandle = [MscCleanupPaths]::Pin($cache)
    Check-Path $cache
    $entries = @(Get-ChildItem -LiteralPath $cache -Force)
    if ($entries | Where-Object { $_.Name -notin @('owner.json','builds') }) { throw 'An unfinished or unknown lifecycle record remains; retain it for recovery.' }
    $owner = Join-Path $cache 'owner.json'
    if (Test-Path -LiteralPath $owner) {
        Check-Path $owner
        $record = Get-Content -LiteralPath $owner -Raw | ConvertFrom-Json
        if ($record.version -ne 1 -or -not $record.accountSid -or -not $record.dataRoot -or -not $record.packageRoot) { throw 'The lifecycle ownership record is not recognized.' }
    }
    $builds = Join-Path $cache 'builds'
    $directories = @()
    if (Test-Path -LiteralPath $builds) {
        Check-Path $builds
        $directories = @(Get-ChildItem -LiteralPath $builds -Force)
        foreach ($build in $directories) {
            if (-not $build.PSIsContainer -or $build.Name -notmatch '^[0-9a-f]{64}$') { throw 'Unrecognized cached build remains.' }
            Check-Path $build.FullName
            $files = @(Get-ChildItem -LiteralPath $build.FullName -Force)
            if ($files.Count -ne 4 -or ($files | Where-Object { $_.PSIsContainer -or $_.Name -notin @('msc.exe','vantage.exe','bedrock-map.exe','VANTAGE-LICENSE.txt') })) { throw 'The cached payload contains unrecognized files.' }
            foreach ($file in $files) { Check-Path $file.FullName }
            $hash = [Security.Cryptography.SHA256]::Create()
            try {
                foreach ($name in @('msc.exe','vantage.exe','bedrock-map.exe')) {
                    $bytes = [IO.File]::ReadAllBytes((Join-Path $build.FullName $name))
                    $null = $hash.TransformBlock($bytes,0,$bytes.Length,$bytes,0)
                }
                $null = $hash.TransformFinalBlock(@(),0,0)
                if (([BitConverter]::ToString($hash.Hash)).Replace('-','').ToLowerInvariant() -ne $build.Name) { throw 'The cached payload digest changed.' }
            } finally { $hash.Dispose() }
        }
    }
    # Validate every child before deleting anything; delete explicit files and
    # empty directories, never recurse through a record-supplied data path.
    foreach ($build in $directories) {
        foreach ($file in (Get-ChildItem -LiteralPath $build.FullName -Force)) { Check-Path $file.FullName; Remove-Item -LiteralPath $file.FullName -Force }
        Remove-Item -LiteralPath $build.FullName -Force
    }
    if (Test-Path -LiteralPath $builds) { Remove-Item -LiteralPath $builds -Force }
    if (Test-Path -LiteralPath $owner) { Remove-Item -LiteralPath $owner -Force }
    $cacheHandle.Dispose()
    Remove-Item -LiteralPath $cache -Force
}
$rootHandle.Dispose()
Remove-Item -LiteralPath $root -Force
$parentHandle.Dispose()
if (-not (Get-ChildItem -LiteralPath $parent -Force)) { Remove-Item -LiteralPath $parent -Force }
