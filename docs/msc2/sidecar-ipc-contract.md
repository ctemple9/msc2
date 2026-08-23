# macOS Bedrock sidecar — IPC contract

Per `docs/msc2/rolling-plan.md` P0.28 and `msc2-engineering.md` §9. Bedrock
Dedicated Server has no macOS build; MSC 1 solves this by running BDS inside
a lightweight Linux VM under `Virtualization.framework`
(`VMBedrockServerBackend.swift`, 451 lines, verified working with a real
device joining on 2026-06-30). `Virtualization.framework` is a Swift/ObjC-only
API — the Rust agent cannot call it directly, so on macOS the `BedrockRuntime`
trait's implementation supervises a small Swift sidecar binary that owns the
VM, and talks to it over a narrow process protocol.

This contract is read directly off what `VMBedrockServerBackend.swift`
already does today (source line references throughout point at it) — it is
not a fresh design. Where MSC 1 has no equivalent because everything runs
in-process today, that is called out explicitly rather than invented.

## Transport

**JSON lines over stdio**, not a unix socket.

The agent spawns the sidecar as a child process — this is a 1:1
parent-supervises-child relationship for the lifetime of one Bedrock server,
the same shape the agent already uses to supervise `bedrock_server` directly
on Linux/Windows and Java server processes everywhere. Stdio gives that for
free: no socket path to choose, create, `chmod`, or clean up after an
unclean exit (a stale socket file left over from a crashed sidecar is a
class of bug a unix socket introduces that stdio cannot have); the pipe's
lifecycle is the process's lifecycle, so EOF on the sidecar's stdout *is*
the crash-notification signal with no extra polling; and closing the
sidecar's stdin is a clean way to ask it to exit. A unix socket would need
the same JSON-envelope framing this contract defines anyway, and buys
nothing here since there is never more than one client.

Framing: one JSON object per line (newline-delimited, UTF-8), in both
directions. Every agent→sidecar message has a `"type"` field naming the
request; every sidecar→agent message has a `"type"` field naming the
response or event. This mirrors `VMBedrockServerBackend.swift`'s own
console-reading loop, which already frames the guest's serial output on
`\n` boundaries (`handleIncoming(data:)`, lines 235–247) — the sidecar
reuses that same framing discipline one level up, multiplexing control
messages and console-stream events onto one line-oriented channel instead
of needing a second transport for logs.

### provision

Ensures the VM's bundled appliance resources are present before boot: the
virtio-built-in kernel and initramfs looked up via `Bundle.main.url` in
`kernelURL()` / `initramfsURL()` (lines 84–91), and the shared world
directory (`config.serverDir`, checked with
`FileManager.default.fileExists`, lines 118–122). Request carries the
server directory path and the target Bedrock version; response is
success or a specific failure reason ("VM kernel is missing from the app",
"VM initramfs is missing from the app", "Server folder not found").

Note: `start(config:appConfig:)` also calls `BedrockProvisioner.ensureInstalled(serverDir:version:)`
(line 139) to stage the BDS binary itself into `serverDir` before boot. That
step has no VM dependency — it downloads and places a binary on the host
filesystem the same way any other component install does — so it is left
open here whether it belongs to this sidecar protocol at all, or stays a
generic agent-side component-install step that runs before `provision` is
even called. Marking this open rather than guessing.

### start

