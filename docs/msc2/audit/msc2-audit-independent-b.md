# MSC 2 Independent Portability Audit

**Author:** Codex  
**Date:** July 29, 2026  
**Status:** Independent pass completed before seeing any Claude file classification
**Repository commit:** `fccd61f0ed743086f1f5db6bef58e228a36010f3` (clean worktree)

## Scope

This audit covers every production Swift file under:

`MSCmacOS/MSCmacOS Swift/`

That is **246 files and 97,357 lines**.

The 21 files in `MSCmacOS/MSCmacOSTests/` are assessed separately as verification assets. The iOS client is not classified as engine code; it is treated as an external production consumer of the Remote API.

This is a static architectural audit, not a proof of cross-platform compilation. Classification used:

- Imports and framework use
- Type and symbol declarations
- References to `AppViewModel` and observable state
- Process, filesystem, networking, security, and platform APIs
- Route/provider relationships
- Existing tests
- Manual inspection of high-risk and ambiguous files

The accompanying `msc2-file-inventory-b.csv` contains one row for every production file.

---

## Executive conclusion

MSC should not be rewritten from a blank page, but Rust remains a sensible destination for the long-running MSC 2 agent.

The present codebase contains three different kinds of value:

1. **Behavior that can be translated and verified** — parsers, compatibility rules, DTOs, server flavor rules, crash signatures, router guidance, and data models.
2. **Workflows whose design must be preserved but whose implementation should become idiomatic Rust** — downloads, installs, backups, world operations, process management, component updates, and remote operations.
3. **Presentation and Apple-platform code that should not be translated** — SwiftUI views, AppKit controls, Keychain integration, launchd integration, and the Virtualization.framework host adapter.

The difficult boundary is the **59 mixed files containing 30,942 lines**. These combine domain behavior, I/O, application state, and user presentation. They should be split by symbol and behavior before their SwiftUI shell is replaced.

The current Remote API is already a substantial migration asset:

- 49 POST routes
- 38 GET routes
- 91 provider declarations
- Approximately 55 KB of DTO/wire schema
- Authentication, roles, named permissions, rate limiting, audit logging, and WebSocket support

The API is not a prototype to discard. Its externally observable behavior should become the starting contract for MSC 2.

---

## Quantitative classification

| Primary bucket | Files | Lines | Share | Meaning |
|---|---:|---:|---:|---|
| UI and application state | 102 | 42,736 | 43.9% | Rebuild in Tauri/web; preserve UX and extract hidden behavior |
| Mixed; symbol-level split required | 59 | 30,942 | 31.8% | Highest migration risk |
| I/O orchestration | 35 | 8,114 | 8.3% | Preserve workflow; reimplement behind Rust service traits |
| Pure/domain logic | 29 | 7,272 | 7.5% | Translate with characterization fixtures |
| API and wire contracts | 8 | 5,652 | 5.8% | Treat observable behavior as normative |
| Platform-specific | 12 | 1,984 | 2.0% | Replace with per-OS adapters or retain sidecars |
| Legacy backend | 1 | 657 | 0.7% | Do not port without an explicit compatibility reason |

Two additional coupling measurements matter:

- The 44 `AppViewModel*.swift` files contain **20,109 lines**.
- **140 of 246 production files** reference the `AppViewModel` symbol.

The second number confirms that dependency direction, not imports, is the real portability issue.

---

## Bucket definitions

### Pure/domain logic

Behavior whose result primarily depends on input values rather than operating-system state. These files are suitable for translation into Rust with shared input/output fixtures.

Examples:

- TPS line parsing
- Startup crash signatures
- Version comparison
- Server flavor rules
- Modrinth slug normalization
- Router guide matching and troubleshooting
- BDS NBT parsing
- Java launch argument construction

“Pure” does not mean line-for-line translation is required. It means behavior can be specified precisely and tested without launching the app.

### I/O orchestration

Workflows that read files, invoke tools, call providers, download assets, or mutate server state, but are not fundamentally tied to SwiftUI.

These should generally be reimplemented in Rust rather than transliterated. Their ordering, validation rules, rollback behavior, filenames, provider quirks, and failure semantics must be preserved.

### API and wire contracts

Routes, DTOs, authentication behavior, permissions, errors, request limits, and WebSocket framing. The current Swift implementation is both executable documentation and a compatibility oracle.

