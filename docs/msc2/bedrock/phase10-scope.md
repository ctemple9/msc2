# Phase 10 scope: Bedrock runtimes

**Status:** P10.17 sidecar implementation recorded. The Rust client boundary
from P10.16 remains unchanged; this step adds the Swift VZ owner behind it.

**Sources read:** the Phase 10 oracle set named in `rolling-plan.md`, the
Phase 5 import and migration notes, the Phase 6 worlds/backups notes, the
Phase 9 networking notes, `docs/msc2/sidecar-ipc-contract.md`, the current Rust
workspace, and the MSC 1 source at
`~/Documents/Swift Projects/minecraft-server-controller` (read-only).

## The boundary in one paragraph

Bedrock is a server runtime, not a third Java server family. Linux and Windows
run the BDS executable directly as a native child process. macOS does not have
a BDS build: the Rust agent supervises the already-frozen JSON-lines Swift
sidecar, and that sidecar owns the Linux VM and its BDS process through
`Virtualization.framework`. The shared Rust contract must describe lifecycle,
readiness, console lines, commands, termination, metrics, and capabilities
without mentioning VZ, guest IPs, or a particular native process API.

This architecture follows D-007 and D-022, both approved on 2026-08-22. D-028
adds the explicit Intel-only macOS Bedrock precondition for this phase. Apple
Silicon remains unavailable under D-028 because no owner test hardware exists.

## Platform boundaries

| Host | Runtime boundary | UDP boundary | Evidence status at P10.1 |
|---|---|---|---|
| Linux | Native `bedrock_server` child process, using the existing Rust process supervision and crash-recovery substrate. | BDS binds the host UDP port directly. No relay. | MSC 2 net-new platform behavior; no MSC 1 native Linux runtime to port. |
| Windows | Native `bedrock_server.exe` child process, using the shared runtime contract plus Windows process-tree ownership and service-session behavior. | BDS binds the host UDP port directly. No relay. | MSC 2 net-new platform behavior; no MSC 1 native Windows runtime to port. |
| macOS | Rust client + supervisor for the Swift sidecar; sidecar owns VZ, the Linux appliance, the shared directory, guest console, and guest lifecycle. | The sidecar starts the host↔guest UDP relay after DHCP guest-IP discovery. | MSC 1 behavioral reference, constrained by the frozen sidecar contract; macOS VZ availability remains an evidence-gated capability. |

There is no native macOS BDS claim. A macOS host can advertise Bedrock only
when the sidecar, Virtualization.framework, bundled appliance resources, and
the relevant BDS version have reproducible evidence. Agent-host support and
BDS-runtime support are separate facts; they must not be inferred from one
another.

## Responsibilities

### Shared Rust responsibilities

- Own the `BedrockRuntime` vocabulary and the lifecycle operation state.
- Read and write BDS `server.properties`, `allowlist.json`, and
  `permissions.json`, preserving MSC 1's raw-property round trip and its
  name/XUID behavior.
- Read Bedrock player records through bounded LevelDB and little-endian NBT
  adapters. Corrupt, truncated, unsupported, or absent data is an explicit
  unavailable result, never a fabricated player or world.
- Own console readiness, version, player-event, command, rolling-log, and
  clean-stop/crash state at the application boundary.
- Coordinate Bedrock save-hold/save-query/save-resume backup snapshots and
  route live restore to the slot-based Worlds behavior described below.
- Stage BDS files for all backends, retain version/provenance information, and
  enforce MSC 2's new verification and rollback rules before an archive is
  runnable.
- Report platform/runtime capability separately from a server's configured
  type and imported-record state.

### Native Linux and Windows responsibilities

- Start, observe, command, stop, and recover a real native BDS process.
- Reuse the established OS process statistics and crash-detection mechanism
  for metrics; do not parse the macOS VM's `[MSCSTATS]` line in a native
  backend.
- Bind the configured UDP port directly and report port-in-use or bind
  failure honestly.
- Preserve the shared graceful stop followed by forced termination policy.
  Windows additionally owns process-tree cleanup and service-session survival.

### macOS sidecar responsibilities

The Rust sidecar client owns message IDs/order, JSON-lines framing, EOF and
malformed-frame handling, sidecar child supervision, and translation to the
shared runtime states. The Swift sidecar owns only the VZ-specific work:

- check virtualization and bundled kernel/initramfs availability;
- build the VZ VM with NAT networking and a read/write virtio-fs share of the
  server directory at the fixed `world` tag;
- boot the guest and run BDS in the appliance;
- frame guest console output, discover DHCP guest IP, and start/cancel the UDP
  relay;
- translate guest clean stop, VM error, boot failure, and forced stop into one
  terminal event; and
- parse and consume sidecar-only `[MSCSTATS]` lines.

