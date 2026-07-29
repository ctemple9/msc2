# MSC 2 Port Audit — Claude's Independent Pass

**Date:** 2026-07-29
**Scope:** `MSCmacOS/MSCmacOS Swift/` — 246 files, 97,603 LOC
**Method:** comments stripped before all reference counting; buckets assigned by dependency direction, not imports
**Blind:** produced without seeing Codex's classification

---

## 0. Correction to my own hypothesis

I argued that import counts understate entanglement, and predicted that "core-looking" files would turn out to be secretly coupled to `AppViewModel` and Combine.

**That was mostly wrong, and the data says so clearly.**

My first pass flagged `StartupCrashAnalyzer.swift`, `TpsLineParser.swift`, `AddonUpdateResolver.swift`, `ModpackClientOnlyClassifier.swift`, and `RouterPortForwardGuideCatalogLoader.swift` as entangled. Every one of those hits was **a comment saying the opposite**:

```
StartupCrashAnalyzer.swift:6   // Pure service — no AppViewModel
TpsLineParser.swift:7          // can be unit-tested without an AppViewModel instance or its @Published state
AddonUpdateResolver.swift:11   // Pure service: no AppViewModel dependency
RouterPortForwardGuideCatalogLoader.swift:5
                               // This file intentionally stays independent from SwiftUI, AppConfig, and AppViewModel
```

With comments stripped, **only 2 of 246 files** carry observable state outside a View or an `AppViewModel` extension: `ConsoleManager.swift` and `AppUpdateChecker.swift`.

The layering is deliberate, documented in file headers, and it held. Codex's cruder import-based count was closer to the truth than my critique of it implied. My methodology point survives in principle — imports genuinely aren't the right metric — but in *this* codebase the two metrics nearly agree, because the separation was done on purpose.

---

## 1. Buckets

Non-overlapping; sums to the full tree.

| Bucket | Files | LOC | Meaning |
|---|---:|---:|---|
| **1 — Pure logic** | 44 | 12,796 | Parsers, models, schemas, decision trees. No I/O. Fixture-testable. |
| **2a — I/O orchestration** | 36 | 9,910 | Download/verify/install, filesystem, process spawn. Portable in design. |
| **2b — VM orchestration** | 44 | 18,839 | `extension AppViewModel` + `AppViewModel.swift`. Domain logic wearing a view-model hat. |
| **3 — Platform** | 13 | 4,832 | Touches an Apple framework. |
| **4 — UI** | 109 | 51,226 | SwiftUI views. Replaced by Svelte, not ported. |
| **Total** | **246** | **97,603** | |

---

## 2. What actually gets ported

The 97.6K headline is misleading. Three large chunks are not port targets:

| Chunk | LOC | Disposition |
|---|---:|---|
| SwiftUI views | 51,226 | **Rewritten** as Svelte. Design carries over, code doesn't. |
| `RemoteAPIServer*.swift` (6 files) | 5,652 | **Becomes the OpenAPI spec.** Highest-value non-code artifact in the repo — 49 POST + 38 GET routes and a 1,504-LOC DTO file that is already a wire schema iOS exercises daily. |
| `AppViewModel+APIWiring*.swift` (7 files) | 2,701 | **Deleted.** This is glue between the view model and the API providers. In MSC 2 the API talks to the engine directly; the layer ceases to exist. |

**Engine actually requiring translation: ≈ 38,000 LOC.**

Of which ~6,472 is the `RouterPortForward*` subsystem — a static knowledge base of router models and guide text. That's a **data migration to JSON**, not a code port. Real logic to translate: **≈ 31,500 LOC.**

---

## 3. Apple-framework dependencies — only one is a genuine blocker

I expected several. There's one.

| File | Framework | Rust story |
|---|---|---|
| `VMBedrockServerBackend.swift` (451) | Virtualization | **Genuine blocker.** Bridgeable via `objc2`, but it's a delegate-heavy async framework you already have working. **Swift sidecar.** |
| `KeychainManager.swift` (233) | Security | `keyring` crate — cross-platform, solves Windows/Linux at the same time |
| `PlayerSkinRenderer` / `PlayerSkinStore` / `BedrockSkinFetcher` (435) | AppKit (NSImage) | `image` crate |
| `ResourcePackHostServer` / `UDPRelay` (205) | Network.framework | `axum` / `tokio` |
| `AppUpdateChecker.swift` (135) | AppKit, Combine | Tauri updater |
| `WorldSlotManager.swift` (1,495) | AppKit | **Misfiled by import.** Actual AppKit use: 2× NSImage (thumbnail), 1× NSSavePanel, NSError. ~30 LOC of macOS inside 1,495 LOC of world logic. Reclassify to 2a. |
| `BedrockServerBackend.swift` (658) | AppKit | 1× NSWorkspace, 1× NSLock, 2× POSIX `kill`. Reclassify to 2a. |
| `RemoteAPIServer.swift` (902) | Darwin | 1× `Darwin.bind`. Socket setup. Trivial. |

**Verdict: `Virtualization` is the only thing that needs a Swift sidecar.** Design that IPC boundary in Wave 0.

---

## 4. The `AppViewModel` extensions are not UI code

44 files / 18,839 LOC is the scariest-looking bucket. Measured UI-state density (lines referencing any of the 130 `@Published` properties):

| File | LOC | Lines touching published state | % |
|---|---:|---:|---:|
| `AppViewModel+ServerControls.swift` | 1,134 | 137 | 12% |
| `AppViewModel+APIWiringServerMgmt.swift` | 587 | 26 | 4% |
| `AppViewModel+Backups.swift` | 997 | 23 | 2% |
| `AppViewModel+ServerImport.swift` | 511 | 7 | 1% |
| `AppViewModel+WorldManagement.swift` | 313 | 7 | 2% |