Boots the VM once provisioning has succeeded: builds the VM configuration
(`buildVMConfiguration`, lines 169–219 — 1–2 vCPUs, RAM from
`config.maxRamGB` clamped to `[minimumAllowedMemorySize,
maximumAllowedMemorySize]` defaulting to 2 GB, `console=hvc0` boot line, NAT
networking, virtio-fs share of `serverDir` tagged `"world"`), then
`VZVirtualMachine.start()` (line 151). Request carries the resolved config
(RAM cap, Bedrock port from `BedrockPropertiesManager.readModel`, line 124);
response is an immediate accepted/rejected ack (boot proceeds
asynchronously — readiness is a separate signal below) or a start failure
("This Mac does not support virtualization...", line 104–107, or the VM
framework's own start error, lines 151–157).

### readiness signal

MSC 1 has no explicit "ready" event — the closest equivalent is
guest-IP discovery: `processLine` watches every console line for
`"[appliance] dhcp:"`, extracts the IP with `parseGuestIP` (lines 258–264,
303–307), and only then starts the UDP relay (`startRelay`, line 263). The
sidecar protocol makes this explicit: a `ready` event fires once the guest
has DHCP-leased an address and the relay is up, carrying the guest IP and
the relayed port — the point at which the agent can report the server as
actually reachable, not merely "VM booting."

### stop

Graceful shutdown: send BDS the `"stop"` console command (line 358) and
arm a 20-second fallback timer (`forceStopWorkItem`, lines 360–366) that
force-stops if the guest hasn't powered itself off in time. MSC 1's own UI
already treats a second Stop press during the pending window as an explicit
force request (lines 348–356) — the sidecar protocol keeps `stop` and
`force-stop` as two distinct request types rather than reusing one with a
flag, so the agent (which owns the timeout/retry policy generically across
every backend) decides when to escalate instead of the sidecar guessing.

### force-stop

Immediate hard power-off: `VZVirtualMachine.stop()` (`forceStopVM`, lines
375–393) — a hard stop that, per the source comment at line 387, does
*not* trigger the normal `guestDidStop` delegate callback, so the sidecar
must independently tear down state and fire termination itself rather than
relying on the graceful-stop notification path. Also the terminal state for
`terminate()` (line 370), i.e. what the agent calls on its own forced
shutdown of the sidecar (crash recovery, server delete, agent shutdown).

### crash notification

MSC 1's `onDidTerminate` closure (line 40) fires exactly once
(`fireDidTerminate`'s `hasFiredTerminate` guard, lines 407–413) from three
places: normal guest poweroff (`guestDidStop`, line 438), VM error
(`didStopWithError`, line 444), and any internal failure path that calls
`teardown()` + `fireDidTerminate()` (VM-support/kernel/initramfs/serverDir
failures at start, lines 104–163; boot failure, lines 152–157). The sidecar
protocol collapses this to one `terminated` event carrying a reason enum
(`clean` / `guest-error:<message>` / `start-failed:<message>`) — the agent
needs to distinguish "server stopped as requested" from "server crashed"
the same way it already must for native Linux/Windows backends.

### console stream

Every line read from the guest's serial console, after `[MSCSTATS]`
performance lines are intercepted and consumed (see below) — the
`onOutputLine` closure (line 39), fed by `processLine`/`emitLine` (lines
249–267, 417–419). One `console-line` event per guest output line,
preserving MSC 1's byte-stream-to-lines framing
(`handleIncoming`/`flushPendingOutput`, lines 235–247, 421–427) including
the flush-on-EOF behavior for a final partial line with no trailing
newline.

### command input

Host→guest text written to BDS's stdin over the serial console
(`sendCommand`, lines 324–338) — trailing newline added if missing,
UTF-8-encode failure and "not running" both surface as
`lastCommandError` (line 41) in MSC 1's synchronous API. The sidecar
protocol makes this a request/response: `command` request carries the raw
command text, response is success or a specific rejection reason
(not-running / encoding failure) — same two failure modes MSC 1 already
distinguishes, not new ones invented for the port.

### shared-directory mapping

The `serverDir` (BDS install + world) is shared into the guest over
virtio-fs under the fixed tag `"world"`, mounted at `/mnt` in-guest
(`VZVirtioFileSystemDeviceConfiguration`, `VZSingleDirectoryShare`, lines
207–212 — the read-write equivalent of Docker's `-v serverDir:/data`,
per the file's own header comment, lines 11–13). This mapping is fixed at
VM-configuration time, before `start`, not a separate runtime message —
documented here because `provision`/`start` both depend on it and a future
sidecar revision may need to support remapping without a full VM rebuild
(e.g. world-slot switching), which MSC 1 does not currently need since it
always tears down and rebuilds the VM configuration fresh (`buildVMConfiguration`
is called anew from `start`, line 142).

### host-directory persistence across VM replacement

MSC 1 never actually replaces a running VM's configuration in place — every
`start()` call builds a fresh `VZVirtualMachineConfiguration` (line 142) and
a fresh `VZVirtualMachine` (line 147); nothing here is torn down and rebuilt
mid-session. World persistence works only because `serverDir` lives on the
host and is *shared*, not copied, into the guest (line 210–211) — so it
survives every VM instance sharing the same tag regardless of appliance
version. The sidecar protocol's obligation is therefore narrow and
already met by construction: never write anything BDS-relevant outside the
shared `serverDir`, and never require the previous VM instance to still
exist for the host-side world files to remain valid. There is no MSC 1
precedent for hot-swapping an appliance under a running guest — if a future
sidecar needs that, it is new design, not a port, and should be called out
as such rather than retrofitted into this contract.

## Frozen frame vocabulary

The Rust agent and Swift sidecar exchange one object per UTF-8 line. These
are the only process-boundary messages:

| Direction | `type` | Required fields | Meaning |
|---|---|---|---|
| agent → sidecar | `provision` | `server_dir`, `version` | Check the bundled VM resources and the shared host directory. |
| sidecar → agent | `provisioned` | `ok`, optional `reason` | Provisioning accepted or rejected. |
| agent → sidecar | `start` | `memory_gb`, `bedrock_port` | Accept a VM boot request; readiness is separate. |
| sidecar → agent | `started` | `accepted`, optional `reason` | Boot request accepted or rejected. |
| sidecar → agent | `ready` | `guest_ip`, `port`, `relay_up` | The guest has an address and the relay is usable. |
| agent → sidecar | `stop` | — | Send BDS `stop` and arm the shared graceful timeout. |
| agent → sidecar | `force-stop` | — | Stop immediately. |
| agent → sidecar | `command` | `command` | Send raw command text to BDS. |
| sidecar → agent | `command-result` | `ok`, optional `reason` | Command accepted or rejected. |
| sidecar → agent | `console-line` | `line` | One already-framed guest console line. |
| sidecar → agent | `terminated` | `reason` | `clean`, `guest-error:<message>`, or `start-failed:<message>`. |

The Rust shared runtime exposes readiness, console, command, termination,
metrics, and capability values without exposing `guest_ip`, relay state, VM
types, or a native process API. A sidecar maps its `ready` frame to a generic
reachable endpoint; native backends can report readiness without inventing a
guest address. Metrics cross the boundary as numeric runtime values after the
sidecar consumes its private `[MSCSTATS]` line, while native backends use the
existing OS process-statistics substrate.

`provision` establishes the fixed read-write mapping of `server_dir` to
`/mnt` under the `world` virtio-fs tag. This is an invariant, not a separate
wire message. Because the directory remains host-owned, a fresh VM can see
the same world files after replacement; the sidecar never persists BDS state
outside that directory.