The Rust API does not need to preserve internal provider closures, but it should preserve externally observable behavior until a deliberately versioned contract supersedes it.

### Platform-specific

Code whose purpose is inherently tied to one host implementation:

- Apple Virtualization.framework
- Keychain
- AppKit image handling
- Network.framework
- launchd watchdog behavior
- macOS process scanning

MSC 2 should expose shared traits for these capabilities and provide explicit platform implementations.

### UI and application state

SwiftUI views, visual design, presentation state, window controllers, and observable objects that exist primarily to serve the macOS interface.

The user experience is an important reference. The SwiftUI implementation itself is not an engine asset. Any parser, validation rule, or filesystem action embedded in these files must be moved out before the file is retired.

### Mixed; symbol-level split required

Files where a whole-file classification would be misleading. They contain two or more of:

- Domain policy
- Filesystem or process orchestration
- API adaptation
- Observable UI state
- Apple-specific behavior
- Presentation

These files are the primary subject of extraction work.

### Legacy

The original Docker-based Bedrock backend is classified separately. It can remain a behavioral reference, but it should not define the MSC 2 runtime unless container-based Bedrock on Linux becomes an explicit supported backend.

---

## Main architectural finding

The existing API boundary is real, but the engine boundary is incomplete.

`RemoteAPIServer` already separates transport from application state through providers. However, most providers are wired directly to `AppViewModel`, and many operations still use UI-owned mutable state to coordinate progress, errors, timers, selections, and results.

The important extraction is therefore:

```text
Current

SwiftUI → AppViewModel → managers/backends/files
                    ↘ Remote API providers

Desired

SwiftUI / iOS / CLI / Web
             ↓
      versioned MSC API
             ↓
   application services / operations
             ↓
 domain rules + platform adapters + repositories
```

The existing API answers much of “what can clients ask MSC to do?” The mixed files still need to answer:

- Which state belongs to a server?
- Which state belongs to a running operation?
- Which state belongs only to a client presentation?
- Which actions must be serialized?
- What survives agent restart?
- What is the rollback boundary?

---

## Highest-risk mixed files