The sidecar is not a second management API and must not persist Bedrock state
outside the shared server directory. The existing contract's `provision`,
`start`, `ready`, `stop`, `force-stop`, `terminated`, `console-line`, and
`command` messages remain the only process boundary.

P10.17's `sidecar/bedrock/BedrockSidecar` executable implements that boundary
over stdin/stdout JSON lines. Its controller validates the Intel host
precondition, virtualization availability, the bundled `vmlinuz-kata` and
`appliance-initramfs.gz` resources, and the host server directory before
accepting `provision`; builds a fresh NAT, serial-console, and read-write
virtio-fs configuration for each start; forwards guest console lines; consumes
`[MSCSTATS]`; and starts the UDP relay only after a DHCP line yields a guest IP
and the listener is ready. The executable emits one terminal `terminated`
event for clean guest shutdown, VM errors, failed starts, and forced stops.
The appliance binaries remain distribution resources, documented in the
sidecar's Resources README rather than checked into the source tree.

## What is ported and what is new

### MSC 1 behavior to preserve

The following is observable in the oracle and is a compatibility obligation:

- `BedrockPropertiesManager.swift` reads raw `key=value` properties, applies
  typed defaults, ignores unknown enum values, preserves unknown keys when the
  typed model is written, and reads/writes allowlist and permissions JSON with
  the documented missing-field behavior. The manager itself does not clamp
  numeric values; API-level settings validation is a separate layer.
- Bedrock commands differ from Java: allowlist commands replace whitelist
  commands, `save hold`/`save query`/`save resume` replace `save-all`, and
  operator changes edit `permissions.json` by XUID rather than sending
  `op`/`deop` to BDS. A leading `/` is stripped before a Bedrock command is
  sent.
- BDS readiness is the case-insensitive `Server started` substring. Version,
  connect, disconnect, reconnect, blank-XUID, and empty-gamertag console
  cases feed online-player state, the XUID name cache, profile refresh, and
  the blank-XUID allowlist backfill guard.
- BDS has no native log file in the oracle. MSC mirrors console lines to
  `logs/latest.log`, rolls the prior session to timestamped console logs, and
  keeps ten rolled logs.
- Bedrock player LevelDB data includes `player_<xuid>` and `~local_player`
  keys. The reader handles `.ldb` tables, `.log` write-ahead records,
  uncompressed and raw-deflate blocks, FULL/FIRST/MIDDLE/LAST reassembly,
  deletions, varints, and the oracle's malformed-input degradation behavior.
- Bedrock player NBT is little-endian and supports the three dimension
  branches, health/food/XP formula bands, inventory/armor/offhand, item
  field-type variants, numeric enchantment IDs, stored enchantments, and
  custom names. Invalid or truncated roots produce no partial profile.
- Bedrock worlds live at `worlds/<level-name>/`. Import resolution accepts a
  selected folder containing `level.dat`, or exactly one level-one child that
  contains `level.dat`; zero or multiple matches fall back to the selection.
  The level name is sanitized using the existing BDS-specific rules before a
  world is copied into `worlds/`.
- `WorldSlotManager.importedWorldMetadata` reads Bedrock `level.dat` as
  little-endian NBT, derives seed/difficulty/gamemode/day-time where present,
  and merges a nonblank backup sidecar seed before parsed values. Bedrock
  slots archive the `worlds` container, not Java's dimension folders.
- Running Bedrock backup creation sends `save hold`, polls `save query` for
  up to ten seconds, proceeds even when the query times out, and always sends
  `save resume` afterward. The oracle has no live-world Bedrock restore path:
  restore directs the user to the slot-based Worlds flow.
- The VZ backend starts its UDP relay only after the appliance reports a DHCP
  address, forwards each client flow independently, and cancels all upstreams
  on teardown. It sends `stop` and forces the VM after twenty seconds; an
  explicit second stop also forces it.

The Docker `BedrockServerBackend.swift` is deliberately excluded. It is a
behavioral reference only under D-008, not a third MSC 2 backend.

### MSC 2 behavior that must be labeled as new

These are not ports of MSC 1 behavior and must not be described as if the
oracle already proved them:

- checksum, signature, or other archive identity verification;
- selecting a manifest entry for the actual host platform rather than always
  reading MSC 1's `linux` entry;
- rejecting a corrupt/unverified archive and atomically rolling back to the
  previous working installation on a failed update;
- native Linux and Windows BDS process supervision, process-tree ownership,
  OS-level metrics, crash recovery, and direct host UDP binding;
- the backend-neutral `BedrockRuntime` trait and shared capability/error
  vocabulary;
- the Rust-to-Swift sidecar client implementation, message IDs/order
  validation, and sidecar crash/unavailable states as public agent results;
- the separate Bedrock compatibility matrix and evidence checker;
- reconciling an imported Bedrock record with actual host/runtime capability
  before presenting it as runnable; and