`AppViewModel` is being used as a **namespace**, not as state. These files are service orchestration that happens to be declared as extensions. They translate; they don't need re-deriving.

---

## 5. A test corpus already exists

I claimed you had no regression fixtures. You have **21 test files, 4,888 LOC, 270 test cases** — concentrated on exactly the risky domain:

| Tests | File |
|---:|---|
| 30 | `DTOContractTests` |
| 27 | `TpsMonitoringTests` |
| 21 | `ComponentVersionParsingTests` |
| 19 | `HeadlessScriptGeneratorTests` |
| 18 | `ModpackClientOnlyTests` |
| 16 | `ServerSettingsSchemaTests`, `HTTPParseRequestTests`, `CurseForgeModpackTests` |
| 15 | `JavaRuntimeGuardsTests` |
| 13 | `NetworkSafetyTests`, `ModpackPinningTests` |
| 12 | `RemoteAPIIntegrationTests`, `ArgsFileResolutionTests` |
| 11 | `ConnectorCrashAnalysisTests` |
| 7 | `StartupCrashAnalyzerTests`, `ServerPropertiesModelTests`, `PackManagedGuardTests`, `AppConfigRoundTripTests` |
| 3 | `MrpackExtractionTests` |

**Caveat that still matters:** expectations are **inline Swift string literals**, not external fixture files. They can't be reused by a Rust implementation as-is. Wave 0 should extract them into language-neutral fixtures (`input file` + `expected JSON`) — a mechanical transform that converts 270 existing tests into the Rust acceptance harness.

Also note `MSCmacOSTests/iOSModelMirrors.swift` — you already hand-maintain and test the macOS↔iOS model translation. That's the cost the OpenAPI-codegen recommendation eliminates.

---

## 6. Proposed port order

Wave 0 is prerequisite. Waves 1+ are vertical slices — each lands in engine, API, and clients together.

| Wave | Content | LOC | Why here |
|---|---|---:|---|
| **0** | `RemoteAPIServer*` → OpenAPI spec; extract 270 tests → JSON fixtures; design VZ sidecar IPC | — | No Rust yet. Produces the spec and the acceptance harness. |
| **1** | `ServerBackend`, `JavaServerBackend`, `ServerProcessManager`, `ServerLifecycleManager`, `ConsoleManager`, `TpsLineParser` + `AppViewModel+ServerControls`/`+OutputHandling` | ~4,000 | **Start/stop/console/status.** Point the existing iOS app at it. First working software. |
| **2** | `AppConfig`, `ConfigManager`, `ServerPropertiesManager`, settings schema + `AppViewModel+ServerSettings`/`+ConfigHelpers`/`+ConfigRecovery` | ~2,500 | Config is a dependency of everything downstream. |
| **3** | `WorldSlotManager`, `AppViewModel+WorldManagement`/`+WorldRepair`, backups | ~4,500 | Highest data-loss risk — port while the codebase is still small enough to review carefully. |
| **4** | `ModrinthAPI`, `CurseForgeAPI`/`Modpack`, `ModJarMetadataParser`, `ModpackClientOnlyClassifier`, `AddonUpdateResolver`, `NeoForgeInstaller`, `ServerJarProviders`, `PaperDownloader`, `JavaRuntimeManager` | ~4,500 | Biggest domain chunk, best test coverage, most accumulated edge cases. |
| **5** | `StartupCrashAnalyzer` + connector analysis | ~600 | Pure logic, 18 existing tests. Fast once fixtures exist. |
| **6** | playit, broadcast, Geyser, DuckDNS, resource packs | ~3,000 | Independent of everything above. |
| **7** | Bedrock — **Linux native first**, macOS VZ sidecar last | ~2,500 | Linux path validates the design without the sidecar in the way. |
| **∥** | `RouterPortForward*` → JSON data migration | 6,472 | Anytime. Data, not code. |

Windows/Tauri stays last, as previously argued.

---

## 7. Points where I expect to disagree with Codex

Surface these first in the diff — they're where the classification is genuinely ambiguous.

1. **The ~44 `AppViewModel+*` files.** Codex's "no SwiftUI/AppKit import ⇒ core-looking" test passes them. I bucket them as view-model orchestration. Same files, opposite label. Codex's count of ~102 core-looking vs my 44+36=80 in buckets 1/2a is largely this.
2. **`WorldSlotManager` and `BedrockServerBackend`.** Import-based classification calls them AppKit/platform. I say the AppKit use is incidental (~30 LOC) and they're I/O orchestration.
3. **`RemoteAPIServer*`.** I classify these as *not port targets at all* — they become the spec. A file-by-file audit that treats them as code to translate will overstate the port by ~5,700 LOC.
4. **`CryptoKit` files** (`ModrinthAPI`, `ResourcePackManager`, `PlayerDataManager`). Apple-only framework, but irrelevant to a Rust port — hashing is native. Should not count as platform-coupled.

---

## 8. Bottom line

The codebase is in materially better shape for this port than either of us assumed. The layering is real and intentional, there's a 270-case test corpus aimed at the risky parts, the API surface is already specified in code, and exactly one Apple framework requires a sidecar.

Revised estimate: **~31,500 LOC of genuine engine logic to translate**, not ~97,000. The dominant remaining risk is not entanglement — it's that the 270 tests encode expectations as inline Swift and must be freed into fixtures before any Rust is written.

Do Wave 0 before writing a line of Rust.