| File | Why it is difficult | Behavior that must survive | Suggested destination |
|---|---|---|---|
| `AppViewModel.swift` | Central object owns observable state, managers, process references, API, console, timers, and presentation | Initialization order, selected/running server distinction, output batching, state transitions | Delete as an engine object; replace with agent services plus client stores |
| `AppConfig.swift` | Domain model, persistence schema, defaults, secrets, UI labels, and host paths share one file | Every coding key, default, migration, secret exclusion, server identity | Versioned config domain plus migration and secret repositories |
| `ConfigManager.swift` | JSON persistence, corruption recovery, Keychain, defaults, and app lifecycle are combined | Atomicity, corruption preservation, historical decoding, secret separation | Rust config repository plus platform secret store |
| `AppViewModel+ServerControls.swift` | Lifecycle, commands, players, backups, restart policy, initiation workflow, and UI presentation are intertwined | Start/stop state machine, graceful shutdown, auto backup/restart, command semantics | Server operation service and persisted operation state |
| `ServerProcessManager.swift` | Process validation, launch, stream framing, command input, termination, and cleanup | Java validation, process ownership, line buffering, graceful/forced stop | Per-OS process supervisor behind a common trait |
| `AppViewModel+OutputHandling.swift` | Valuable parsers are hidden inside state mutation and notification behavior | Ready detection, player joins, TPS, world time, fatal errors, broadcast auth | Pure event parsers feeding an agent event reducer |
| `ConsoleManager.swift` | Pure line parsing and bounded log behavior share Combine/UI filtering state | Level/tag inference, sanitization, retention bounds, chat extraction | Agent console buffer plus client-side filters |
| `OverviewChatCardView.swift` | A console chat parser is embedded below a SwiftUI card | Chat, advancement, join, and leave parsing | Pure console-event parser; rebuild card in frontend |
| `AppViewModel+Backups.swift` | Save-off/save-all coordination, console waits, ZIP creation, metadata, pruning, restore, and UI state | Consistent online backup, metadata, retention, restore safety, timeout behavior | Transactional backup service with operation journal |
| `WorldSlotManager.swift` | Mostly portable world logic, but includes AppKit thumbnails and shell ZIP behavior | Slot metadata, activation, import inference, duplicate/copy/export semantics | World repository and archive service; separate image adapter |
| `AppViewModel+WorldSlots.swift` | UI state coordinates destructive world operations and backup restore | Preconditions, active-slot identity, rollback, import/export behavior | World operation service |
| `AppViewModel+WorldManagement.swift` | Rename/replace/unzip/copy logic and rollback are embedded in view-model methods | Folder-set semantics, validation, rollback after partial moves | Transactional world mutation service |
| `AppViewModel+ServerTransfer.swift` | Archive creation, manifest generation, import inspection, secrets, conflicts, and UI progress | Manifest format, sanitization, port conflict behavior, secret handling | Versioned transfer package service |
| `AppViewModel+ServerCreation.swift` | Provider selection, directory creation, initial slots, cross-play templates, archives | Resulting directory structure and metadata for every server flavor | Server factory with provider-specific provisioners |
| `AddServerWizardView.swift` | A 2,779-line UI also performs staging, process checks, archive handling, and source inspection | Validation and staging behavior, not SwiftUI navigation | Thin frontend wizard backed by draft/validation API |
| `AppViewModel+ServerImport.swift` | Server detection heuristics and filesystem import share state updates | Loader/version/world detection and conflict rules | Pure detector plus import transaction |
| `AppViewModel+ModManagement.swift` | `.mrpack`, CurseForge, dependency install, overrides, and filesystem mutation are combined | Manifest interpretation, client-only decisions, override precedence | Modpack import service with provider adapters |
| `ModpackClientOnlyClassifier.swift` | Policy is mostly portable but reads/modifies JAR state and references app models | Precedence of manifest, Modrinth, and JAR evidence | Pure classification policy plus file adapter |
| `AppViewModel+AddonUpdates.swift` | Resolution, provenance mutation, download/install, and repair state are combined | Pinning, trusted links, dependency repair, safe replacement | Add-on update operation service |
| `AppViewModel+ComponentsVersions.swift` | Local scans, online checks, updates, Bedrock provisioning, archives, and UI state | Version source priority, downgrade checks, archive behavior | Component inventory and update services |
| `AppViewModel+HealthCards.swift` | Filesystem, Java, VM, ports, network probes, process execution, and UI card text | Diagnostic checks and severity decisions | Diagnostic rule engine plus platform probes |
| `AppViewModel+Playit.swift` | Output parsing, credentials, HTTP API, tunnel provisioning, SVC config, and UI prompts | Claim flow, tunnel identity, address parsing, secret lifecycle | Playit provider service plus secret repository |
| `ResourcePackManager.swift` | Java and Bedrock pack formats, extraction, metadata, UUIDs, activation, hashing | Pack discovery, validation, activation and safe removal | Resource-pack domain and archive service |
| `JavaRuntimeManager.swift` | Portable version policy and platform-specific runtime discovery share one type | Java requirements, normalization and selection rules | Pure Java policy plus per-OS discovery adapter |
| `JavaInstaller.swift` | Download/install logic and SwiftUI presentation share one file | Provider selection, archive extraction, runtime verification | Java provisioner plus frontend operation view |
| `SetupWizardView.swift` | Onboarding UI performs prerequisite and environment probes | Probe definitions and resulting setup decisions | Frontend wizard using host-capability API |
| `PrerequisitesView.swift` | Java, VM, process, filesystem, and UI checks are combined | Accurate prerequisite detection | Agent capability/diagnostic checks |
| `ServerSettingsView.swift` | Draft models and validation sit beside SwiftUI controls | Validation and mapping between Java/Bedrock settings | Shared settings schema; frontend forms generated from schema |
| `MinecraftCommandRegistry.swift` | Command definitions are portable, but category colors are SwiftUI values | Commands, arguments, server compatibility, suggestion rules | JSON/Rust command catalog plus frontend presentation map |
| `VMBedrockServerBackend.swift` | Proven delegate-heavy VZ lifecycle, shared directory, process channel, and VM state | Current working macOS Bedrock behavior | Retained Swift macOS sidecar behind Bedrock runtime protocol |

---

## Strongest translation candidates

These are the best places to establish Rust parity because they have clear inputs, clear outputs, and valuable existing knowledge:

1. `ComponentVersionParsing.swift`
2. `TpsLineParser.swift`
3. `StartupCrashAnalyzer.swift`
4. `ModrinthSlugNormalizer.swift`
5. `PluginNameParser.swift`
6. `PluginSourceDetector.swift`
7. `JavaServerFlavor.swift`
8. `JavaServerLaunchHelper.swift`
9. `BedrockNBTReader.swift`
10. `AppViewModelModels.swift`, especially `ServerPropertiesModel`
11. `RouterPortForwardGuideMatcher.swift`
12. `RouterPortForwardFallbackDecisionTree.swift`
13. `RouterPortForwardTroubleshootingEngine.swift`
14. `HeadlessScriptGenerator.swift` as compatibility behavior, even though MSC 2 headless mode supersedes scripts

These should use language-neutral fixtures wherever possible. Rust output should be compared against expected values, not against implementation details of Swift.

---

## I/O workflows that should be specified before reimplementation

These are portable in design but too stateful for safe line-by-line translation:

- Add-on resolution and provenance
- Bedrock LevelDB reads
- Bedrock and Java player data reads
- Bedrock properties, allowlist, and permissions
- Server JAR downloads
- Forge and NeoForge installer execution and args-file discovery
- CurseForge and Modrinth pack import
- Resource-pack activation
- Backup create/prune/restore
- World slots, replacement, repair, and conversion
- Server transfer packages
- Playit tunnel lifecycle
- Xbox Broadcast helper lifecycle
- Config persistence and migration

Each needs fixture-backed examples of successful, interrupted, malformed, and rollback cases.

---

## Platform boundary

| Current file | MSC 2 treatment |
|---|---|
| `VMBedrockServerBackend.swift` | Keep as a macOS Swift sidecar initially; expose a small process protocol to Rust |
| `UDPRelay.swift` | Replace with Rust UDP relay or retain inside the Bedrock sidecar if VM addressing requires it |
| `KeychainManager.swift` | Implement a `SecretStore` trait: Keychain, Windows credentials/DPAPI, Linux protected store |
| `WatchdogRunner.swift` | Replace with launchd, Windows Service recovery, and systemd policies |
| `JavaProcessScanner.swift` | Implement per-OS process enumeration and ownership checks |
| `PlayerSkinRenderer.swift` | Move rendering to the frontend or a portable image library |
| `PlayerSkinStore.swift` | Separate image bytes/metadata from AppKit image types |
| `BedrockSkinFetcher.swift` | Return data/decoded portable images; do not expose `NSImage` from the agent |
| `ResourcePackHostServer.swift` | Reimplement using the Rust HTTP stack rather than Network.framework |
| `AppUpdateChecker.swift` | Split release metadata from platform installer/update behavior |
| `AppUtilities.swift` | Split networking helpers, process helpers, and platform presentation |
| `QuickStartWindowController.swift` | Rebuild as Tauri window behavior; no engine port |

`BedrockServerBackend.swift`, the legacy Docker backend, should not be ported by default. A future Linux compatibility-container backend should be designed around current requirements rather than inherited from this obsolete implementation.

---

## Remote API assessment

### Assets to preserve

- Wire field names and optional/default behavior
- Current route meanings
- Role and named-permission behavior
- Rate-limiting intent
- Request-size protections
- 404-versus-405 behavior
- Audit records
- WebSocket authentication and console/status delivery
- iOS-visible error semantics

### Internal details that need not survive

- Mutable provider closure storage
- Hand-written socket parsing if a mature Rust HTTP stack replaces it
- `AppViewModel` as the provider owner
- Internal DTO nesting dictated only by Swift file organization

### Contract recommendation

The current routes and DTOs should be converted into a versioned OpenAPI description plus explicit WebSocket event schemas. The Swift implementation and iOS DTO contract tests remain the compatibility oracle while the specification is being validated.

MSC 2 should introduce an explicit support window rather than promising indefinite compatibility:

- Agent reports API major/minor and capabilities.
- Additive minor changes remain backward compatible.
- Breaking changes require a new major route namespace or negotiated protocol.
- The supported iOS/desktop version window is stated in releases.

---

## Existing test assets

The macOS test target contains **21 files, 4,888 lines, and 270 test methods**.

