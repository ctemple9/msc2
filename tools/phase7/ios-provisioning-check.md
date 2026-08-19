# Phase 7 iOS provisioning check

Purpose: point the copied iOS client's create-server, version, and health
screens at the real MSC 2 agent (P7.23/P7.24's routes) and fix whatever
actually differs from what those screens assumed.

## What this check actually is, and isn't

This was run by an agent with no interactive simulator/device control — no
way to tap through screens or take a live pairing walkthrough. What it
**is**:

1. A field-by-field comparison of every DTO `RemoteAPIModels.swift` already
   declares for these routes against the real Rust DTOs P7.23/P7.24 built,
   and of every call site in `RemoteAPIClient.swift`/`DashboardViewModel.swift`/
   `ServerVersionView.swift`/`HealthView.swift`/`DashboardView.swift` against
   the real route behavior.
2. Real fixes for every mismatch found (below).
3. A real `xcodebuild` compile of the whole `MSCRemoteiOS` target and its
   test target against the fixed code, on this machine:
   - `xcodebuild -project clients/ios/MSCRemoteiOS.xcodeproj -scheme MSCRemoteiOS -destination 'id=AD2970D3-53DE-40FB-93AC-4107DE04CF87' build` → **BUILD SUCCEEDED**
   - same command with `build-for-testing` → **TEST BUILD SUCCEEDED**
   (destination id is this machine's "iPhone 17 Pro" simulator; any
   available iOS Simulator destination works — `xcodebuild ... -showdestinations`
   lists what's registered.)

What it is **not**: a live pairing/create/version/health walkthrough against
a running agent, and not a check that the app behaves correctly once a human
actually taps through it. **Cameron still needs to run that part** — see
"What's still open" below.

## Preconditions for the still-open manual check

1. Build and launch the iOS app: `xcodebuild -project clients/ios/MSCRemoteiOS.xcodeproj -scheme MSCRemoteiOS build`, or open the project in Xcode and Run.
2. Start a Phase 7 agent (`msc serve`) and pair the app with a valid bearer token.

## Bugs found and fixed (real, not hypothetical — each confirmed against the actual Rust route this phase built)

1. **`ServerCreateResultDTO` had no `operationId` field.** P7.9's own
   contract correction means `POST /v1/servers/create` returns almost
   immediately with an `operationId`, not a finished server — the real
   creation (jar download, or for Forge/NeoForge a real supervised
   installer run) happens after the response. Without the field, Codable
   silently dropped it, and `DashboardViewModel.createFreshServer`
   treated the immediate `success: true` as "the server now exists" and
   refreshed the server list right away — a real capability (progress/
   failure reporting for a multi-minute install) quietly missing, exactly
   what D-023 exists to catch. Fixed: added the field, and
   `createFreshServer` now polls the operation via the already-existing
   `pollOperationToTerminal(id:)` (the same helper Phase 6's world/backup
   screens use) before refreshing state, surfacing a real operation
   failure/cancellation instead of a false success.

2. **`VersionChangeResultDTO` had the same missing-`operationId` gap**, plus
   a second, independent bug: `ServerVersionView.applySelected()` branched
   on `result.message` for known failure codes (`"server_running"`,
   `"download_in_progress"`, `"no_active_server"`, `"backup_failed"`) —
   this assumed MSC 1's own local-call semantics, where a refusal came back
   as `success: false` in an ordinary 200 response. The frozen Phase 7
   contract instead returns those as typed HTTP error statuses (409/429),
   which `RemoteAPIClient` throws as `RemoteAPIError.httpStatus` —
   `result.message` was never going to contain those code strings, so
   every one of those branches was dead, and the fallback ("Inconclusive —
   check Components...") fired for a plain "stop the server first" case.
   Fixed: added `operationId`; added a `code` to
   `RemoteAPIError.httpStatus` (previously discarded by
   `bestEffortErrorMessage`, which only ever kept the human message) and a
   `RemoteAPIError.apiErrorCode` accessor; reworked
   `DashboardViewModel.changeVersion` to return a `VersionChangeOutcome`
   enum (`.succeeded`/`.refused(code:message:)`/`.inconclusive`) that
   polls the real operation on success and carries the real error code on
   refusal; updated `ServerVersionView.applySelected()` to switch on that
   code instead of the dead `result.message` pattern.

3. **Health card/overall severity string mismatch.** `HealthView.swift`'s
   `severityColor(_:)` switches on the literal strings `"red"`/`"yellow"`/
   `"green"` (default → neutral color, which also covers `"gray"`) — this
   was already the iOS side's real, shipped assumption. `openapi.json`
   pins no enum for `HealthCardDTO.severity`/`HealthResponseDTO
   .overallSeverity`, so this route's first draft was free to choose its
   own vocabulary, and initially chose `ok`/`warning`/`critical`/`unknown`
   — every card would have silently rendered in the neutral fallback
   color regardless of real severity. Fixed on the Rust side
   (`crates/msc-agent/src/routes/health.rs`) to emit
   `green`/`yellow`/`red`/`gray`, matching the real iOS switch.

## What's still open (not a P7.26 gap — genuinely no existing screen)

- **Templates** (`GET`/`POST /v1/templates`): the copied iOS client has no
  Templates screen at all — not one of the four screens this phase's own
  scope names (`RemoteAPIClient.swift`, `ServerVersionView.swift`,
  `HealthView.swift`, `DashboardView.swift` — Templates isn't among them).
  Building a new screen is out of this step's scope; `msc template list/
  export/create` (P7.25) is the only client surface for this route today.
- **`POST /v1/java-runtimes/install`**: a new route with no MSC 1 UI
  equivalent (`JavaInstaller.swift`'s own `.pkg`-download sheet is
  macOS-only and out of scope per this phase's own decision record). No
  iOS screen calls it. `msc java install <major>` (P7.25) is the only
  client surface.
- **Bedrock refusal UI**: `POST /v1/servers/create`'s `capability_unavailable`
  409 surfaces through `DashboardViewModel.createFreshServer`'s generic
  `catch` block as `error.localizedDescription` (the real server message,
  e.g. "HTTP 409: Bedrock server creation is not available until Phase
  10.") — nothing is hidden, but it isn't given bespoke copy the way the
  known version-change codes now are. Left as-is; a cosmetic improvement,
  not a correctness gap.
- **The interactive walkthrough itself**: pairing, actually tapping
  through create/version/health/repair against a live agent. See
  "Preconditions" above.

## Record

When the manual walkthrough runs, add a short result note to
`docs/msc2/rolling-plan.md` under P7.26:

- device or simulator used
- whether pair, create (both a download-and-go and, if feasible, a Forge/
  NeoForge install-step family), version change, health cards, and health
  repair all passed
- any bug found, with the exact screen and action that exposed it
