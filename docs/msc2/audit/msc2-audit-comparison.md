# MSC 2 Audit Comparison

**Compared:** Codex independent audit and Claude independent audit  
**Repository:** `fccd61f0ed743086f1f5db6bef58e228a36010f3`  
**Date:** July 29, 2026

## Outcome

The two audits converge on the important decision:

> Build MSC 2 around a Rust agent, but port verified behavior domain-by-domain rather than performing a blank-slate rewrite.

Both audits independently conclude:

- The current codebase contains deliberate separation and substantial reusable behavioral knowledge.
- The Remote API is a production contract, not a prototype.
- Existing tests should be converted into language-neutral fixtures before Rust becomes authoritative.
- SwiftUI is replaced rather than translated.
- Most Apple-specific code becomes ordinary cross-platform adapters.
- The proven Virtualization.framework Bedrock implementation should remain a Swift sidecar initially.
- Linux-native Bedrock should validate the shared runtime contract before the macOS sidecar is integrated.
- Worlds and backups require stronger tests before porting because they carry the greatest data-loss risk.

The disagreement is mainly about **how cleanly the `AppViewModel` orchestration can be translated**, not whether its behavior should survive.

---

## Numerical normalization

Claude reports 97,603 lines; Codex reports 97,357. The difference is exactly 246—one per source file—and comes from line-count convention around terminal newlines. It is not a source-scope disagreement.

| Area | Claude | Codex | Interpretation |
|---|---:|---:|---|
| Production files | 246 | 246 | Exact agreement |
| Tests | 21 files / 270 tests | 21 files / 270 tests | Exact agreement |
| Pure logic | 44 files | 29 files | Claude uses a broader definition |
| I/O orchestration | 36 files | 35 files | Near agreement |
| AppViewModel/mixed orchestration | 44 files | 59 files | Primary classification disagreement |
| Platform-specific | 13 files | 12 files | Near agreement |
| UI | 109 files | 102 files | Codex marks seven behavior-bearing UI files as mixed |
| API | Folded into other buckets | 8-file separate bucket | Labeling difference |
| Legacy | Folded into I/O | 1-file separate bucket | Real disposition difference |

Claude’s report does not contain its promised 246-row file classification. Therefore, an exact file-by-file diff cannot yet be calculated. This comparison covers every aggregate claim and every file or subsystem Claude explicitly identifies.

One correction: Claude calls `RemoteAPIServer*.swift` six files, but the 5,652-line figure includes **eight**:

1. `RemoteAPIServer.swift`
2. `RemoteAPIServer+ComponentRoutes.swift`
3. `RemoteAPIServer+HTTP.swift`
4. `RemoteAPIServer+Settings.swift`
5. `RemoteAPIServer+UserRoutes.swift`
6. `RemoteAPIServer+WebSocket.swift`
7. `RemoteAPIServerDTOs.swift`
8. `RemoteAPIServerSupport.swift`

The line total and architectural conclusion remain correct.

---

## Disagreement 1: `AppViewModel` extensions

### Claude

The extensions are domain services using `AppViewModel` mostly as a namespace. Low density of references to `@Published` properties suggests the orchestration can be translated directly rather than re-derived.

### Codex

The extensions contain valuable translatable behavior, but they are mixed because their methods depend on implicit mutable state, managers, selected-server state, timers, callbacks, presentation flags, and other `AppViewModel` methods. Counting only direct references to published properties understates that coupling.

Examples:

- `AppViewModel+Backups.swift` touches little published state, but coordinates process commands, console waiters, active worlds, server-running state, ZIP operations, metadata, pruning, restore, and UI-visible completion.
- `AppViewModel+ServerControls.swift` combines lifecycle, command dispatch, timers, restart policy, backup scheduling, player moderation, and first-run presentation.
- `AppViewModel+OutputHandling.swift` combines valuable parsers with mutation of player, metric, lifecycle, notification, and diagnostic state.
- `AppViewModel+HealthCards.swift` combines diagnostic policy with filesystem, process, VM, network, and UI action decisions.

### Resolution

Claude is right that these files contain real service behavior and should not be discarded. Codex is right that they should not be treated as clean modules merely because published-property references are sparse.