| Test file | Migration value |
|---|---|
| `AppConfigRoundTripTests.swift` | High; config keys, defaults, secret exclusion, and corruption handling |
| `ArgsFileResolutionTests.swift` | High; Forge/NeoForge installation layout quirks |
| `ComponentVersionParsingTests.swift` | High; direct language-neutral parser fixtures |
| `ConnectorCrashAnalysisTests.swift` | Very high; accumulated crash and alias behavior |
| `CurseForgeModpackTests.swift` | Very high; pack/loader interpretation |
| `DTOContractTests.swift` | Critical; wire compatibility oracle |
| `HTTPParseRequestTests.swift` | Medium; behavior matters, but a mature Rust HTTP stack replaces parser internals |
| `HeadlessScriptGeneratorTests.swift` | Medium; preserves launch-command compatibility |
| `JavaRuntimeGuardsTests.swift` | High; runtime policy and normalization |
| `ModpackClientOnlyTests.swift` | Very high; subtle policy precedence and safe file disabling |
| `ModpackPinningTests.swift` | Very high; provider and loader pinning rules |
| `MrpackExtractionTests.swift` | High; archive permission and malformed-manifest behavior |
| `NetworkSafetyTests.swift` | Critical; private-host and Tailscale exposure policy |
| `PackManagedGuardTests.swift` | Critical; prevents unsafe pack mutation |
| `RemoteAPIIntegrationTests.swift` | Critical; authentication, permissions, rate limiting, routing, WebSocket and audit behavior |
| `RemoteAPITestSupport.swift` | Support code; useful while Swift remains the oracle |
| `ServerPropertiesModelTests.swift` | Critical; preserving unknown properties prevents destructive rewrites |
| `ServerSettingsSchemaTests.swift` | Critical; validation, clamping, rejection and canonical tokens |
| `StartupCrashAnalyzerTests.swift` | Very high; direct parser fixtures |
| `TpsMonitoringTests.swift` | Very high; loader-specific production formats |
| `iOSModelMirrors.swift` | Critical support asset for DTO drift detection |

### Important weakness

Most current tests exercise pure parsing and API contracts. The most destructive workflows have much less coverage than their risk warrants.

---

## Missing characterization and fixture coverage

Before a Rust implementation becomes authoritative for user data, the following need golden fixtures or integration tests:

### Worlds and backups

- Complete world-slot create/activate/duplicate/copy/delete/import/export matrix
- Java multi-folder worlds (`world`, `world_nether`, `world_the_end`)
- Bedrock world layouts
- ZIP path traversal and symlink escape attempts
- Backup while running, including `save-off`, `save-all`, timeout, and resume
- Failed archive creation
- Interrupted restore
- Retention rules when only one known-good backup exists
- Rollback after partial rename or replacement
- Real historical MSC backup metadata

### Process lifecycle

- Partial output lines and mixed newline conventions
- Graceful stop timeout followed by forced termination
- Process tree cleanup
- Agent restart while a server remains running
- Duplicate launch prevention
- Port conflict
- Java executable validation on macOS, Windows, and Linux
- Forge/NeoForge args-file launches on all path syntaxes
- Service logout/reboot behavior

### Modpacks and components

- Real `.mrpack` and CurseForge server packs
- Overrides precedence and permission bits
- Blocked/manual CurseForge files
- Missing and circular dependencies
- Pack-managed update refusal
- Hash/provenance matching
- Atomic JAR replacement and rollback
- Interrupted downloads and corrupted archives
- Loader installers across representative Minecraft generations

### Console and diagnostics

- Java and Bedrock join/leave lines
- Chat and advancement lines
- Broadcast authentication prompts
- Ready-state detection for every server family
- Soft-failure scanning
- Crash logs gathered from real failures
- Bounded console history and reconnect behavior

### Bedrock

- Real compacted LevelDB tables and write-ahead logs
- Bedrock NBT and player records from multiple BDS versions
- Allowlist and permissions round trips
- VM boot/readiness/stop/crash lifecycle
- UDP relay behavior and shutdown
- Host-directory persistence across VM replacement

### API

- At least one authorization and validation test for every mutating route
- WebSocket reconnect, backpressure, bounded history, and malformed frames
- Idempotency for retried long operations
- Cancellation and agent restart during operations
- File transfer size, traversal, partial upload, and atomic completion
- Old-client/new-agent and new-client/old-agent compatibility fixtures

### Configuration and migration

- A corpus of historical `server_config_swift.json` versions
- Missing, renamed, malformed, and unknown fields
- Duplicate or conflicting server IDs and paths
- Secret migration on all operating systems
- Atomic-write interruption
- Coexistence rules when MSC 1 and MSC 2 see the same server

---

## Proposed domain port order

This order minimizes irreversible work and produces useful parity checks early.

### 0. Freeze behavioral evidence