- additive public API, CLI, and copied-iOS capability disclosure needed to
  expose those runtime states.

## Download and provisioning provenance

MSC 1's `BedrockProvisioner` resolves versions through the public
`kittizz/bedrock-server-downloads` manifest, reads the `linux` URL from each
release entry (even though its destination is a macOS VM guest), supports a
pinned URL match and newest-release selection, writes `.msc_bds_version`,
preserves `server.properties`, `allowlist.json`, `permissions.json`,
`whitelist.json`, and never overwrites `worlds/` during an update. An installed
non-forced server may continue from its existing files when the manifest is
unavailable; a legacy install without a marker may be backfilled for a latest
request.

**Important provenance limit:** MSC 1's own provisioner performs no checksum
or signature verification at all. That absence is the oracle behavior. Any
checksum, signature, archive identity, platform-entry dispatch, corrupt
archive rejection, or atomic failed-update rollback added by MSC 2 is new
behavior and needs its own fixtures and evidence. It must not be described as
"preserved verification" or attributed to MSC 1.

The Phase 10 design keeps BDS staging in the agent-side common path so native
Linux, native Windows, and the macOS sidecar receive the same verified
installation. The sidecar's `provision` message remains responsible for
VM-specific appliance resources; it is not a replacement for agent-side BDS
distribution staging.

## UDP sequencing resolution

The open sequencing question in `msc2-port-plan.md` §6 is resolved here:
`UDPRelay.swift` is VM-specific host↔guest forwarding. It is not a general
Bedrock requirement. A native Linux or Windows `bedrock_server` owns the host
UDP socket directly, so no relay stage is inserted between the agent and BDS.

Consequently:

- `fixtures/bedrock-udp/` is reserved for VM-relay cases: per-client-flow
  isolation, bidirectional pump startup, cancellation cleanup, bind failure,
  and DHCP-before-relay sequencing;
- native direct-bind and port-in-use cases belong in
  `fixtures/bedrock-runtime/`; and
- a native runtime must never claim relay success as proof that its direct
  UDP bind works.

## Reconciliation with prior phases

### Phase 5 — imports and configuration

Phase 5 already detects a Bedrock import when `bedrock_server` or
`bedrock_server.exe` is present and no JAR is present. It preserves the raw
properties path, scans Bedrock worlds, writes the imported record under the
`bedrock` root, and currently records the Bedrock port from the pre-override
properties value. That pre-override `ConfigServer.bedrock_port` behavior is a
known MSC 1 quirk and must be tested before P10 changes any reconciliation
logic. A record that imports successfully is not yet proof that its BDS
binary, world, settings, or host runtime can actually run.

The current Rust workspace has import and configuration models, but no
Bedrock runtime implementation. P10 must turn an imported record into one of
three honest states: runnable on the selected backend, runnable only after a
required provisioning/capability step, or unavailable with a reason.

### Phase 6 — worlds and backups

Phase 6 reconciles live folders and `world_slots/` before world mutations. For
Bedrock that means `worlds/` is the live-folder presence check and slot
activation must preserve `worlds/<level-name>/`. Phase 6 intentionally left
production Bedrock `level.dat` repair and live save-command coordination
runtime-dependent; those are Phase 10 work. Its current public live-backup
restore guard is Java-only, which is correct for the pre-runtime state. P10
must preserve the user-visible boundary: Bedrock backup restore goes through
world slots, not a fabricated live restore operation.

### Phase 9 — networking and helpers

Phase 9 supplies the Bedrock port/address handoff used by Playit, Geyser, the
router guidance, Xbox Broadcast, and network diagnostics. P10 consumes that
resolved port and must keep player-facing UDP traffic separate from the
management listener. It does not reimplement Playit, Geyser/Floodgate, or
management authentication. The direct native UDP bind and the VM relay are
runtime internals with separate evidence, as resolved above.

The copied iOS client already consumes the shared `/start`, `/stop`,
`/command`, `/allowlist`, `/worlds/*`, `/backups*`, version, settings, and
capability surfaces, using `serverType` to decide when Bedrock controls are
shown. P10 therefore extends or fills those shared contracts additively; it
does not invent a parallel Bedrock-only management API or make iOS infer
runtime support from the server type alone.

## Owned symbol-ledger rows

The symbol ledger covers behaviors inside MSC 1 mixed/UI files; dedicated
platform, pure-domain, and I/O files in the file inventory are not silently
missing rows. P10 owns the Bedrock portions of these existing agent rows:

| Ledger rows | Existing MSC 1 areas | P10 boundary |
|---|---|---|
| 6; 49; 70–72; 191–192; 252–253 | Bedrock properties, settings, commands, allowlist, permissions | Port raw-file behavior and command semantics; keep manager behavior distinct from API validation/clamping. |
| 20–22; 15; 18 | Bedrock/host metrics | Port the shared metric result; VM `[MSCSTATS]` parsing is sidecar-only, while native metrics are MSC 2 OS-process behavior. Row 21's Docker path remains excluded by D-008. |
| 26; 56; 60 | Bedrock discovery, creation, and import routes | Reconcile existing records with actual runtime capability; do not equate import detection with readiness. |
| 39; 52–53; 181; 293; 295 | Players, hidden profiles, Bedrock world data, console player events, XUID backfill | P10 owns data/profile identity and event behavior. `BedrockSkinFetcher.swift` image/avatar resolution is explicitly deferred to Phase 11. |
| 30; 162–170; 272–278 | Bedrock world layout, slots, import metadata, creation | Preserve `worlds/<level-name>/`, Bedrock NBT metadata, sanitization, and transactional creation/import. |
| 40; 50; 209–218; 211 | Backup listing/configuration/creation/restore | Port Bedrock save coordination and logs; preserve the no-live-Bedrock-restore boundary. |
| 230–232; 47 | Bedrock versions and component update routes | Preserve version pin/latest normalization and downgrade backup guard; replace Docker/VM staging with the common verified distribution path in later steps. |
| 19; 79–80; 185–186; 188; 190–191; 281–286 | Lifecycle, process control, readiness, console, commands | Reuse the existing lifecycle substrate; add native platform supervision as new behavior and keep sidecar details behind the runtime boundary. |

The dedicated oracle files are the implementation/evidence inputs for those
rows: `BedrockPropertiesManager.swift`, `BedrockProvisioner.swift`,
`BedrockVersionFetcher.swift`, `VMBedrockServerBackend.swift`,
`BedrockPlayerDataManager.swift`, `BedrockNameCache.swift`,
`BedrockHiddenProfiles.swift`, `BedrockLevelDB.swift`,
`BedrockNBTReader.swift`, `UDPRelay.swift`, the listed `AppViewModel` files,
the embedded `RemoteAPIServer` routes, and their copied iOS consumers.

## Explicit exclusions and unresolved decisions

- `BedrockServerBackend.swift` (Docker) is excluded under D-008.
- `BedrockSkinFetcher.swift` is excluded from the Phase 10 agent capability;
  player skin/avatar presentation remains Phase 11 work.
- The sidecar contract is already frozen and is not reopened as a second API.
- D-007, “macOS Bedrock stays Swift behind a sidecar,” is **Approved**.
- D-022, “MSC platform support and Bedrock platform support are separate
  matrices,” is **Approved**.

Those two architectural decisions are now settled for Phase 10. The remaining
evidence gate is whether a particular host, appliance, or BDS version can be
advertised as supported; an unavailable result must remain explicit.

## P10.28 exact gate record

The Phase 10 gate is tied to the exact code candidate recorded by P10.27:
commit `2ccb1d0d509dcedb50e3f9c153845ee44934ff93`, tested by GitHub Actions run
`32655288252`. The later P10.27 documentation commit is not substituted for
that candidate. The run's macOS, Linux, Windows, and headless no-GUI jobs are
recorded as green, while its synthetic-only limits remain explicit: it did not
download BDS, start a live Bedrock server or VM, require a Mojang account, or
make a public-network reachability claim.

P10.28's gate checker validates the exact fixture counts, additive API and
copied-iOS contract, native Linux/native Windows/Intel-macOS-sidecar boundary,
separate compatibility matrix, real-or-unavailable distribution and runtime
records, synthetic smoke wiring, and this exact CI candidate. Its Verify line
then runs the synthetic public path and the workspace regression suite once;
the result is evidence for REVIEW, not an agent declaration that the phase is
closed.

## P10.36 production wiring guard

The Phase 10 production check is a fail-closed source audit that runs beside
the synthetic production-router smoke on Linux, Windows, and macOS, and in the
headless no-GUI job. It rejects three regressions that fixture counts alone
cannot see: a restored literal Bedrock refusal in production Rust, a
`GET /v1/capabilities` response that invents Bedrock support instead of using
the `BedrockRuntimeSelection` chosen by the composition root, and any frozen
API response DTO that loses its additive `BedrockRuntimeStateDto` field. It
also requires the smoke to launch the real `msc serve` composition root and
checks that the smoke source remains offline: committed fixtures and loopback
HTTP only, with no BDS download, provider, public-network, or
Virtualization.framework activity.

The check is intentionally source-level and does not claim live BDS or VM
support. The executable lifecycle and public integration proof remain
`phase10-smoke.sh --synthetic`; the check makes it harder for that proof to
be replaced with a detached fake harness or for the production path to drift
back to Phase 9's pre-Bedrock refusal behavior.
