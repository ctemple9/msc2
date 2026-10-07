# Embedded into native helpers. Never load a script from a caller-supplied path.
function Invoke-MscDesktopLifecycle($request, $expectedHashes) {
    $ErrorActionPreference = 'Stop'
    # Pin directory handles without delete sharing for the entire operation.
    # Attribute checks alone leave a junction-replacement race during elevation.
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class MscLifecyclePaths {
    static readonly Dictionary<string, SafeFileHandle> pinned =
        new Dictionary<string, SafeFileHandle>(StringComparer.OrdinalIgnoreCase);
    [StructLayout(LayoutKind.Sequential)] struct Info {
        public uint attributes;
        public System.Runtime.InteropServices.ComTypes.FILETIME created, accessed, written;
        public uint volume, sizeHigh, sizeLow, links, indexHigh, indexLow;
    }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern SafeFileHandle CreateFile(string path, uint access, uint sharing,
        IntPtr security, uint disposition, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern bool GetFileInformationByHandle(SafeFileHandle handle, out Info info);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr OpenSCManager(string machine, string database, uint access);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr OpenService(IntPtr manager, string name, uint access);
    [DllImport("advapi32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern bool ChangeServiceConfig(IntPtr service, uint type, uint start, uint error,
        string binary, string group, IntPtr tag, string dependencies, string account,
        string password, string display);
    [DllImport("advapi32.dll")] static extern bool CloseServiceHandle(IntPtr handle);
    [DllImport("advapi32.dll", SetLastError=true)] static extern bool DeleteService(IntPtr handle);
    public static void Detach() {
        var manager = OpenSCManager(null, null, 1);
        if (manager == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        try {
            var service = OpenService(manager, "com.ctemple.msc2.agent", 0x10000);
            if (service == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            try {
                if (!DeleteService(service)) throw new Win32Exception(Marshal.GetLastWin32Error());
            } finally { CloseServiceHandle(service); }
        } finally { CloseServiceHandle(manager); }
    }
    public static void SetBinary(string binary) {
        var manager = OpenSCManager(null, null, 1);
        if (manager == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        try {
            var service = OpenService(manager, "com.ctemple.msc2.agent", 2);
            if (service == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            try {
                if (!ChangeServiceConfig(service, UInt32.MaxValue, UInt32.MaxValue,
                    UInt32.MaxValue, binary, null, IntPtr.Zero, null, null, null, null))
                    throw new Win32Exception(Marshal.GetLastWin32Error());
            } finally { CloseServiceHandle(service); }
        } finally { CloseServiceHandle(manager); }
    }
    public static void AtomicText(string path, string text) {
        var temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
        var acl = new System.Security.AccessControl.FileSecurity();
        acl.SetAccessRuleProtection(true, false);
        foreach (var sid in new[] { "S-1-5-18", "S-1-5-32-544" }) {
            acl.AddAccessRule(new System.Security.AccessControl.FileSystemAccessRule(
                new System.Security.Principal.SecurityIdentifier(sid),
                System.Security.AccessControl.FileSystemRights.FullControl,
                System.Security.AccessControl.AccessControlType.Allow));
        }
        acl.AddAccessRule(new System.Security.AccessControl.FileSystemAccessRule(
            new System.Security.Principal.SecurityIdentifier("S-1-5-32-545"),
            System.Security.AccessControl.FileSystemRights.Read,
            System.Security.AccessControl.AccessControlType.Allow));
        acl.SetOwner(new System.Security.Principal.SecurityIdentifier("S-1-5-32-544"));
        try {
            using (var file = new System.IO.FileStream(temporary, System.IO.FileMode.CreateNew,
                System.Security.AccessControl.FileSystemRights.Write, System.IO.FileShare.None,
                4096, System.IO.FileOptions.None, acl)) {
                var bytes = new System.Text.UTF8Encoding(false).GetBytes(text);
                file.Write(bytes, 0, bytes.Length);
                file.Flush(true);
            }
            if (System.IO.File.Exists(path)) System.IO.File.Replace(temporary, path, null);
            else System.IO.File.Move(temporary, path);
        } finally { if (System.IO.File.Exists(temporary)) System.IO.File.Delete(temporary); }
    }
    public static void Pin(string path) {
        if (pinned.ContainsKey(path)) return;
        var handle = CreateFile(path, 0, 3, IntPtr.Zero, 3, 0x02200000, IntPtr.Zero);
        if (handle.IsInvalid) { handle.Dispose(); throw new Win32Exception(Marshal.GetLastWin32Error()); }
        Info info;
        if (!GetFileInformationByHandle(handle, out info)) {
            handle.Dispose(); throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        if ((info.attributes & 0x400) != 0) {
            handle.Dispose(); throw new InvalidOperationException("Refusing a redirected local service path.");
        }
        if ((info.attributes & 0x10) != 0) pinned.Add(path, handle);
        else handle.Dispose();
    }
}
'@
    $serviceName = 'com.ctemple.msc2.agent'
    $serviceRoot = 'C:\ProgramData\MSC2\Services'
    $stateRoot = Join-Path $serviceRoot 'DesktopLifecycle'
    $metadataPath = Join-Path $serviceRoot ($serviceName + '.metadata')
    $ownerPath = Join-Path $stateRoot 'owner.json'
    $sc = Join-Path ([Environment]::GetFolderPath('Windows')) 'System32\sc.exe'

    function Guard-Path([string]$path) {
        $full = [IO.Path]::GetFullPath($path)
        if ($full -notmatch '^[A-Za-z]:\\' -or $full -match '[\r\n|]' -or $full.Substring(2).Contains(':')) { throw 'Local service paths must be absolute local paths.' }
        $cursor = $full
        $ancestors = @()
        while ($cursor) {
            $ancestors += $cursor
            $cursor = Split-Path -Parent $cursor
        }
        [Array]::Reverse($ancestors)
        foreach ($ancestor in $ancestors) {
            if (Test-Path -LiteralPath $ancestor) { [MscLifecyclePaths]::Pin($ancestor) }
        }
        return $full.TrimEnd('\')
    }
    function Same-Path([string]$a, [string]$b) {
        return (Guard-Path $a) -ieq (Guard-Path $b)
    }
    function Write-Json([string]$path, $value) {
        $null = Guard-Path $path
        $temp = $path + '.tmp'
        $null = Guard-Path $temp
        [IO.File]::WriteAllText($temp, ($value | ConvertTo-Json -Depth 8 -Compress), (New-Object Text.UTF8Encoding($false)))
        if (Test-Path -LiteralPath $path) { [IO.File]::Replace($temp, $path, $null) }
        else { [IO.File]::Move($temp, $path) }
    }
    function Protect-Directory([string]$path) {
        $null = Guard-Path $path
        if (Test-Path -LiteralPath $path) { Assert-Protected $path; return }
        $acl = New-Object Security.AccessControl.DirectorySecurity
        $acl.SetAccessRuleProtection($true, $false)
        foreach ($sid in @('S-1-5-18', 'S-1-5-32-544')) {
            $identity = New-Object Security.Principal.SecurityIdentifier($sid)
            $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($identity, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
        }
        $users = New-Object Security.Principal.SecurityIdentifier('S-1-5-32-545')
        $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($users, 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
        $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
        # Create with the restricted ACL atomically; never claim a pre-existing
        # directory merely by overwriting its permissions.
        $null = [IO.Directory]::CreateDirectory($path, $acl)
        $null = Guard-Path $path
        Assert-Protected $path
    }
    function Assert-Protected([string]$path) {
        $null = Guard-Path $path
        $acl = Get-Acl -LiteralPath $path
        $owner = $acl.GetOwner([Security.Principal.SecurityIdentifier]).Value
        if ($owner -notin @('S-1-5-18', 'S-1-5-32-544')) { throw 'The lifecycle record is not owned by Windows administrators.' }
        foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -notin @('S-1-5-18', 'S-1-5-32-544')) {
                $write = [Security.AccessControl.FileSystemRights]::Write -bor [Security.AccessControl.FileSystemRights]::Delete -bor [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership
                if ($rule.FileSystemRights -band $write) { throw 'The lifecycle record is writable outside Windows administrators.' }
            }
        }
    }
    function Service {
        return Get-CimInstance -ClassName Win32_Service -Filter "Name='com.ctemple.msc2.agent'" -ErrorAction Stop
    }
    function Invoke-Sc([string[]]$arguments) {
        $null = & $sc @arguments 2>&1
        if ($LASTEXITCODE -ne 0) { throw "The local Windows service operation failed (code $LASTEXITCODE)." }
    }
    function Metadata {
        $null = Guard-Path $metadataPath
        if (-not (Test-Path -LiteralPath $metadataPath -PathType Leaf)) { throw 'The existing service has no readable MSC metadata; no files or service settings were changed.' }
        Assert-Protected $metadataPath
        $text = [IO.File]::ReadAllText($metadataPath)
        $map = @{}
        foreach ($line in ($text -split '\r?\n')) {
            if (-not $line) { continue }
            $pair = $line.Split('=', 2)
            if ($pair.Count -ne 2 -or $map.ContainsKey($pair[0])) { throw 'The existing service metadata is ambiguous.' }
            $map[$pair[0]] = $pair[1]
        }
        foreach ($key in @('service_name', 'binary_path', 'working_directory', 'log_path', 'run_user', 'arguments_hex', 'environment_hex', 'expected_port')) {
            if (-not $map.ContainsKey($key)) { throw 'The existing service metadata is incomplete.' }
        }
        if ($map.service_name -ne $serviceName) { throw 'The existing service metadata identifies another service.' }
        $map.Raw = $text
        return $map
    }
    function Decode-Hex([string]$hex) {
        if ($hex -notmatch '^(?:[0-9a-fA-F]{2})*$') { throw 'The service environment encoding is invalid.' }
        $bytes = New-Object byte[] ($hex.Length / 2)
        for ($i = 0; $i -lt $bytes.Length; $i++) { $bytes[$i] = [Convert]::ToByte($hex.Substring($i * 2, 2), 16) }
        return (New-Object Text.UTF8Encoding($false, $true)).GetString($bytes)
    }
    function Account-Sid([string]$account) {
        return (New-Object Security.Principal.NTAccount($account)).Translate([Security.Principal.SecurityIdentifier]).Value
    }
    function Assert-Definition($svc, $meta) {
        if ((Account-Sid $svc.StartName) -ne (Account-Sid $meta.run_user)) { throw 'The service account differs from its recorded owner.' }
        $binary = Guard-Path $meta.binary_path
        $prefix = '"' + $binary + '"'
        if (-not $svc.PathName.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
            $prefix = $binary
            if ($binary.Contains(' ') -or -not $svc.PathName.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'The service executable differs from its recorded installation.' }
        }
        $tail = $svc.PathName.Substring($prefix.Length)
        if ($tail -notmatch '^\s+service-run\s+--service-name\s+com\.ctemple\.msc2\.agent\s+--bind\s+127\.0\.0\.1:[0-9]+$' -or $tail.Trim() -ne "service-run --service-name $serviceName --bind 127.0.0.1:$($meta.expected_port)") {
            throw 'The service command is not the fixed local MSC service entry point.'
        }
        return $tail
    }
    function Bundle-Digest([string]$directory) {
        $null = Guard-Path $directory
        $hash = [Security.Cryptography.SHA256]::Create()
        try {
            foreach ($name in @('msc.exe', 'vantage.exe', 'bedrock-map.exe')) {
                $file = Guard-Path (Join-Path $directory $name)
                $bytes = [IO.File]::ReadAllBytes($file)
                $null = $hash.TransformBlock($bytes, 0, $bytes.Length, $bytes, 0)
            }
            $null = $hash.TransformFinalBlock(@(), 0, 0)
            return ([BitConverter]::ToString($hash.Hash)).Replace('-', '').ToLowerInvariant()
        } finally { $hash.Dispose() }
    }
    function Owned($svc, $meta, [string]$previousRoot) {
        $null = Assert-Definition $svc $meta
        $binary = Guard-Path $meta.binary_path
        # A marked standalone archive is independent and is never coordinated here.
        $archiveMarker = Join-Path (Split-Path -Parent $binary) '.msc2-owned'
        if ((Test-Path -LiteralPath $archiveMarker -PathType Leaf) -and ([IO.File]::ReadAllText($archiveMarker).Trim() -eq 'msc2-headless-archive')) { return $false }
        if (Test-Path -LiteralPath $ownerPath -PathType Leaf) {
            Assert-Protected $ownerPath
            $owner = Get-Content -LiteralPath $ownerPath -Raw | ConvertFrom-Json
            if ($owner.version -ne 1 -or -not (Same-Path $owner.packageRoot $previousRoot) -or -not (Same-Path $owner.binary $binary) -or -not (Same-Path $owner.dataRoot $meta.working_directory) -or $owner.accountSid -ne (Account-Sid $svc.StartName)) {
                throw 'The service ownership record differs from this desktop package.'
            }
            return $true
        }
        # Legacy desktop installs have no ownership record. Require the original
        # account's standard desktop data root and an exact old-package digest.
        $sid = Account-Sid $svc.StartName
        $profile = (Get-ItemProperty -LiteralPath "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\$sid" -Name ProfileImagePath).ProfileImagePath
        $dataRoot = Guard-Path (Join-Path ([Environment]::ExpandEnvironmentVariables($profile)) 'AppData\Roaming\MSC2')
        if (-not (Same-Path $meta.working_directory $dataRoot)) {
            # The standard machine/user archive locations are independently owned.
            $parent = Split-Path -Parent $binary
            if ($parent -ieq 'C:\Program Files\MSC2\bin' -or $parent -ieq (Join-Path ([Environment]::ExpandEnvironmentVariables($profile)) 'AppData\Local\MSC2\bin')) { return $false }
            throw 'The service data directory does not establish desktop ownership.'
        }
        $build = Split-Path -Leaf (Split-Path -Parent $binary)
        if ($build -notmatch '^[0-9a-f]{64}$' -or -not (Same-Path $binary (Join-Path $dataRoot "agent\builds\$build\msc.exe"))) { throw 'The existing service is not a recognized copied desktop build.' }
        if ((Bundle-Digest (Split-Path -Parent $binary)) -ne $build) { throw 'The copied desktop build has changed.' }
        # Same-product repair can restore missing package files. Its embedded
        # manifest proves the copied payload without relying on damaged files.
        $repairMatch = $request.installed
        foreach ($name in @('msc.exe', 'vantage.exe', 'bedrock-map.exe')) {
            if ((Get-FileHash -LiteralPath (Join-Path (Split-Path -Parent $binary) $name)).Hash -ine $expectedHashes.$name) { $repairMatch = $false }
        }
        if (-not $repairMatch -and (-not $previousRoot -or (Bundle-Digest (Join-Path (Guard-Path $previousRoot) 'agent')) -ne $build)) {
            throw 'The copied agent cannot be matched to this desktop package. Keep the installation for inspection.'
        }
        return $true
    }
    function Wait-State([string]$state) {
        $deadline = [DateTime]::UtcNow.AddSeconds(90)
        do {
            $svc = Service
            if (-not $svc) { throw 'The service disappeared during package maintenance.' }
            if ($svc.State -eq $state) {
                if ($state -eq 'Stopped' -and $svc.ExitCode -ne 0) { throw 'The agent did not report a successful shutdown.' }
                return
            }
            Start-Sleep -Milliseconds 250
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "The local agent did not reach $state; package changes are refused."
    }
    function Stop-Owned($svc, $meta) {
        if ($svc.State -eq 'Stopped') { return }
        if ($svc.State -ne 'Running') { throw 'The local service is already changing state; retry after it settles.' }
        $environment = Decode-Hex $meta.environment_hex
        if (($environment -split '\n') -notcontains 'MSC2_SERVICE_MAINTENANCE_PROTOCOL=1') {
            throw 'This older running agent cannot acknowledge safe package shutdown. Stop Minecraft and then the local agent explicitly before running Setup again.'
        }
        # No STOP fallback and no process killing if the graceful handshake fails.
        Invoke-Sc @('control', $serviceName, '128')
        Wait-State 'Stopped'
    }
    function Restore-Running {
        # Windows can have a running service whose startup setting is Disabled.
        # Temporarily permit this explicit restart, then restore that boot policy.
        $disabled = (Service).StartMode -eq 'Disabled'
        try {
            if ($disabled) {
                $transaction.bootTemporarilyEnabled = $true
                Save-State
                Invoke-Sc @('config', $serviceName, 'start=', 'demand')
            }
            Invoke-Sc @('start', $serviceName)
            Wait-State 'Running'
        } finally {
            if ($disabled) {
                $svc = Service
                if (-not $svc -or (Account-Sid $svc.StartName) -ne $transaction.accountSid -or ($svc.PathName -ne $transaction.originalCommand -and $svc.PathName -ne $transaction.newCommand) -or $svc.StartMode -notin @('Manual', 'Disabled')) {
                    throw 'The service changed externally while restoring its boot policy.'
                }
                Invoke-Sc @('config', $serviceName, 'start=', 'disabled')
                $transaction.bootTemporarilyEnabled = $false
                Save-State
            }
        }
    }
    function Save-State { Write-Json $transactionPath $transaction }
    function Restore-Boot {
        $mode = switch ($transaction.startMode) {
            'Automatic' { if ($transaction.delayedAuto) { 'delayed-auto' } else { 'auto' } }
            'Manual' { 'demand' }
            'Disabled' { 'disabled' }
            default { throw 'The original service boot policy is not recognized.' }
        }
        Invoke-Sc @('config', $serviceName, 'start=', $mode)
    }
    function Assert-Unchanged($svc) {
        if (-not $svc -or $svc.PathName -ne $transaction.originalCommand -or (Account-Sid $svc.StartName) -ne $transaction.accountSid -or $svc.StartMode -ne $transaction.startMode) {
            throw 'The service definition changed during Setup; refusing to overwrite another installation.'
        }
    }

    $null = Guard-Path $serviceRoot
    $guid = [Guid]::Parse($request.transaction).ToString('D')
    $transactionPath = Join-Path $stateRoot ($guid + '.json')
    if ($request.operation -eq 'prepare') {
        $svc = Service
        if (-not $svc) {
            if (Test-Path -LiteralPath $metadataPath) { throw 'MSC service metadata remains without its service; inspect it before changing this package.' }
            return
        }
        $meta = Metadata
        if (-not (Owned $svc $meta $request.previousRoot)) { return }
        if ($svc.State -notin @('Running','Stopped')) { throw 'The local agent is already changing state.' }
        Protect-Directory $stateRoot
        if (Get-ChildItem -LiteralPath $stateRoot -File | Where-Object { $_.Name -match '^[0-9a-f-]{36}\.json(?:\.tmp)?$' }) {
            throw 'An unfinished package transaction requires recovery before retrying.'
        }
        $delayed = Get-ItemProperty -LiteralPath "Registry::HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\$serviceName" -Name DelayedAutoStart -ErrorAction SilentlyContinue
        $transaction = @{ version=1; originalCommand=$svc.PathName; accountSid=(Account-Sid $svc.StartName); startMode=$svc.StartMode; delayedAuto=($delayed.DelayedAutoStart -eq 1); wasRunning=($svc.State -eq 'Running'); metadata=$meta.Raw; previousRoot=$request.previousRoot; packageRoot=$request.packageRoot; oldOwner=$null; applying=$false; newCommand=$null; bootTemporarilyEnabled=$false; removing=$request.remove; removalDisabled=$false }
        if (Test-Path -LiteralPath $ownerPath) { $transaction.oldOwner = [IO.File]::ReadAllText($ownerPath) }
        Save-State
        Stop-Owned $svc $meta
        Assert-Unchanged (Service)
        if ($transaction.removing) {
            # Retain SCM credentials until commit; disabling prevents restart
            # while Windows Installer removes the application package.
            $transaction.removalDisabled = $true
            Save-State
            Invoke-Sc @('config', $serviceName, 'start=', 'disabled')
        }
        return
    }
    if (-not (Test-Path -LiteralPath $transactionPath -PathType Leaf)) { return }
    Assert-Protected $transactionPath
    $transaction = Get-Content -LiteralPath $transactionPath -Raw | ConvertFrom-Json
    if ($transaction.version -ne 1) { throw 'The local service recovery record is not recognized.' }

    switch ($request.operation) {
        'apply' {
            $svc = Service
            Assert-Unchanged $svc
            if ($svc.State -ne 'Stopped') { throw 'The agent must remain stopped while its payload changes.' }
            if (-not (Same-Path $request.packageRoot $transaction.packageRoot)) { throw 'The package destination changed during maintenance.' }
            $source = Join-Path (Guard-Path $transaction.packageRoot) 'agent'
            foreach ($entry in $expectedHashes.PSObject.Properties) {
                $file = Guard-Path (Join-Path $source $entry.Name)
                if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ine $entry.Value) { throw 'The installed payload differs from the payload bound to the native lifecycle helper.' }
            }
            $digest = Bundle-Digest $source
            $buildRoot = Join-Path $stateRoot 'builds'
            Protect-Directory $buildRoot
            $destination = Join-Path $buildRoot $digest
            if (Test-Path -LiteralPath $destination) { Assert-Protected $destination }
            else { Protect-Directory $destination }
            foreach ($entry in $expectedHashes.PSObject.Properties) {
                $target = Guard-Path (Join-Path $destination $entry.Name)
                if (-not (Test-Path -LiteralPath $target)) {
                    $incoming = Guard-Path ($target + '.incoming-' + $guid)
                    try {
                        Copy-Item -LiteralPath (Join-Path $source $entry.Name) -Destination $incoming -Force
                        if ((Get-FileHash -LiteralPath $incoming -Algorithm SHA256).Hash -ine $entry.Value) { throw 'The staged payload does not match its package.' }
                        [IO.File]::Move($incoming, $target)
                    } finally { if (Test-Path -LiteralPath $incoming) { Remove-Item -LiteralPath $incoming } }
                }
                if ((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ine $entry.Value) { throw 'The immutable service build does not match its package.' }
            }
            $meta = Metadata
            $tail = Assert-Definition $svc $meta
            $newBinary = Join-Path $destination 'msc.exe'
            $transaction.newCommand = '"' + $newBinary + '"' + $tail
            $transaction.applying = $true
            Save-State
            [MscLifecyclePaths]::SetBinary($transaction.newCommand)
            $environment = Decode-Hex $meta.environment_hex
            $environment = (@($environment -split '\n' | Where-Object { $_ -and $_ -notlike 'MSC2_SERVICE_MAINTENANCE_PROTOCOL=*' }) + @('MSC2_SERVICE_MAINTENANCE_PROTOCOL=1')) -join "`n"
            $environmentHex = ([BitConverter]::ToString([Text.Encoding]::UTF8.GetBytes($environment))).Replace('-', '').ToLowerInvariant()
            $updated = ($meta.Raw -split '\r?\n' | Where-Object { $_ } | ForEach-Object {
                if ($_ -like 'binary_path=*') { "binary_path=$newBinary" }
                elseif ($_ -like 'environment_hex=*') { "environment_hex=$environmentHex" }
                else { $_ }
            }) -join "`n"
            $null = Guard-Path $metadataPath
            [MscLifecyclePaths]::AtomicText($metadataPath, $updated + "`n")
            Write-Json $ownerPath @{ version=1; packageRoot=$transaction.packageRoot; binary=$newBinary; dataRoot=$meta.working_directory; accountSid=$transaction.accountSid }
        }
        'resume' {
            $svc = Service
            if (-not $svc -or $svc.PathName -ne $transaction.newCommand -or (Account-Sid $svc.StartName) -ne $transaction.accountSid -or $svc.StartMode -ne $transaction.startMode) { throw 'The replacement service definition changed before restart.' }
            if ($transaction.wasRunning) { Restore-Running }
        }
        'rollback' {
            $svc = Service
            if (-not $svc -and $transaction.removing) { throw 'Package rollback could not restore the detached Windows service. Worlds and settings remain. Reinstall MSC and set up local hosting again; Windows credentials must be entered again.' }
            $temporaryBoot = $transaction.bootTemporarilyEnabled -and $transaction.startMode -eq 'Disabled' -and $svc.StartMode -eq 'Manual'
            $removalBoot = $transaction.removalDisabled -and $svc.StartMode -eq 'Disabled'
            if (-not $svc -or (Account-Sid $svc.StartName) -ne $transaction.accountSid -or ($svc.StartMode -ne $transaction.startMode -and -not $temporaryBoot -and -not $removalBoot) -or ($svc.PathName -ne $transaction.originalCommand -and $svc.PathName -ne $transaction.newCommand)) { throw 'The service changed externally; automatic rollback cannot overwrite it.' }
            if ($removalBoot) { Restore-Boot }
            if ($temporaryBoot) { Invoke-Sc @('config', $serviceName, 'start=', 'disabled') }
            $transaction.bootTemporarilyEnabled = $false
            Save-State
            if ($transaction.applying -or $transaction.removing) {
                if ($svc.State -ne 'Stopped') { Stop-Owned $svc (Metadata) }
                [MscLifecyclePaths]::SetBinary($transaction.originalCommand)
                $null = Guard-Path $metadataPath
                [MscLifecyclePaths]::AtomicText($metadataPath, $transaction.metadata)
                if ($transaction.oldOwner) { [MscLifecyclePaths]::AtomicText($ownerPath, $transaction.oldOwner) }
                elseif (Test-Path -LiteralPath $ownerPath) { Remove-Item -LiteralPath $ownerPath }
            }
            if ($transaction.wasRunning -and (Service).State -eq 'Stopped') { Restore-Running }
            Remove-Item -LiteralPath $transactionPath
        }
        'commit' {
            if ($transaction.removing) {
                $svc = Service
                if (-not $svc -or $svc.State -ne 'Stopped' -or $svc.StartMode -ne 'Disabled' -or $svc.PathName -ne $transaction.originalCommand -or (Account-Sid $svc.StartName) -ne $transaction.accountSid) { throw 'Removal cannot detach a service whose state or owner changed.' }
                # All reversible cleanup precedes DeleteService. A failure here
                # retains Windows' stored password for the rollback action.
                $null = Guard-Path $metadataPath
                Remove-Item -LiteralPath $metadataPath
                if (Test-Path -LiteralPath $ownerPath) { Remove-Item -LiteralPath $ownerPath }
                [MscLifecyclePaths]::Detach()
                # No fallible action may turn successful detachment into a
                # rollback that would require recovering an unreadable password.
                try { Remove-Item -LiteralPath $transactionPath } catch {
                    [Console]::Error.WriteLine('MSC was detached, but its recovery record remains. Retain the record for inspection before another package operation.')
                }
                return
            }
            # Retain previous immutable builds; P16.49 needs them for health recovery.
            Remove-Item -LiteralPath $transactionPath
        }
        default { throw 'Unsupported native desktop lifecycle operation.' }
    }
}