Create language-neutral fixtures from current tests and real user data. Capture API routes, DTO examples, config examples, launch commands, logs, packs, and server-directory shapes.

This is not a rewrite phase; it establishes what “same behavior” means.

### 1. Domain types and pure rules

Port server identity, server flavor, version comparison, Java policy, property models, command definitions, TPS parsing, crash analysis, slug normalization, and router logic.

These establish Rust conventions and a cross-language parity harness without touching user files.

### 2. API contract and operation model

Write the versioned HTTP and WebSocket contract from the existing implementation. Define operation IDs, progress, errors, capabilities, and cancellation. Implement a skeletal agent whose routes can be exercised without real server mutation.

### 3. Filesystem, configuration, secrets, and audit substrate

Implement:

- Approved server roots and path safety
- Atomic writes
- Versioned configuration and migrations
- Secret-store trait
- Audit log
- Download staging and checksum verification
- Operation journal

Every later domain depends on these safety properties.

### 4. Java lifecycle vertical slice

Support one imported Paper server end to end:

- Import/detect
- Start
- Console
- Command
- Status and metrics
- Graceful stop
- Restart
- API and CLI control

This proves headless service ownership on macOS and Linux before broad component work.

### 5. World and backup safety

Port world discovery, slots, transactional mutations, backups, retention, verification, and restore. This should precede broad UI parity because it is the highest data-loss domain.

### 6. Java provisioning and server families

Add Vanilla, Purpur, Fabric, NeoForge, and Forge provisioning, runtime selection, installer flows, archive behavior, and startup diagnostics.

### 7. Mods, plugins, and modpacks

Port Modrinth/Hangar/CurseForge providers, metadata parsing, dependencies, client-only classification, pack-managed guards, import, update, and client export.

### 8. Networking and helpers

Add Playit, resource-pack hosting, DuckDNS, port diagnostics, Xbox Broadcast, Geyser/Floodgate, notifications, and helper process management.

### 9. Bedrock runtimes

Implement a common Bedrock runtime contract:

- Linux native backend
- Windows native backend
- Optional Linux compatibility container
- Existing macOS VZ behavior through a Swift sidecar

Port Bedrock files, properties, players, LevelDB, allowlist, permissions, metrics, and UDP behavior against shared fixtures.

### 10. Cross-platform desktop and web clients

Build the Tauri/web interface against the proven API. Preserve the current MSC information architecture and design language. UI completion should not gate headless agent correctness.

---

## Recommended Rust boundaries

```text
msc-domain
  server models, flavors, versions, settings, diagnostics,
  parsed events, operation states, capabilities

msc-application
  lifecycle, backups, worlds, imports, updates, provisioning,
  players, packs, notifications

msc-infrastructure
  filesystem repositories, HTTP providers, archives, process supervisor,
  metrics, config, audit, secret-store traits

msc-api
  HTTP routes, DTOs, WebSocket events, authentication, permissions

msc-agent
  service startup, dependency assembly, operation recovery

msc-cli
  local/remote commands

msc-platform-macos
  launchd, Keychain adapter, Swift VZ sidecar client

msc-platform-windows
  Windows Service, credentials, Job Objects, firewall integration

msc-platform-linux
  systemd, process/cgroup integration, protected secret store

msc-desktop
  Tauri shell and shared web frontend
```

The number of crates can initially be smaller. These are dependency boundaries, not a requirement to create every package immediately.

---

## Final assessment

### Is a Rust rewrite justified?

Yes, if Windows, Linux, macOS, first-class services, and a shared Tauri/web interface remain firm product requirements.

Rust is justified by the desired deployment model, process and filesystem control, service ecosystem, portable distribution, and long-term architecture—not by saving tens of megabytes over a Swift agent.

### Is a blank-slate rewrite justified?

No.

The current Swift code, tests, API, and real server behavior must serve as the executable specification. The right unit of migration is a verified domain or vertical slice, not a percentage of files translated.

### Should the macOS VZ implementation be rewritten first?

No.

The working Swift implementation should remain behind a narrow sidecar protocol until the shared Bedrock runtime contract is proven on native Linux and Windows. Rewriting the delegate and lifecycle bridge offers high risk with little early product value.

### What is the single most important next artifact?

A behavior and fixture corpus, followed by a disagreement-first comparison of this audit and Claude’s independent classification.

The files where the two audits disagree should receive manual symbol-level review. Those disagreements will be the best predictor of hidden migration cost.