The shared disposition is:

1. Characterize behavior.
2. Identify each method’s explicit inputs, outputs, state reads, state writes, and side effects.
3. Translate domain and orchestration behavior into Rust services.
4. Replace implicit `AppViewModel` dependencies with repositories, operation contexts, event sinks, and platform traits.
5. Delete the Swift extension only after parity is demonstrated.

This is “translate behavior after dependency extraction,” not “redesign from memory” and not “mechanically transliterate the extension.”

---

## Disagreement 2: UI count

Claude classifies 109 files as UI; Codex classifies 102 as UI and 7 additional files as mixed.

The difference likely includes files such as:

- `AddServerWizardView.swift`
- `JavaInstaller.swift`
- `PrerequisitesView.swift`
- `ServerSettingsView.swift`
- `SetupWizardView.swift`
- `OverviewChatCardView.swift`
- Other views containing validation, parsing, process checks, or filesystem staging

### Resolution

The SwiftUI presentation is replaced by Svelte/Tauri. Before deletion, mixed UI files receive symbol-level inspection so that embedded parsers, validation rules, and workflows are moved to the agent or a shared frontend schema.

`OverviewChatCardView.swift` is a concrete example: the card is disposable SwiftUI, but `ChatFeedParser` is engine knowledge.

---

## Disagreement 3: `WorldSlotManager.swift`

### Claude

Classifies it as portable I/O orchestration because only a small portion uses AppKit.

### Codex

Classifies it as mixed, explicitly noting that it is mostly portable world logic and should be split from AppKit thumbnail behavior.

### Resolution

There is no substantive architectural disagreement.

- Port slot metadata, activation, import, export, copy, duplicate, inference, and filesystem behavior.
- Separate thumbnail decoding/resizing into a portable image service or frontend.
- Do not let the `import AppKit` line cause the world subsystem to be treated as macOS-only.

---

## Disagreement 4: legacy `BedrockServerBackend.swift`

### Claude

Classifies the Docker Bedrock backend as portable I/O orchestration.

### Codex

Classifies it as legacy and recommends not porting it unless compatibility evidence requires it.

### Resolution

Codex’s disposition is safer. The repository documentation identifies this backend as legacy and retained for reference, while `VMBedrockServerBackend` is active.

If MSC 2 needs an Ubuntu compatibility-container backend on Debian, it should be designed against the new `BedrockRuntime` contract. The old Docker implementation may provide behavioral clues, but it should not automatically become production Rust scope.

---

## Disagreement 5: Remote API files

### Claude

Says the files are not code-port targets; they become OpenAPI.

### Codex

Places them in an API/wire-contract bucket and says observable behavior is normative while internal implementation can be replaced by a mature Rust HTTP stack.

### Resolution

This is terminology, not architecture.

- Do not translate the hand-written socket server line by line.
- Extract OpenAPI plus explicit WebSocket schemas.
- Preserve authentication, permissions, rate limiting, body limits, routing semantics, DTO defaults, and iOS compatibility.
- Implement those contracts anew in Rust.

The seven `AppViewModel+APIWiring*.swift` files are glue and can disappear after their mappings and edge behavior are captured.

---

## Disagreement 6: router subsystem

### Claude

Treats approximately 6,472 lines of `RouterPortForward*` as a JSON data migration rather than code.

### Codex

Splits the same 14 files into:

| Kind | Files | Lines |
|---|---:|---:|
| Pure rules/models | 7 | 3,861 |
| UI | 5 | 2,183 |
| Catalog I/O | 1 | 137 |
| Mixed runtime resolution | 1 | 291 |

### Resolution

The guide catalog, router records, troubleshooting text, and static step content should move to JSON.

The matcher, fallback resolver, composer, validator/maintenance logic, and troubleshooting rule engine still require executable behavior. They can be translated to Rust or represented through a deliberately designed declarative rule interpreter, but they do not become JSON merely because their inputs are mostly static.

The UI reader and picker become Svelte.

---

## Disagreement 7: test readiness

### Claude

Emphasizes that a strong 270-test corpus already exists and should be mechanically converted to external fixtures.

