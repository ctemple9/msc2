# Phase 20 owner measurement record

Recorded October 16, 2026. **All physical measurements pending.** P20.1 inspected
source and locked APIs only; no test, benchmark, build or import was run. No
speedup is claimed. Run this record against the current installed app first,
then repeat with the matching P20.7 native candidate. Keep source files unchanged.

## Identify the run

Record installed desktop and agent versions, source commit, OS, CPU/RAM, disk
models/filesystems, source and staging volumes, free space, power mode, running
Minecraft servers and background disk activity. For remote runs record both
machines, LAN/Tailscale route, wired/Wi-Fi and measured link speed. Record whether
source/archive/provider caches are warm, cold or unknown; do not flush OS caches
or label a first run cold without evidence. Use a disposable destination server
for import/replace comparisons, never the production server.

On macOS these read-only commands capture useful machine/storage facts:

```sh
sw_vers
system_profiler SPHardwareDataType SPStorageDataType
 df -h
 git rev-parse HEAD
```

Omit serial numbers and private paths when sharing output. On Linux use
`uname -a`, `lscpu`, `lsblk -o NAME,SIZE,FSTYPE,MOUNTPOINTS` and `df -h`;
on Windows use System Information, Disk Management and the app version display.

## Inputs and copy floor

Choose the approximately 8 GB world ZIP, a representative modpack ZIP/mrpack,
a stopped raw server folder plus its ZIP, and an MSC transfer archive if
available. Record exact bytes and SHA-256 outside the timed import. This hash
read warms caches: label the following run accordingly. The commands below are
macOS commands; replace the example paths with actual selected copies.

```sh
stat -f '%z bytes' '/absolute/path/world.zip'
shasum -a 256 '/absolute/path/world.zip'
stat -f '%z bytes' '/absolute/path/pack.mrpack'
shasum -a 256 '/absolute/path/pack.mrpack'
du -sk '/absolute/path/stopped-server'
```

Create a **new, empty measurement directory** on the intended staging volume.
Choose a fresh destination filename for every run; cp can overwrite a file.
The following ordinary copy omits macOS cp’s `-c` clone option; `-X` omits
extended attributes. Record these options with the byte-copy comparison:

```sh
/usr/bin/time -l /bin/cp -X '/absolute/path/world.zip' '/measurement-volume/new-run/world-copy.zip'
/usr/bin/time -l /bin/cp -X '/absolute/path/pack.mrpack' '/measurement-volume/new-run/pack-copy.mrpack'
/usr/bin/time -l /bin/cp -RX '/absolute/path/stopped-server' '/measurement-volume/new-run/server-copy'
```

Record real/user/system time and maximum resident set size. macOS cp's process
RSS is not app/agent peak memory, and cp completion does not guarantee durable
flush. Measure clone separately if used; do not compare clone time to streamed
bytes as equivalent work. Repeat same-volume and cross-volume where available.
For an available SSH destination, a network/disk reference is:

```sh
/usr/bin/time -l scp '/absolute/path/world.zip' 'user@host:/new-measurement-directory/world-copy.zip'
```

SSH encryption, protocol and cache behavior differ from MSC; label this a
reference, not an upload benchmark. No remote access or credentials are assumed.
On Linux use `/usr/bin/time -v cp --reflink=never SOURCE NEW_DESTINATION`;
on Windows time `Copy-Item` with `Measure-Command` and record process memory
separately. Keep all output artifacts outside the repository.

## Current app and candidate: repeat the same procedure

1. Open the destination host and record versions. Start Activity Monitor (or
   platform process monitor), watching desktop, webview helper and agent memory
   separately. Record idle memory first; sample peaks throughout the operation.
2. Choose the world ZIP through its existing import sheet. Record time immediately
   before selection, first visible progress, transfer completion, validation/
   inspection completion and final successful import. Record legacy chunk size.
   If current UI combines stages, record **combined/unknown**, never estimate a
   hidden stage from total time. Confirm the imported world can be selected and
   its source archive still has its original size/hash.
3. Repeat for modpack inspection and installation. Record provider/dependency
   downloads separately when visible; cache state and manual-file waiting time
   must accompany the result. Do not attribute provider wait to upload speed.
4. Import the stopped raw folder, its ZIP and available MSC transfer into
   disposable destinations using existing supported workflows. Current
   sourcePath must exist on the agent; a desktop path on another machine is not
   a valid remote baseline. Mark unsupported remote-selected-folder cases N/A.
5. Export the disposable world/server/client add-ons where available, recording
   archive preparation and download separately when exposed.
6. Repeat available local same-volume, local cross-volume and remote cases with
   identical inputs, power/network conditions and recorded cache state. Prefer
   three observed runs and report each, not only the fastest. Do not repeat a
   release workflow to obtain a benchmark build.

P20.7 native progress must expose transfer/copy bytes and elapsed time separately
from verifying, inspection, extraction and provider installation. Before that
instrumentation exists, unavailable stage measurements remain pending. No
current CLI command exposes all these stage timings; a fabricated benchmark
command would hide that limitation. The UI procedure above measures actual
current behavior. Copy commands establish an independent throughput reference.

MiB/s = exact transferred bytes / 1,048,576 / transfer seconds. Folder allocated
size from du is not an exact wire-byte denominator; use measured archive bytes
for network rate and mark folder-copy rate unknown unless exact bytes are known.
Record stage durations as nonoverlapping or explicitly overlapping; do not sum
parallel activities as elapsed time.

| Run/input | Versions + placement/cache | Prepare s | Transfer/copy bytes + s | Verify/inspect s | Extract/provider s | Total s | Desktop/webview/agent peak memory | Result |
|---|---|---|---|---|---|---|---|---|
| Current ~8 GB world | pending | pending | pending | pending | pending | pending | pending | pending |
| Current representative pack | pending | pending | pending | pending | pending | pending | pending | pending |
| Current raw folder/ZIP | pending | pending | pending | pending | pending | pending | pending | pending |
| Current MSC transfer/export | pending | pending | pending | pending | pending | pending | pending | pending |
| Native candidate, same inputs | pending P20.7 | pending | pending | pending | pending | pending | pending | pending |

Record observed source preservation, correct destination host, cancellation and
partial cleanup separately in P20.7 acceptance. Do not deliberately corrupt or
replace production data for a performance run. No physical acceptance result is
closed by this document.