### Codex

Agrees, but emphasizes that the highest-risk destructive workflows are underrepresented.

### Resolution

Wave 0 should do both:

1. Extract all reusable inline expectations into language-neutral fixtures.
2. Add missing high-risk fixtures for:
   - World slot mutations and rollback
   - Online backup coordination and failed restore
   - Process termination and agent restart
   - Real Bedrock LevelDB/WAL data
   - Real modpack archives and interrupted installs
   - API route-wide authorization/validation
   - Historical config migrations
   - Cross-platform paths and symlink traversal

The existing corpus is a strong beginning, not sufficient proof for destructive parity.

---

## Disagreement 8: estimated port size

### Claude

Estimates approximately 31,500 lines of genuine logic to translate after excluding UI, API implementation, wiring, and router data.

### Codex

Avoids a translation-line estimate because MSC 2 also requires important code that does not currently exist:

- Linux and Windows service integration
- Cross-platform process ownership
- Operation journaling and recovery
- Long-running operation progress/cancellation
- Safe remote file streaming
- OpenAPI/WebSocket generation
- Cross-platform secret stores
- Update installation
- Compatibility-container and native Bedrock runtimes
- Tauri/web client state and reconnect behavior

### Resolution

Use 31,500 lines as a useful indicator that the preserved behavioral corpus is finite. Do not use it as an effort or schedule estimate. Translation size and new-platform engineering are different quantities.

---

## Port-order comparison

Both audits agree on:

1. Contract and fixtures first
2. Java lifecycle as the first working vertical slice
3. Worlds/backups early because of data risk
4. Components/modpacks after core lifecycle and storage
5. Integrations later
6. Linux-native Bedrock before macOS sidecar
7. Tauri UI after the agent contract is proven

The main ordering difference is configuration:

- Claude puts the first Java lifecycle slice before full configuration.
- Codex puts filesystem/configuration/security substrate before lifecycle.

### Combined order

1. **Behavioral evidence**
   - OpenAPI and WebSocket schemas
   - External fixtures from existing tests
   - New destructive-workflow fixtures
   - VZ sidecar IPC contract

2. **Rust domain and safety substrate**
   - Domain types and pure parsers
   - Approved roots and path safety
   - Minimal versioned config repository
   - Secret-store trait
   - Audit log
   - Operation model

3. **Paper lifecycle vertical slice**
   - Import one existing server
   - Start, console, command, status, metrics, stop, restart
   - API, CLI, and existing iOS client
   - macOS and Linux process adapters

4. **Complete configuration and migration**
   - Historical MSC config corpus
   - Settings schema
   - Corruption recovery
   - MSC 1 coexistence/import behavior

5. **Worlds and backups**

6. **All Java families and provisioning**

7. **Mods, plugins, modpacks, updates, and diagnostics**

8. **Networking and helper integrations**

9. **Bedrock runtime family**
   - Linux native
   - Windows native
   - Optional Linux compatibility container
   - macOS VZ Swift sidecar

10. **Tauri/web interface**

Windows agent CI and platform abstractions should begin with the process/config substrate even if full Windows product support arrives later. Deferring all Windows validation until the GUI phase risks discovering path and service assumptions too late.

---

## Final joint recommendation

The audits now support a stronger statement than either original position:

> MSC 2 should use Rust as its cross-platform service foundation, while MSC 1 remains the executable specification and compatibility oracle.

Do not begin by recreating the GUI or porting every manager.

Begin by freeing existing behavior from Swift-only test literals and implicit `AppViewModel` state. Then prove one Paper server can be controlled by the Rust agent through the existing API shape. Every later domain should cross the boundary only when its fixtures pass and its rollback behavior is explicit.

## Missing artifact needed for an exact diff

Claude should export its classification as:

```text
file,bucket,difficulty,confidence,recommended_action
```

with exactly 246 data rows. Once available, it can be joined directly against `msc2-codex-file-inventory.csv` to produce:

- Exact agreement count
- Exact disagreement count
- File-level bucket confusion matrix
- Disagreements sorted by LOC and risk
- A final adjudicated disposition for every file

