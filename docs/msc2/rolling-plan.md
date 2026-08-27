# MSC 2 — Rolling Plan

> ## STATUS: Phase 11 (desktop/web clients) is in progress; **Phase 12 (client redesign) is now planned** below. Phase 11 shipped a working client wired to the real agent, but its UI diverged from MSC 1's information architecture and design language — Phase 12 rebuilds the presentation layer to MSC 1 fidelity, *refreshed*. Terminal UI moved to Phase 13. Phase 12's design system (S0) and shell (S1) were shaped and locked as reference specimens in `docs/msc2/renderings/`, governed by `docs/msc2/antiAIslop.md` (hard rule #11).
> **Next move:** P12.2 (Overview tab) is DONE — Cameron verified it 2026-08-25. P12.2b–j (Java player-data NBT backend, built with Codex) all landed and are marked DONE. P12.3/P12.3a/P12.3b/P12.3c/P12.3d/P12.3e (Players tab, session-log backend, Bedrock identify/skin fixes, real Bedrock stats/inventory) are all DONE — Cameron verified them. P12.3f (session-log client swap) and P12.3g (live-review polish) are built and **awaiting Cameron's verification**. P12.4 (Worlds tab + world wizards) is built and **awaiting Cameron's verification**. **P12.4a–e (2026-08-26) gave P12.4's five found gaps real backends and are all DONE** — Cameron verified them: P12.4a exposes Chunker's already-real `supported_formats` over HTTP, P12.4b wires the already-built `set_slot_thumbnail`/`save_thumbnail` application code to a route, P12.4c lets `POST /v1/worlds/import` redeem an already-on-disk backup, and P12.4d/e port and wire the one genuinely new feature, real Bedrock world repair. **P12.4f (2026-08-26) connects all five to their frontends and is built, awaiting Cameron's verification** — see its own entry below for a real contract/code drift it found and fixed (repair's response schema) and a pre-existing tooling inconsistency it found and deliberately left for a separate decision (`generate.ts`'s quote style vs. the rest of the repo's). See this file's 2026-08-25 notes below for the fuller history of gaps found and how each was handled. **P12.5 and P12.6 (Packs tab removal, Performance tab) are DONE** — Cameron verified them. **P12.7 (Components tab + plugin browser) is built (2026-08-26), awaiting Cameron's verification** — see its own entry below for a small additive `ScreenApi.upload` interface fix it needed and the real pre-existing backend/contract gaps it found and left alone. **P12.7a (2026-08-26) is DONE** — after seeing the real plugin browser, Cameron flagged blank authors/`0 downloads`/no icons; traced to a real two-layer backend bug (`ModrinthSearchHit` never captured those fields, and the route hardcoded them to empty anyway) and fixed both, plus the client's missing icon render. **P12.7b (2026-08-26) is DONE** — icons still didn't show after P12.7a's rebuild; the real cause was one layer deeper, a `#[serde(rename_all = "camelCase")]` acronym-casing bug turning `icon_url` into wire key `iconUrl` instead of the contract's `iconURL`, silently unread by the client. Fixed with explicit `#[serde(rename = ...)]` overrides on all three affected DTO fields plus regression tests. **P12.7c and P12.7d close the one documented gap P12.7/P12.7a left open** — MSC 1's `ModrinthProjectDetailView` (gallery, full About text, per-version compatibility + install) has no backing route. Split into two steps so Codex and Claude Code could work them independently: **P12.7c (2026-08-26) is DONE** — Codex added the contract + Rust backend (two new `GET /v1/catalog/projects/:projectId[/versions]` routes, an additive `versionId` field on the existing install request). **P12.7d (2026-08-26) is built, awaiting Cameron's verification** — the client detail sheet; found along the way that `ServerDTO` carries no Minecraft-version field, so the compatibility banner/badges thread it through from `GET /v1/components`'s existing `primaryComponent.installedVersion` instead (see its own entry for the full finding). **P12.7e (2026-08-26) is DONE** — installing a plugin through the (now-working) browser surfaced a real, pre-existing Phase 8 gap: installed add-ons never got a `currentVersion`, always reading "Unknown version," even though both values the oracle's fallback chain needs were already computed elsewhere in the codebase and simply never threaded through. **P12.7f (2026-08-26) is built, awaiting Cameron's verification** — his own UX preference, not an oracle port: the plugin row's persistent Toggle + Remove button is replaced with a click-anywhere action menu (Enable/Disable, View, Uninstall), with "View" opening `ProjectDetailSheet` for an already-installed add-on via a new, smaller `ProjectDetailItem` type instead of a contract change. **P12.9 (2026-08-27) is built, awaiting Cameron's verification** — hit the same "frozen contract, no backend" gap P12.3 hit for Players: `GET /v1/files`/`GET /v1/files/read` had DTOs in the contract but zero implementation anywhere. Cameron chose to build the backend inline rather than split into precursor steps (see its own entry for why); the step also adds a `PlatformAdapter.revealInFileManager` seam (new Tauri command, browser fallback) for Show in Finder, and explicitly leaves edit/save out of scope (no write route exists in the contract at all). Per this worktree's parallel-execution instructions, `App.svelte`'s section registry was left untouched (an explicit off-limits shared file) — the `files` entry still needs adding there before the tab is reachable; the exact snippet is in P12.9's own entry. **P12.10 (2026-08-27) is built, awaiting Cameron's verification** — see its own entry below: the real docked console lives in `ConsoleDock.svelte`, not the `Files:` line's legacy `src/lib/sections/console/` folder, and needed small additive `api`-prop wiring through `ApplicationShell.svelte`/`App.svelte` (cleared with Cameron first, since both are shared-foundation files) to reach the agent at all. **P12.4k (2026-08-27) is built, awaiting Cameron's verification** — brings `ServerEditorWorldTab.swift`'s Import ZIP / Replace World / Duplicate Slot into the Worlds tab per its own design reversal above; Import ZIP and Replace World are global header-row actions (they don't act on a selected card), while Duplicate is an inline per-card confirm (the backend names the copy "{name} copy" and takes no name argument, so no name-entry sheet was needed). Replace World's `newLevelName` has no client-readable source for Java (`GET /v1/settings` only exposes `level-name` as a field for Bedrock) — the sheet reads it back from settings when available and otherwise defaults to Minecraft's own "world" folder name, matching the oracle's own "read current value, pass it back unchanged" behavior as closely as the existing contract allows without a contract amendment. **P12.12 (2026-08-27) is built, awaiting Cameron's verification** — the Server Editor sheet (General, Broadcast), reached via a new `Edit…` action in `ManageSheet.svelte`. Found a real, systemic contract characteristic while building it: every route this tab touches except rename/eula/delete (`/v1/config/ram`, all of `/v1/broadcast/*`, `/v1/playit*`, `/v1/duckdns`, `/v1/resourcepacks*`) has no `serverId` parameter at all — each one resolves a single agent-wide "active server" server-side, so editing a card that isn't currently active would silently read/write the *wrong* server. Memory and the whole Broadcast tab are gated on a fresh `GET /v1/status` check and show a plain "Set as Active" affordance (reusing `ManageSheet`'s own `setActive`) instead of mutating blind. Also found several `ServerEditorGeneralTab.swift`/`ServerEditorBroadcastTab.swift` fields with no backing route anywhere in the contract despite the domain model already carrying them (`crates/msc-domain/src/app_config_schema.rs`'s `notes`, `auto_restart_on_crash`, `notification_prefs`) — left out rather than faked: Notes, Automation (auto-restart-on-crash), the four per-server notification toggles, the headless-script generator, Server Directory editing, Broadcast's IP Mode picker/host preview, and Reset Xbox Sign-In. Alt Account Profile's email/gamertag/password (`/v1/broadcast/credentials`) is POST-only in the contract, so those fields render blank every time the tab opens rather than pretending to show a stored value. Settings/JARs/Backups/World are confirmed absent as Editor tabs, matching this step's own design decision above.
> **P12.3 blocked on missing backend (decided 2026-08-25):** before rebuilding the Players tab, Cameron flagged that MSC 1's Players tab includes a read-only Java player inventory/stats viewer (`PlayerNBTReader.swift` + `PlayerInventoryView.swift`, hosted in `PlayerProfileDetailSheet.swift`) that never made it past the file-inventory audit into an actual phase step — no domain crate, no API route, and P12.3's own `What:` line never mentioned it. Investigation found `GET /v1/players/profiles` is **already frozen in the API contract** (`docs/msc2/api-contract/openapi.json`: `PlayerProfileDTO`/`PlayerStatsDTO`/`InventoryItemDTO`, plus `POST /v1/players/hidden`, `POST /v1/players/skin-override`, `GET /v1/players/{profileId}/skin`) but has **no handler at all** — today `GET /v1/players` only serves Bedrock (`crates/msc-agent/src/routes/bedrock.rs`; a Java server gets `note: "not_bedrock"`, empty list). This is a straight port against an already-frozen contract, not new API design. Cameron chose to block P12.3 and build the backend first (steps P12.2b–P12.2j below) rather than ship Players tab without it. Online Now / Seen This Session / Session Log are unaffected — those are console-derived (already built in P11.11) and stay in P12.3 itself. **Mutation actions, decided 2026-08-25:** of MSC 1's 5 player-data mutation actions (migrate to offline UUID, migrate to manual UUID, copy, duplicate, delete), none were in the frozen contract. Cameron chose **4 of the 5** — delete, migrate-to-offline-UUID, migrate-to-custom-UUID, and duplicate — added as new steps P12.2g (contract amendment) through P12.2j (route wiring), fully specified (exact DTO field names/types, exact error codes, pinned known-answer test vectors for the offline-UUID algorithm) since Cameron is running these with Codex. `copyPlayerData` (overwrite one player's data onto another's) is the one action still **deferred, not dropped** — add it later as its own contract-amendment step when wanted.
> **Phase 11 → 12 sequencing (decided 2026-08-25):** the committed P11.28g–j agent work is done and carries forward as Phase 12's foundation. The two unfinished Phase 11 steps — P11.28k and the P11.29 gate — are **superseded and folded into P12.17**, because they verify the first-launch UI and MSC 1 fidelity that only the redesign delivers; the whole client gate now runs once against the redesigned client. Phase 12 begins now.
> **Java tab decision (decided 2026-08-27):** reopens P12.12's closed scope. Reviewing the oracle showed MSC 1's Java executable path/Detect/Install Java… (`PreferencesJavaSection` in `MSCSettingsSections.swift`, reused by the Setup Wizard) lives in the app-wide `MSCSettingsView` Preferences window, editing one global `AppConfig.javaPath` used by every Java server — never a per-server screen. Cameron chose to surface that same global value in Server Editor instead, as a new third **Java** tab shown only for Java servers, because that's the moment it's actually needed — not a per-server value, just a per-server-relevant place to edit it. Added as **P12.12a** (built) and, after Cameron's visual review found three real gaps/fixes, corrected by **P12.12b** (adds Extra JVM flags, drops the host-wide banner, fixes per-tab sheet resizing) — both **awaiting Cameron's verification**. P12.14's future MSC Settings scope is narrowed accordingly (see its own entry) so the same global state isn't drafted from two independent screens.
> **Last updated:** 2026-08-27 (P12.9, P12.10, P12.4k, P12.12, P12.12a, P12.12b)

**Previous phases (Setup through Phase 10) and their amendments have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status and active work stay here.

---

## How this document works

This is the **working state** of the build. The vision documents say where we're going; the port plan says in what order; this file says **where we actually are right now**.

Phases are fixed and come from `msc2-port-plan.md`. **Steps are written one phase at a time**, as we reach each phase — not up front. Phase 11 is active; P11.1 now awaits Cameron's verification.

Each phase runs the six-move loop in `CLAUDE.md`: Plan → Read → Execute → Verify → Review → Advance.

### Step format

Every step looks like this:

```
### P0.3 — Extract TPS parser fixtures
**Status:** not started | in progress | awaiting verification | DONE
**Files:** fixtures/tps/, tools/extract-fixtures/
**What:** Pull the 27 TPS test cases out of MSC 1's TpsMonitoringTests.swift
         into input/expected JSON pairs.
**Verify:** `ls fixtures/tps/*.json | wc -l` → 27
**Commit:** P0.3: extract TPS parser fixtures        <- the message, not a hash
```

Every step also carries a **Batch:** field, telling an agent whether it may be run unattended:

| Batch value | Meaning |
|---|---|
| `safe` | Mechanical, and its Verify is a script Cameron has already reviewed. Batch freely. |
| `stop-after` | Runnable in a batch, but the batch **ends here** — the result needs looking at before continuing. |
| `solo` | Judgment work or a new checker script. Run it alone. Its output needs a cross-check by the other agent before the phase closes. |

**Status is only moved to DONE by Cameron**, after he runs the Verify command himself. An agent may set it to *awaiting verification* and stop.

**A step whose Verify only counts things is `stop-after` at best.** Counting proves something exists, not that it is right.

---

## Phases

Gates are in `msc2-port-plan.md`. This is the map, not the detail.

| Phase | Name | State |
|---|---|---|
| **Setup** | Repo, docs, agent instructions, CI, editor config | complete |
| **0** | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| **6** | Worlds and backups | complete |
| **7** | Server families and provisioning | complete |
| **8** | Mods, plugins, modpacks | complete |
| **9** | Networking and helpers | complete |
| **10** | Bedrock runtimes | complete |
| **11** | Desktop and web clients | agent layer done (P11.28g–j); UI verification (P11.28k, P11.29) folded into Phase 12 |
| **12** | Client redesign (MSC 1 fidelity, refreshed) | **in progress — P12.2b–j, P12.3/a–e, P12.4a–j, P12.5, P12.6, P12.7a, and P12.7b DONE; P12.3f/g, P12.4, P12.7, P12.9, and P12.12 built, awaiting Cameron's verification** |
| 13 | Terminal UI (deferred from v1) | not started |

---

## Phase 11 — Desktop and web clients

**Gate** (`msc2-port-plan.md` §3): one Svelte frontend is delivered through
both the Tauri desktop shell and the agent-served browser UI, built against
the proven API while preserving MSC 1's information architecture and design
language. `GET /v1/help/{helpId}` resolves the embedded handbook, concept, and
router-guide content, and clients consume that contract rather than carrying
their own divergent teaching text. UI completion never gates headless agent
correctness.

**Working exit criteria:** macOS, Windows, and Linux desktop builds and the
browser load the same generated frontend bundle and expose every Phase
11-owned capability that the connected agent advertises and the calling token
permits. All client state is host-keyed, the current host and active server are
always visible, reconnecting restores bounded console and operation state, and
dangerous actions remain explicit. TypeScript DTOs are generated from
`docs/msc2/api-contract/openapi.json`; no handwritten mirror DTOs are allowed.
The agent serves the same static bundle plus embedded educational content, and
the Desktop/Web column of `client-capability-matrix.csv` is updated with every
surface rather than filled in retrospectively. Browser cookie auth and Tauri
local/remote auth close Phase 11's remaining D-012 scope without weakening
Phase 9's loopback-by-default, explicit-Tailscale-only posture. Linux proof
must launch the real Tauri binary through WebKitGTK under a display server and
interact with it; a Chromium-only browser run is not Linux desktop evidence.
The gate also requires fresh-profile evidence for MSC 1's first-launch
experience: setup sheet, Concept Guide, Handbook handoff, guided tour ordering
and anchors, skip/reopen state, splash playback or fallback, reduced-motion
behavior, and the boundary between agent-owned first-server initiation and
client-owned presentation.

**Extensibility boundary:** navigation and routing are a registry of section
descriptors keyed by stable strings and capability predicates, not a closed
enum, exhaustive switch, or fixed tab array. The route families
`/hosts/:hostId/servers/:serverId/bedrock/*` and
`/hosts/:hostId/servers/:serverId/profiles/*` are reserved without shipping
either screen. A later Bedrock client group registers its sections only when
`GET /v1/capabilities` advertises a usable Bedrock backend; it never infers support
from the host OS or client build. A later player-profiles phase must first port
the ledgered agent workflows (profile loads, Mojang/Floodgate resolution,
manual Bedrock identification, UUID migration/data mutation, hidden profiles,
and skin storage/serving), extend the public contract and capability response,
regenerate TypeScript, then register its section. Phase 11's registry,
host/server-scoped route parameters, lazy section loading, permission filters,
and unknown-capability tolerance are the seam that later phase consumes; none
of that work should require replacing Phase 11 navigation.

**Execution order:** no Phase 11 step is blocked on ownership of `crates/`,
the shared contract, CI, or packaging. P11.1–P11.19 build and prove the common
client foundation and non-Bedrock surfaces. P11.20 consumes the completed
Bedrock capability/runtime-state contract to prove the extension seam without
shipping Bedrock screens.
P11.21–P11.29 may then change shared agent/auth/packaging code and close the
gate. Step-to-step dependencies still apply, and a batch must never continue
after a failed Verify.

**Owner choices confirmed during PLAN (2026-08-22):** general-LAN management
remains unavailable for v1. Browser management stays on loopback or an
explicitly configured Tailscale path, and Tailscale never replaces
authentication or permission checks; Phase 11 does not build a local
certificate authority or ask users to bypass browser certificate warnings.
On macOS and Windows, MSC may download and verify a coordinated desktop,
agent, and compatible sidecar update, but it asks before installation; it does
not silently install automatically. Linux update installation remains owned by
the package manager, with MSC limited to an actionable availability notice.

### Group A — Shared client foundation and non-Bedrock surfaces (in progress)

### P11.1 — Scope the client rebuild from the iOS oracle and capability matrix
**Status:** DONE
**Files:** `docs/msc2/clients/phase11-scope.md`, `docs/msc2/audit/msc2-symbol-ledger.csv`, `tools/phase11/scope-check.py`, `docs/msc2/rolling-plan.md`
**What:** Read all 53 copied iOS client files as the primary behavioral and screen-structure reference, then use the MSC 1 macOS views only for the desktop information architecture and visual language. Map every current OpenAPI/WebSocket operation and matrix row to a Phase 11 screen, shared infrastructure, honest future state, or explicitly out-of-scope agent gap. Record the D-003 same-screen rule, D-013 host scoping, D-023 matrix-update rule, D-026 help ownership, D-021 client resource bounds, and the exact Bedrock/player-profile extensibility handoffs above. Do not assign the player-profile agent rows to Phase 11 merely because their old DTOs remain in the frozen baseline.
**Verify:** `python3 tools/phase11/scope-check.py docs/msc2/clients/phase11-scope.md docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.1: scope desktop and web clients`
**Batch:** solo

### P11.2 — Scaffold one standalone Svelte and Tauri client
**Status:** DONE
**Files:** `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src/`, `clients/desktop-web/static/`, `clients/desktop-web/src-tauri/`, `clients/desktop-web/svelte.config.js`, `clients/desktop-web/vite.config.ts`, `clients/desktop-web/tsconfig.json`
**What:** Create one TypeScript Svelte frontend with a static build suitable for both agent serving and a thin Tauri 2 shell. Keep `src-tauri` standalone from the root Cargo workspace, with its own lockfile and no server-management behavior, so the existing headless Rust workspace does not acquire GUI build dependencies. Establish formatting, type-checking, unit-test, production-build, and bundle-identity commands.
**Verify:** `npm --prefix clients/desktop-web run verify:scaffold`
**Commit:** `P11.2: scaffold shared Svelte client`
**Batch:** solo

### P11.3 — Generate TypeScript from the frozen OpenAPI contract
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/api/generate.ts`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `tools/phase11/generated-types-check.py`
**What:** Generate the HTTP request/response type surface directly from the current frozen `docs/msc2/api-contract/openapi.json`, preserving optional/additive fields needed for D-010 skew, including `BedrockRuntimeStateDTO` even though Phase 11 ships no Bedrock screens. Make regeneration deterministic and fail when checked-in output differs from the contract. Handwritten transport helpers may wrap generated types, but no hand-authored DTO mirror is permitted.
**Verify:** `npm --prefix clients/desktop-web run api:check`
**Commit:** `P11.3: generate TypeScript API types`
**Batch:** solo

### P11.4 — Build the contract-backed client test harness
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/testing/`, `clients/desktop-web/tests/contract/`, `clients/desktop-web/tests/fixtures/`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`
**What:** Add deterministic fake HTTP, WebSocket, upload/download, operation, auth, capability, permission, old-agent/new-agent, and reconnect scenarios using generated DTO shapes. Include unknown optional fields and absent future capability keys so the UI proves additive skew tolerance. This becomes the reviewed test boundary later `safe` screen batches use; it must not invent Bedrock screens or player-profile behavior. The separate P11.20 seam test consumes the real Bedrock capability/runtime-state contract.
**Verify:** `npm --prefix clients/desktop-web run test:contract`
**Commit:** `P11.4: add client contract harness`
**Batch:** solo

### P11.5 — Establish extensible information architecture and routing
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/navigation/`, `clients/desktop-web/src/routes/`, `clients/desktop-web/tests/navigation/`, `docs/msc2/clients/phase11-scope.md`
**What:** Implement the descriptor registry, nested host/server route parameters, permission and capability predicates, lazy component loading, stable deep links, narrow/wide layouts, and unknown-section fallback. Prohibit a closed section enum, exhaustive section switch, fixed tab-count assumptions, and checks such as `hostOs == linux` standing in for capability discovery. Reserve but do not register or render Bedrock and player-profile route families; tests must prove a synthetic future descriptor can be added without editing the shell/router and remains hidden until its named advertised capability is present.
**Verify:** `npm --prefix clients/desktop-web run test:navigation`
**Commit:** `P11.5: add extensible client routing`
**Batch:** solo

### P11.6 — Make all connection and cache state host-scoped
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/hosts/`, `clients/desktop-web/src/lib/stores/`, `clients/desktop-web/tests/hosts/`
**What:** Implement D-013's host registry, minimal host switcher, per-host connection/capability/permission/server/console/operation caches, active-server selection, stale-data isolation, and explicit host identity on every destructive confirmation. Credentials remain behind an injected credential adapter so the browser and Tauri mechanisms can land later without migrating store shapes. No singleton active host, global console buffer, or credential field may leak across hosts.
**Verify:** `npm --prefix clients/desktop-web run test:hosts`
**Commit:** `P11.6: add host-scoped client state`
**Batch:** safe

### P11.7 — Implement the generated HTTP and resilient stream client
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/api/`, `clients/desktop-web/src/lib/streams/`, `clients/desktop-web/src/lib/operations/`, `clients/desktop-web/tests/transport/`
**What:** Build one host-aware transport over generated request/response types, `ErrorDTO`, version headers, capability refresh, bounded staged transfers, and cookie-or-bearer credential adapters. Add console, operation, and notification stream reconnect with bounded history, deduplication, cancellation, terminal-state recovery, and explicit unsupported/old-client states. Keep browser and desktop on the same calls; shell IPC may supply credentials or native services but never an alternative management API.
**Verify:** `npm --prefix clients/desktop-web run test:transport`
**Commit:** `P11.7: build shared API transport`
**Batch:** safe

### P11.8 — Build the responsive MSC design system and application shell
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/components/`, `clients/desktop-web/src/lib/styles/`, `clients/desktop-web/src/routes/+layout.svelte`, `clients/desktop-web/tests/visual/`
**What:** Translate the copied iOS component structure and MSC 1 macOS design language into reusable tokens, cards, tables, forms, dialogs, alerts, empty/loading/error states, keyboard focus, reduced motion, and responsive sidebar/bottom-navigation shells. Preserve desktop's server-list/sidebar and always-available console concepts without baking today's section count into layout. Include the client-owned first-launch/splash seam needed for the setup sheet, Concept Guide, Handbook handoff, guided-tour overlay, and animation/fallback behavior; do not replace that sequence with a generic welcome screen. The shell must visibly name the selected host and server and remain usable at phone, tablet, and desktop widths.
**Verify:** `npm --prefix clients/desktop-web run test:visual-shell`
**Commit:** `P11.8: build shared MSC interface shell`
**Batch:** stop-after

### P11.9 — Build fleet, provisioning, and lifecycle workflows
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/home/`, `clients/desktop-web/src/lib/sections/fleet/`, `clients/desktop-web/tests/screens/fleet.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement status, active-server switching, create/import/rename/delete/EULA, Java family/version/runtime selection and install, templates, start/stop/restart, clear confirmations, capability/permission gates, and durable operation progress. Use the iOS create/import flows as the functional reference and desktop macOS views for hierarchy only. Update each delivered Desktop/Web matrix row in this same step; unsupported or agent-Planned routes stay `Planned`, never implied by a disabled decorative control.
**Verify:** `npm --prefix clients/desktop-web run test:screen-fleet && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.9: add fleet and lifecycle screens`
**Batch:** safe

### P11.10 — Build console, commands, operations, notifications, and performance
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/console/`, `clients/desktop-web/src/lib/sections/performance/`, `clients/desktop-web/src/lib/components/operations/`, `clients/desktop-web/src/lib/components/notifications/`, `clients/desktop-web/tests/screens/live.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement the bounded live console with history/search/filter/pause/copy/clear-local-view, command history/favorites, operation progress/cancel/recovery, notification feed, performance metrics/charts, help affordances, and reconnect behavior. Use DOM/SVG/CSS rendering with a low-cost fallback rather than assuming Chromium-only WebGL/canvas behavior. Update the matching Desktop/Web matrix cells.
**Verify:** `npm --prefix clients/desktop-web run test:screen-live && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.10: add live server screens`
**Batch:** safe

### P11.11 — Build the online roster without claiming player profiles
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/players-online/`, `clients/desktop-web/tests/screens/players-online.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Render only the generic online roster the connected agent actually advertises. Do not build the Bedrock allowlist/permissions UI in this phase. Keep the registered section identity distinct from the reserved future `profiles` route; do not call the frozen-but-unimplemented profile, skin, hidden-profile, session-history, UUID migration, or player-data mutation routes and do not present their matrix cells as implemented. Prove the online section still works when profile capability fields are unknown, absent, or later added.
**Verify:** `npm --prefix clients/desktop-web run test:screen-players-online && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.11: add online player roster`
**Batch:** safe

### P11.12 — Build worlds, backups, and staged transfer workflows
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/worlds/`, `clients/desktop-web/src/lib/sections/backups/`, `clients/desktop-web/src/lib/transfers/`, `clients/desktop-web/tests/screens/worlds-backups.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement slot inventory/activation/create/rename/duplicate/delete/import/export/convert, direct active-world mutations where the API supports them, thumbnails, backup create/config/delete/restore, bounded uploads/downloads, transactional warnings, progress/cancel/recovery, and risk-appropriate confirmations. Update every genuinely delivered Desktop/Web matrix cell and leave unavailable agent paths visible only as truthful capability explanations.
**Verify:** `npm --prefix clients/desktop-web run test:screen-worlds-backups && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.12: add world and backup screens`
**Batch:** safe

### P11.13 — Build add-on, modpack, and component workflows
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/addons/`, `clients/desktop-web/src/lib/sections/components/`, `clients/desktop-web/tests/screens/addons.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement installed add-ons, catalog search, install/update/toggle/remove/source actions, system-component state, client export, modpack inspect/import/replace, and D-027 manual browser-download then bounded staged-upload completion. Preserve provider-unavailable, dependency, pack-managed, cancellation, and provenance explanations. Update the matching Desktop/Web matrix rows; never hardcode provider or server-family lists where the contract supplies them.
**Verify:** `npm --prefix clients/desktop-web run test:screen-addons && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.13: add add-on and modpack screens`
**Batch:** safe

### P11.14 — Build settings, health, networking, helpers, and access administration
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/settings/`, `clients/desktop-web/src/lib/sections/health/`, `clients/desktop-web/src/lib/sections/connectivity/`, `clients/desktop-web/src/lib/sections/access/`, `clients/desktop-web/tests/screens/administration.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Render schema-driven settings without a client-side field enum; implement health cards/problems/repairs, RAM/Java/Geyser, connectivity diagnostics, Playit, DuckDNS, Xbox Broadcast, resource packs, and named-token create/update/revoke with one-time-secret handling. Permission and capability filters must remove unavailable actions while keeping explanations. Agent-Planned files/watchdog/profile routes remain Planned rather than receiving fake screens. Update each delivered matrix row.
**Verify:** `npm --prefix clients/desktop-web run test:screen-administration && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.14: add administration screens`
**Batch:** safe

### terra clean up

#### Review findings

- The original P11.5–P11.8 foundations were not connected to the running
  shell: it selected hard-coded sections and a hard-coded host/server instead
  of loading the token's permissions and the host's advertised capabilities.
- Reconnecting streams changed label but never scheduled a retry, and staged
  downloads had no client-side memory ceiling.
- The P11.11 roster requested `/v1/session-log` and presented a join/leave
  history even though that route is deliberately deferred with player profiles.
- The capability matrix overstated deferred Bedrock/world and shared-context
  routes as delivered. It also marked live WebSocket channels implemented while
  the shell still reads the bounded HTTP snapshots; those channels wait for the
  authenticated wiring in P11.21–P11.23. A rejected world mutation also
  changed local UI state, and destructive confirmations did not name their
  exact host and target.
- The focused tests covered helpers but did not protect all of those integration
  boundaries against regression.

#### Items completed in this cleanup

- Connect the shell to `/v1/capabilities` and `/v1/me`, use the descriptor
  registry to filter routes, and retain host/server identity in the route.
- Give streams bounded automatic reconnect attempts and give staged downloads a
  512 MiB client-memory ceiling.
- Remove the deferred session-history request and restore its matrix row to
  `Planned`.
- Correct the affected capability-matrix claims, preserve state after rejected
  world actions, name host plus target in destructive confirmations, and leave
  live streams honestly `Planned` until their authentication boundary exists.
- Add regression coverage for the stream/download and deferred-route boundaries.

### P11.15 — Extract and validate the educational content corpus
**Status:** DONE
**Files:** `content/help/`, `content/guides/`, `fixtures/help-content/`, `fixtures/onboarding/`, `tools/phase11/help-content-check.py`, `docs/msc2/clients/phase11-scope.md`
**What:** Extract MSC 1's 31-topic handbook, concept guide, router catalog/records/steps, troubleshooting content, and onboarding copy into the confirmed Markdown-with-YAML-front-matter and structured guide data formats. Preserve source citations and label content versus executable router rules; do not duplicate prose in Svelte. Record unresolved diagram assets honestly. Include the Concept Guide page order, onboarding step content/order, skip/reopen wording, and the source mapping for every first-launch explanation; visual anchoring and animation remain client-owned. Include coverage for every `helpId` already emitted by settings, health, diagnostics, performance, connectivity, and errors, including the `bedrock.runtime-unavailable` later-audit requirement. Add deterministic onboarding fixtures proving fresh-install, already-seen, skipped, reopened, and unknown/future-topic cases.
**Verify:** `python3 tools/phase11/help-content-check.py --all`
**Commit:** `P11.15: extract educational content`
**Batch:** solo

### P11.16 — Render contract-served help and guides in the shared client
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/help/`, `clients/desktop-web/src/lib/sections/handbook/`, `clients/desktop-web/tests/screens/help.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Build safe Markdown rendering, related-topic navigation, handbook/concept/router-guide readers, contextual `helpId` links, unknown-topic degradation, and client-owned onboarding anchors against the fake contract. Implement and test the fresh-install sequence of setup sheet → Concept Guide → guided tour, including the exact step ordering, user-action pauses, form-card hide/resume behavior, completion state, skip behavior, Handbook handoff, and reopen-from-preferences behavior. Implement the splash animation seam with a bounded fallback and reduced-motion path; preserve the source asset or record an explicit reviewed replacement. Every explanation comes from a response or structured content fixture, never a screen-local copy. The registry must allow future Bedrock and profile topics/sections to appear additively without new shell logic.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && python3 tools/phase11/help-content-check.py --client`
**Commit:** `P11.16: render shared help content`
**Batch:** safe

### P11.17 — Prove browser parity, accessibility, and responsive layouts
**Status:** DONE
**Files:** `clients/desktop-web/tests/e2e/browser/`, `clients/desktop-web/playwright.config.ts`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`
**What:** Exercise the production static bundle against the contract harness at narrow and wide widths, keyboard-only navigation, reduced motion, destructive confirmations, host switching, reconnect, upload/download, and deep-link reload. Add a fresh-profile onboarding walkthrough proving setup completion, Concept Guide → tour sequencing, step ordering and anchors, user-action pauses, skip/resume/reopen flags, Handbook handoff, splash playback/fallback, and reduced-motion behavior. Run Chromium plus browser WebKit for fast compatibility feedback, while recording plainly that this is browser evidence and does not replace P11.19's native Linux WebKitGTK proof.
**Verify:** `npm --prefix clients/desktop-web run test:e2e-browser`
**Commit:** `P11.17: prove shared browser workflows`
**Batch:** stop-after

### P11.18 — Add the thin Tauri shell without desktop-only screens
**Status:** DONE
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/tauri/`
**What:** Load the exact production Svelte bundle and expose only narrow native adapters for credentials, file pickers, notifications, menus, window lifecycle, and later agent installation/update. Each native affordance must invoke a shared web workflow with a browser fallback; no route or screen may test `isTauri` to reveal desktop-only management behavior. Keep the standalone Tauri crate outside the root workspace so headless Rust builds do not acquire GUI dependencies.
**Verify:** `npm --prefix clients/desktop-web run test:tauri-boundary`
**Commit:** `P11.18: add thin Tauri shell`
**Batch:** solo

### P11.19 — Exercise the real Linux Tauri renderer through WebKitGTK
**Status:** DONE
**Files:** `clients/desktop-web/tests/e2e/tauri-linux/`, `clients/desktop-web/wdio.conf.ts`, `tools/phase11/linux-webkitgtk-smoke.sh`, `docs/msc2/clients/evidence/`
**What:** On a Debian/Ubuntu desktop runner with `libwebkit2gtk-4.1`, `webkit2gtk-driver`, and Xvfb, launch the built Tauri binary and drive its real window through the native WebDriver path. Verify visible shell, navigation, CSS layout, forms, dialogs, live-console fallback, deep links, one mutating fake workflow, and the fresh-profile onboarding entry path including the reduced-motion/fallback branch; record the WebKitGTK package/version and screenshot evidence. A Vite page opened in Chrome or Playwright's bundled WebKit does not satisfy this step.
**Verify:** `bash tools/phase11/linux-webkitgtk-smoke.sh --native`
**Commit:** `P11.19: prove Linux WebKitGTK rendering`
**Batch:** stop-after

### Group B — Bedrock extension seam (ready after Group A; no Bedrock screens)

### P11.20 — Prove capability-driven Bedrock extension seams
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/navigation/`, `clients/desktop-web/tests/navigation/bedrock-extension.test.ts`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/clients/phase11-scope.md`
**What:** Recheck generated TypeScript against the current frozen OpenAPI document and consume its finalized `serverTypes.bedrock` plus `BedrockRuntimeStateDTO` advertisement without hand-written Bedrock DTOs or host-OS inference. Use a test-only future section descriptor to prove Bedrock navigation is absent when unsupported, can be registered when capability state permits it, survives unknown backend/reason values additively, and fits existing layouts/routes without restructuring. Ship no Bedrock section, creation flow, settings, player, allowlist, world, backup, console, or runtime screen in Phase 11; keep those matrix cells Planned for the later Bedrock client group.
**Verify:** `npm --prefix clients/desktop-web run api:check && npm --prefix clients/desktop-web run test:bedrock-extension && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.20: preserve Bedrock client extension seam`
**Batch:** solo

### Group C — Shared agent, auth, packaging, and gate work (ready after Group B)

### P11.21 — Close the remaining desktop and browser authentication design
**Status:** DONE
**Files:** `docs/msc2/clients/phase11-auth.md`, `docs/msc2/msc2-decisions.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/api-contract/auth-scope-phase2.md`, `docs/msc2/lifecycle/pairing-phase4.md`, `crates/msc-api/tests/phase11_auth_conformance.rs`
**What:** Turn D-012's approved mechanisms and Phase 9 posture into one testable contract: same-machine Tauri bootstrap resistant to arbitrary local-process impersonation, per-host remote desktop pairing and secret-store keys, browser pairing-to-httpOnly-SameSite cookie exchange, session revocation/expiry, exact allowed-origin/CSP rules, CSRF tokens for cookie-authenticated mutations, and bearer exemption. Preserve loopback-by-default management and authenticated explicit-Tailscale access; per the owner-confirmed v1 choice above, keep general-LAN management unavailable and do not build certificate provisioning or a local trust system. Use additive versioned routes and `ErrorDTO`; do not put raw credentials in Svelte-accessible storage or URLs.
**Verify:** `python3 tools/api-contract-check.py --v1-summary && cargo nextest run -p msc-api --test phase11_auth_conformance`
**Commit:** `P11.21: freeze desktop and browser auth`
**Batch:** solo

### P11.22 — Implement browser sessions, origin policy, CSP, and CSRF
**Status:** DONE
**Files:** `crates/msc-agent/src/auth/`, `crates/msc-agent/src/routes/browser_session.rs`, `crates/msc-agent/tests/browser_auth.rs`, `clients/desktop-web/src/lib/auth/browser.ts`, `clients/desktop-web/tests/auth/browser.test.ts`
**What:** Implement the frozen browser pairing/session path on the existing credential registry, with one-use challenges, httpOnly cookies, revocation and restart behavior, exact origin checks, restrictive CSP, CSRF on every cookie-authenticated mutation, rate limits, and audit attribution. Prove bearer clients remain unaffected and a hostile origin/local script cannot turn ambient browser authority into a server mutation.
**Verify:** `cargo nextest run -p msc-agent --test browser_auth && npm --prefix clients/desktop-web run test:auth-browser`
**Commit:** `P11.22: add secure browser sessions`
**Batch:** stop-after

### P11.23 — Implement local and remote Tauri credentials per host
**Status:** DONE
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/tests/auth/desktop/`, `crates/msc-agent/src/auth/`, `crates/msc-agent/tests/desktop_auth.rs`
**What:** Implement the chosen same-machine authorization handshake and remote pairing exchange, storing one credential per agent host ID in the platform credential store through the shell so secrets never enter browser storage. Verify local convenience does not become loopback-open authorization, remote credentials obey permission/expiry/revocation, switching hosts cannot reuse another host's credential, and the web build retains its cookie flow with no divergent screen.
**Verify:** `cargo nextest run -p msc-agent --test desktop_auth && npm --prefix clients/desktop-web run test:auth-desktop`
**Commit:** `P11.23: add per-host desktop credentials`
**Batch:** stop-after

### P11.24 — Serve embedded help content and port router-guide rules
**Status:** DONE
**Files:** `crates/msc-domain/src/router_guides.rs`, `crates/msc-domain/tests/router_guides.rs`, `crates/msc-agent/src/help.rs`, `crates/msc-agent/src/routes/help.rs`, `crates/msc-agent/tests/help_routes.rs`, `content/help/`, `content/guides/`, `fixtures/help-content/`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`
**What:** Embed the validated content corpus, implement `GET /v1/help/{helpId}` plus the additive handbook/concept/router-guide catalog routes required to browse it, and port the ledgered router matcher/fallback/composer/troubleshooting rules into Rust against fixtures. Explicitly include the `bedrock.runtime-unavailable` topic and its structured unavailable reasons. Return raw Markdown/structured steps for every client to render; unknown or version-new topics degrade through `ErrorDTO`. Do not absorb client onboarding anchors or presentation into the agent.
**Verify:** `cargo nextest run -p msc-domain --test router_guides && cargo nextest run -p msc-agent --test help_routes && npm --prefix clients/desktop-web run test:screen-help`
**Commit:** `P11.24: serve shared educational content`
**Batch:** solo

### P11.25 — Serve the same production bundle from the agent
**Status:** DONE
**Files:** `crates/msc-agent/src/web_ui.rs`, `crates/msc-agent/tests/web_ui.rs`, `clients/desktop-web/`, `tools/phase11/bundle-identity-check.py`
**What:** Embed or package the exact Svelte production output the Tauri shell loads, serve hashed assets with correct MIME/cache headers and CSP, support safe client-side deep-link fallback without shadowing `/v1`, and provide an explicit unavailable result when a headless package intentionally omits web assets. Prove byte identity between the browser-served and Tauri-loaded bundles and preserve D-011's no-GUI dependency boundary in the agent.
**Verify:** `python3 tools/phase11/bundle-identity-check.py && cargo nextest run -p msc-agent --test web_ui`
**Commit:** `P11.25: serve shared web bundle`
**Batch:** stop-after

### P11.26 — Install and manage the local agent through the shell
**Status:** DONE
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/setup/`, `clients/desktop-web/tests/agent-install/`, `tools/phase11/agent-install-smoke.sh`, `packaging/`
**What:** Add shell-only native commands behind shared setup screens to detect, install, start, stop, repair, and report the platform service and compatible agent/sidecar package. Closing the window must never stop the service or a server. Browser users see the same status/setup route with a truthful instruction/fallback when native install is unavailable; there is no desktop-only screen. Preserve existing service identity, privilege, headless, and rollback rules.
**Verify:** `bash tools/phase11/agent-install-smoke.sh --synthetic`
**Commit:** `P11.26: manage local agent installation`
**Batch:** stop-after

### P11.27 — Define and prove coordinated desktop, agent, and sidecar updates
**Status:** DONE
**Files:** `docs/msc2/clients/phase11-update.md`, `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/updates/`, `packaging/`, `tools/phase11/update-smoke.sh`
**What:** Implement the owner-confirmed prompted, signed macOS/Windows update policy as a compatibility-aware set: download and verify release identity before staging, ask before installation, keep the running agent until replacement is ready, preserve configuration/secrets/worlds, pair the exact compatible Bedrock sidecar where applicable, roll back a failed update, and allow app/agent version skew only within D-010's advertised window. Never install silently. Linux defers installation to its package manager and receives an actionable notice, not a second self-updater. Never merge MSC updates with server/loader/add-on update controls.
**Verify:** `bash tools/phase11/update-smoke.sh --synthetic --all-platforms`
**Commit:** `P11.27: add coordinated client updates`
**Batch:** solo

### P11.28 — Build and exercise desktop and web candidates on all three platforms
**Status:** DONE
**Files:** `.github/workflows/ci.yml`, `tools/phase11/desktop-web-smoke.sh`, `clients/desktop-web/`, `docs/msc2/clients/evidence/`
**What:** Add production frontend/type tests, agent-served browser smoke, and real Tauri builds to macOS, Windows, and Linux CI while preserving the headless no-GUI job. Exercise the same core workflow in browser and desktop modes; on Linux run P11.19's native WebKitGTK smoke, not a Chromium substitute. Record platform renderer/package versions and explicit unavailable signing/notarization evidence without claiming unperformed release distribution.
**Verify:** `bash tools/phase11/desktop-web-smoke.sh --synthetic --all-surfaces`
**Commit:** `P11.28: prove tri-platform clients`
**Batch:** stop-after

### P11.28a — Port the remaining MSC 1 first-time setup pages
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/help/FirstLaunchGate.svelte`, `clients/desktop-web/tests/screens/help.test.ts`, `clients/desktop-web/tests/e2e/browser/`, `clients/desktop-web/tests/e2e/tauri-linux/`, `crates/msc-agent/src/routes/capabilities.rs`, `crates/msc-agent/src/routes/networking.rs`, `crates/msc-agent/src/routes/versions.rs`, `crates/msc-api/src/dto/`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`, `differences.md`
**What:** Port MSC 1’s Server Setup, Playit.gg, Xbox Broadcast, Tailscale, and You’re All Set pages into the existing MSC 2 first-launch flow. Use real agent probes for the servers root, Java 21+, Bedrock runtime, Xbox helper, and Tailscale; keep optional services skippable; persist the selected server types; and hand off explicitly to first-server creation through the existing Concept Guide.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run build && npm --prefix clients/desktop-web run api:generate -- --check && cargo check -p msc-api -p msc-agent && cargo nextest run -p msc-api --test dto_conformance`
**Commit:** `P11.28a: port remaining first-time setup pages`
**Batch:** stop-after

### P11.28b — Repair first-time setup controls and host probes
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/tests/tauri/platform-boundary.test.ts`
**What:** Keep Bedrock selectable when the selected built-in runtime is in `provisioning_required`, open setup links through the operating system’s default browser in Tauri, verify the Xbox helper after download by re-reading the agent’s status and filename, and make Tailscale checks visibly report Checking, installed, not installed, or unavailable.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run build && cargo check -p msc-api -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P11.28b: repair first-time setup controls`
**Batch:** stop-after

### P11.28c — Add native setup pickers and Java verification
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/screens/help.test.ts`, `clients/desktop-web/tests/tauri/platform-boundary.test.ts`, `crates/msc-agent/src/routes/versions.rs`
**What:** Use the native Tauri dialog for the servers-root folder and Java executable Browse actions, retain a manual path fallback in browser mode, and include manually configured Java outside standard search roots in the agent’s real version probe so Check for Java and Use PATH report truthfully.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run build && cargo check -p msc-api -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P11.28c: add native setup pickers and Java verification`
**Batch:** stop-after

### P11.28d — Make Xbox helper verification stateful in the test host
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `clients/desktop-web/playwright.config.ts`
**What:** Keep the deterministic browser/Tauri test host’s Xbox helper status consistent with its download endpoint, return the helper filename, and exercise the setup flow through the verified-download message so a successful fake download cannot be reported as missing. This remains test-only and must not become a production host.
**Verify:** `npm --prefix clients/desktop-web run test:e2e-browser`
**Commit:** `P11.28d: make Xbox helper verification stateful in the test host`
**Batch:** stop-after

### P11.28e — Wire Tailscale checks through every setup surface
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`, `clients/desktop-web/tests/screens/help.test.ts`
**What:** Pass the agent API into the Handbook’s compact first-launch setup, which was previously rendered without it, and make the Tailscale Check button report an unavailable connection instead of silently returning when no API is present.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run build`
**Commit:** `P11.28e: wire Tailscale checks through every setup surface`
**Batch:** stop-after

### P11.28f — Reserve port 48001 for MSC2
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-domain/src/app_config_schema.rs`, `clients/desktop-web/src-tauri/src/lib.rs`, `packaging/agent-service-layout.json`, platform service tests, `docs/msc2/`, `tools/`, `corpus/server-dirs/README.md`
**What:** Move MSC2’s default management/service port from 48400 to 48001 so it can run beside MSC1 without collisions. Keep MSC1 and historical audit/iOS references on 48400, while updating MSC2’s agent defaults, Tauri installer, service metadata, live-test examples, and assertions.
**Verify:** `cargo fmt --all -- --check && cargo test -p msc-infrastructure --test service_model && cargo test -p msc-platform-macos --test service_plist && cargo test -p msc-platform-linux --test systemd_unit && cargo test -p msc-platform-windows --test service_definition && cargo check -p msc-agent -p msc-domain && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml && bash tools/phase11/agent-install-smoke.sh --synthetic`
**Commit:** `P11.28f: reserve port 48001 for MSC2`
**Batch:** stop-after

### P11.28g — Remove fake hosts from the shipped client
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/navigation/`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/`, `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`
**What:** Remove the hard-coded `demo-agent` and fake host switch from the production shell. Keep D-013’s host-keyed architecture for real configured hosts, but make the initial desktop host the actual local agent and keep fake HTTP/contract-host behavior exclusively in test entry points. The shipped app must never present a fake host as a selectable destination.
**Verify:** `npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build && ! rg -n "demo-agent" clients/desktop-web/src`
**Commit:** `P11.28g: remove fake hosts from the shipped client`
**Batch:** stop-after

### P11.28h — Connect the Tauri client to the real local agent
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/api/`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/auth/desktop/`, `clients/desktop-web/tests/tauri/`
**What:** Wire `DesktopSessionAuth` and its native authorized-request bridge into the real `ApiClient` instead of falling back to cookie authentication in Tauri. Use `http://127.0.0.1:48001` as the local agent origin, obtain/store a host-scoped credential through the approved desktop pairing/bootstrap path, and keep browser sessions on the browser cookie adapter. Do not add a shell-token environment-variable shortcut to the shipped client.
**Verify:** `npm --prefix clients/desktop-web run test:auth-desktop && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run build && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P11.28h: connect Tauri to the real local agent`
**Batch:** stop-after

### P11.28i — Start the installed local agent without terminal commands
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/tests/agent-install/`, `clients/desktop-web/tests/tauri/`, `tools/phase11/`
**What:** On desktop launch, inspect the local service through the existing native service adapter. Start an installed-but-stopped MSC2 agent automatically and wait for its health endpoint on `48001`; when the service is not installed, expose one explicit Install action that uses the existing packaged-agent/service path and elevation boundary. Closing the window must not stop the agent or Minecraft servers, and normal operation must not require a terminal.
**Verify:** `bash tools/phase11/agent-install-smoke.sh --synthetic && npm --prefix clients/desktop-web run test:tauri-boundary && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P11.28i: start the local agent from the desktop shell`
**Batch:** stop-after

### P11.28j — Show an honest first-run and agent-unavailable state
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/help/FirstLaunchGate.svelte`, `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/tests/screens/`, `clients/desktop-web/tests/agent-install/`
**What:** Replace the generic connection failure path with a clear state model for missing, stopped, starting, ready, incompatible, and unavailable agents. Give the operator the next safe action—install, start, repair, or reconnect—without showing the first-time setup as complete before the real local agent is ready. Preserve browser users’ truthful no-native-service fallback.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run build`
**Commit:** `P11.28j: show real agent readiness states`
**Batch:** stop-after

### P11.28k — Walk the real client and agent through first-time setup
**Status:** superseded — deferred into P12.17 (decided 2026-08-25). This exercises the first-time-setup UI that Phase 12 rebuilds (P12.13); its real-agent/auth/first-server-handoff proof runs once against the redesigned client at the Phase 12 gate rather than against the superseded UI. The committed P11.28g–j agent wiring it would have exercised carries forward as Phase 12's foundation.
**Files:** `tools/phase11/real-client-agent-smoke.sh`, `clients/desktop-web/tests/e2e/`, `docs/msc2/clients/evidence/`, `differences.md`
**What:** Add a real-agent smoke path that launches or connects to MSC2 on `48001`, opens the actual Tauri client, authenticates through the real desktop credential path, and exercises the first-time setup from the opening screen through the final first-server handoff. Cover the server-root picker, Java check, Bedrock disclosure, optional helper links/checks, helper download state, Back/Next/Skip behavior, and restart/reopen behavior. Record fake-harness-only coverage separately and update `differences.md` only from this real-client evidence.
**Verify:** `bash tools/phase11/real-client-agent-smoke.sh --macos`
**Commit:** `P11.28k: prove real client and agent setup`
**Batch:** stop-after

### P11.29 — Reconcile the capability matrix and run the exact Phase 11 gate
**Status:** superseded — folded into P12.17 (decided 2026-08-25). Its checks (capability matrix, D-003 bundle/screen identity, generated DTO drift, D-013 host isolation, browser/Tauri auth, native Linux WebKitGTK rendering, tri-platform packaging, headless independence, and the onboarding/first-launch preservation contract) depend on the final client UI, so they run once against the *redesigned* client at the Phase 12 gate. Running this gate against the superseded UI is pointless — its own criterion requires preserving MSC 1's design, which only the redesign delivers.
**Files:** `docs/msc2/client-capability-matrix.csv`, `tools/phase11/phase11-check.py`, `docs/msc2/clients/evidence/phase11-ci.md`, `docs/msc2/clients/phase11-scope.md`, `docs/msc2/rolling-plan.md`
**What:** Run only after P11.28a–P11.28k. Check every contract and WebSocket operation against real Desktop/Web implementation evidence, leaving agent-Planned, Bedrock-screen, and player-profile rows explicitly Planned and no cell blank. Prove D-003 bundle/screen identity, generated DTO drift, D-013 host isolation, capability/permission routing, D-026 served-content use, browser/Tauri auth, browser responsive behavior, native Linux WebKitGTK rendering, tri-platform packaging, headless independence, and the exact green CI candidate. Also require the onboarding preservation contract in `phase11-scope.md`: fresh-profile setup → Concept Guide → guided tour → Handbook behavior, step/anchor coverage, skip/reopen persistence, splash playback/fallback, reduced-motion behavior, and honest separation of agent-owned first-server initiation from client-owned presentation. This is Phase 11's only full-workspace run; the other agent decides in REVIEW whether the literal gate holds.
**Verify:** `python3 tools/phase11/phase11-check.py --gate && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run --workspace`
**Commit:** `P11.29: check desktop and web gate`
**Batch:** solo

### P11.30 — Allow CORS on the unauthenticated /v1/health probe
**Status:** DONE (committed `0e2cc4f`). Discovered during P12.2 visual verification, not part of any planned step: a dev-mode Tauri window's own `fetch()` to this pre-credential readiness probe is cross-origin (devUrl `:1420` vs agent `:48001`), and the agent sent no CORS header at all, so the probe silently failed and the shell never got past "Agent starting" even though the agent was healthy. This route already runs outside the bearer-auth gate and carries no secrets, so a permissive allowance is safe here.
**Files:** `crates/msc-agent/src/routes/health.rs`
**What:** Set `Access-Control-Allow-Origin: *` on the `GET /v1/health` response only.
**Verify:** `curl -s -D - http://127.0.0.1:48001/v1/health` (with the agent running) shows the header; `cargo nextest run -p msc-agent --test browser_auth --test desktop_auth`.
**Commit:** `P11.30: allow CORS on the unauthenticated /v1/health probe`
**Batch:** solo

### P11.31 — Finish and land the local-agent auto-bootstrap auth
**Status:** DONE. Full account in `toughproblems/local-agent-auth-bootstrap.md` — read that before touching this area again. Discovered during P12.2 visual verification (same as P11.30): the real Tauri app could not connect to the local agent at all, through six stacked, independent bugs. Found `git stash@{0}` ("WIP: local-agent auto-bootstrap auth (pre-P12.1)") — Cameron's own substantial prior implementation of the same-machine bootstrap channel `docs/msc2/clients/phase11-auth.md` describes but this codebase never actually had working — and applied it (clean; only `App.svelte` had a trivial non-conflicting overlap with P12.2) rather than rewriting from scratch. Fixed forward from there: (1) the LaunchDaemon needs macOS admin elevation to start/stop, now done via `osascript ... with administrator privileges`; (2) ad-hoc-signing an already-running dev binary invalidates its own live code identity, fixed by signing-then-re-exec at startup instead of signing in place; (3) **keychain-based storage for the installation key and the store's root key was removed entirely** — both are now plain 0600 files under the agent's own secrets directory, since the agent and its desktop shell always run as the same regular user and keychain's ACL/session semantics kept failing in non-interactive-daemon contexts in ways that were each individually correct-looking and still wrong in practice; (4) a `ClientProof` struct was missing `#[serde(rename_all = "camelCase")]`, so every bootstrap proof submission failed JSON parsing (`hostId` sent, `host_id` expected) before it was ever checked. Real dev-workflow caveat recorded in the toughproblems doc: the installed plist's `MSC2_MACOS_DESKTOP_REQUIREMENT` snapshots one specific run's ad-hoc cdhash, which changes on every `tauri dev` relaunch — click "Repair service" once, right after the app under test opens, and don't restart it again before testing.
**Files:** `crates/msc-platform-macos/src/service.rs`, `crates/msc-platform-macos/src/secret_store.rs`, `crates/msc-platform-macos/Cargo.toml`, `crates/msc-agent/src/auth.rs`, `crates/msc-agent/src/auth/desktop.rs`, new `crates/msc-agent/src/auth/local_bootstrap.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/Cargo.toml`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/src/lib/platform/{index,browser,tauri,types}.ts`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, associated tests, new `toughproblems/local-agent-auth-bootstrap.md`
**What:** Make the real Tauri app able to authenticate to its own freshly-installed local agent without any manual pairing step, end to end, on macOS.
**Verify:** `cargo nextest run -p msc-platform-macos && cargo nextest run -p msc-agent --test browser_auth --test desktop_auth`; `npm --prefix clients/desktop-web run test:tauri-boundary && npx vitest run clients/desktop-web/tests/agent-install/agent-install.test.ts && npm --prefix clients/desktop-web run test:auth-desktop`. Cameron's own real-app verification: fresh install → Repair service → Reconnect connects without a manual pairing code.
**Commit:** `P11.31: land the local-agent auto-bootstrap auth`
**Batch:** solo

### P11.31a — Move the desktop app's own stored credential off keychain too
**Status:** awaiting verification. Cameron hit a real login-keychain access prompt ("msc2-desktop-web wants to use your confidential information stored in 'com.ctemple.msc2.desktop'...") during P12.3 verification and correctly recalled that keychain had been removed — P11.31's "removed entirely" only covered two *agent-daemon* secrets (installation key, store root key). A third secret was missed: the Tauri desktop app's own stored pairing credential (`StoredDesktopCredential`), still going through `MacosSecretStore::default_keychain_for_service` (the real login keychain) in `desktop_secret_store()`. Fixed to `MacosSecretStore::system()` — the same self-provisioning, file-rooted store the agent itself uses, sharing `agent_data_directory()`'s `secrets/` directory with the `local-bootstrap.key` file this same process already writes there directly (same user, same reasoning P11.31 already established for the other two secrets). Removed the now-unused `DESKTOP_SECRET_SERVICE` constant. Left `crates/msc-agent/src/auth.rs`'s own `default_keychain_for_service` call untouched — that one backs the foreground-`msc serve`-smoke-harness path the module's own doc comment already carves out as deliberate, not a fourth instance of this gap. Full account appended to `toughproblems/local-agent-auth-bootstrap.md` (§5).
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `toughproblems/local-agent-auth-bootstrap.md`
**What:** Stop the Tauri desktop app's own credential store from touching the macOS login keychain, matching the reasoning and pattern P11.31 already applied to the agent's two other secrets.
**Verify:** `cd clients/desktop-web/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`. Cameron's own real-app verification: fresh pair/reconnect on macOS no longer shows a Keychain access prompt.
**Commit:** `P11.31a: move the desktop app's own stored credential off keychain too`
**Batch:** solo

---

## Phase 12 — Client redesign (MSC 1 visual + behavioral fidelity)

**Gate** (`msc2-port-plan.md` §3): every MSC 1 screen and sheet has a rebuilt
MSC 2 counterpart that (a) matches MSC 1's shape and behavior — verified
screen-by-screen against MSC 1 by Cameron — and (b) passes the `antiAIslop.md`
checklist. The data/agent layer is unchanged in contract, and no screen exists in
the desktop app that is absent from the web UI (D-003 corollary).

**Why this phase exists.** Phase 11 delivered a working desktop/web client wired
to the real agent, but its UI diverged from MSC 1's information architecture and
design language — the very thing Phase 11 was meant to preserve. Phase 12 course-
corrects: it rebuilds the presentation layer so MSC 2 looks *and behaves* like
MSC 1, **refreshed** into one consistent, deliberately-designed system (MSC 1
grew by accretion and is inconsistent with itself; the refresh unifies it —
cleaner, less bulky, more modern, without becoming a different app).

**Keep / replace line (do not cross it).**
- **KEEP untouched** — the data layer that talks to the agent: `src/lib/api/`,
  `src/lib/platform/`, `src/lib/auth/`, `src/lib/stores/`, `src/lib/streams/`,
  `src/lib/operations/`, `src/lib/hosts/`. Backend/agent/Tauri/Rust changes are
  allowed only when a screen genuinely needs one, and are called out per step.
- **REPLACE** — the presentation: `src/App.svelte`, `src/lib/components/`,
  `src/lib/sections/*`, `src/lib/navigation/`, the shell.

**Required reading before any step:** `docs/msc2/antiAIslop.md` (hard rule #11)
and the locked reference specimens in `docs/msc2/renderings/`
(`status-card`, `buttons-and-type`, `primitives`, `shell`,
`decorated-vs-disciplined`). MSC 1 is the oracle for shape + behavior; its source
is at `~/Documents/Swift Projects/minecraft-server-controller/MSCmacOS/` and its
screenshots at `~/Documents/MSCSS/`. Each step begins by reading that screen's
own MSC 1 view(s) for behavior, not just its screenshot.

**How Phase 12 verification differs (recorded deviation from the normal loop).**
This is a *design* phase. Its `Verify:` is Cameron's **visual review** of the
running screen against MSC 1 plus the anti-slop checklist — not a single runnable
command. Two normal hard rules are relaxed here, deliberately:
- **Verify is a visual review**, not a pass/fail command. Where a cheap structural
  component test exists (`vitest`), the step names it too, but the *gate* is the
  eye, and the loop is: execute → Cameron looks → adjustment requests → repeat.
- **More than one commit per step is expected** — one commit per view/sheet, plus
  adjustment commits from the review loop. The "one commit per step" rule does not
  bind Phase 12.
Every step's Batch is therefore `solo`.

**Sections.** S0 (design system) and S1 (shell) were shaped and locked in advance
as the reference specimens; their steps below turn those locked specimens into
code. The rest apply the locked system to each screen, one at a time.

If a screen needs a UI pattern not covered by the locked S0 primitives, stop and surface it to Cameron as a design decision — extend the system deliberately (and add a renderings/ specimen), never improvise a one-off.

### P12.11 — Manage Servers / Hosts sheet (+ multi-host)
**Status:** DONE
**Files:** `src/lib/sections/fleet/`, sheet components, `src/lib/components/shell/ControlSidebar.svelte`, `src/App.svelte`
**What:** Rebuild the Manage sheet as the multi-host home, per the 2026-08-27 design discussion (recorded below). **Local** is auto-registered on first launch (via `DesktopSessionAuth.bootstrapLocal()`), always listed first, and cannot be removed — only remote hosts can be added or removed, so the UI never has to handle a zero-host state. A remote host is added through a new small form (label + base URL + pairing code) that calls the already-built `DesktopSessionAuth.redeemRemotePairing(baseUrl, pairingCode)` (`src/lib/auth/desktop.ts`) and registers the result in `HostStore` (`src/lib/hosts/registry.ts`) — no contract or backend changes, this is presentation over an already-finished path. With exactly one host, Manage renders pixel-identical to MSC 1's flat `ManageServersView` (server cards, Export/Import, Add Server) plus a small "Add Host…" affordance — no host-grouping chrome for a capability nobody's using. Once a second host exists, add a thin host-group header (label, connection dot, server count, rename/reconnect/remove-host actions) above each host's block of cards; the server cards themselves keep MSC 1's exact card treatment unchanged. Also replace `ControlSidebar.svelte`'s current flat `<select>` picker with the host-aware picker already specified in the locked `docs/msc2/renderings/shell.html` spec (`● Local ▸ Survival`, opening a menu grouped by host, with `Manage…` opening this sheet) — this requires `App.svelte` to stop hardcoding `localAgentHostId` and drive host selection from `HostStore` for real. **Left unresolved by this step, flagged rather than silently decided:** the original text folded "connectivity" (Playit/DuckDNS/resource packs/Xbox broadcast) into this same sheet; those look more like per-server concerns (arguably Server Editor's Broadcast sub-tab, P12.12) than host-level ones, and this wasn't covered in the design discussion — decide its home before or during this step rather than defaulting to "inside Manage" by inertia. Reference MSC 1 `ManageServersView` + `~/Documents/MSCSS/Manage Servers`, adapted for D-013 multi-host. **Resolved (2026-08-27, during P12.12's own planning):** connectivity (Playit/DuckDNS/resource packs/Xbox broadcast) is per-server, not host-level — it stays out of Manage and lives in Server Editor's Broadcast tab; see P12.12's own entry below.
**Verify:** `npm run dev`, open Manage with one host (compare to MSC 1, unchanged shape) and after adding a second (host grouping appears, Local still pinned/undeletable); exercise the sidebar picker's grouped menu and `Manage…` entry; compare to checklist. Structural: `npm run test:screen-fleet`.
**Commit:** `P12.11: rebuild manage servers and hosts`
**Batch:** solo

**Built (2026-08-27), awaiting Cameron's verification.** Manage is now `ManageSheet.svelte`, an actual `Sheet.svelte` modal (opened from the sidebar's `Manage…`), not the old `fleet` route/tab — `FleetSection.svelte` and the `fleet` section entry are retired, its `model.ts` reused as-is. `App.svelte` now owns a `HostStore` instance: Local is registered on mount as a plain registry record (id/label/baseUrl) and is never offered a remove action in the UI; the actual credential exchange for it is unchanged (`createAgentTransport` still calls `DesktopSessionAuth.bootstrapLocal()` internally for Tauri, exactly as before — the registry entry is bookkeeping, not a new auth path). `hostId` now comes from `hostStore.selectedHost` and `switchHost()` re-runs the existing `initializeClient()`/`restoreHostContext()` connection lifecycle unchanged.

**Finding, not a judgment call: multi-host UI is Tauri-only.** `src/lib/platform/index.ts`'s `createAgentTransport` (KEEP-list, unmodified) always targets `window.location.origin` off Tauri, with no per-host `baseUrl` override — a browser tab can only ever reach the one agent that served it. So `isDesktopShell` (resolved once in `App.svelte` via `getPlatform()`, named to avoid the literal string the D-003 platform-boundary test (`tests/tauri/platform-boundary.test.ts`) forbids in screen/route sources) gates Add Host, host-group headers, and cross-host switching everywhere; a browser always renders the exact single-host, no-chrome shape. This isn't new backend work — `redeemRemotePairing`/`HostStore` were already built — just where the UI is allowed to show up.

**Known, deliberate gaps against MSC 1's `ManageServersView`, left rather than papered over:** (1) the delete confirmation offers only "Remove from Controller" — `ServerDeleteRequestDTO` carries just `serverId`, no disk-delete flag, so MSC 1's "Delete from Disk" choice isn't offered. (2) The footer has no "Export…" — there is no `/v1/servers/export` route in the contract at all, so nothing was wired to a route that doesn't exist. (3) "Add Server…" is the existing simple name+version create form, not a port of MSC 1's multi-step `AddServerWizardView` — that wizard is real scope, not attempted here.

**Verify run so far:** `npm run test:screen-fleet` (2 passed), `npx svelte-check` (7 errors, all pre-existing/unrelated — `src/lib/auth/desktop.ts`, `SetupIntro.svelte`, `navigation/route.ts`, `tests/auth/desktop/desktop.test.ts`), `npx prettier --check` clean on every touched file, `npm run build` clean. Also ran the other test files that reference `App.svelte`/`ApplicationShell.svelte`/`ControlSidebar.svelte` (`tests/screens/help.test.ts`, `tests/navigation/navigation.test.ts`, `tests/tauri/platform-boundary.test.ts`, `tests/agent-install/agent-install.test.ts`, `tests/visual/shell.test.ts`) as a non-full-suite regression check: all pass except one pre-existing, unrelated failure already present at `HEAD` before this step (`tests/navigation/navigation.test.ts:141`, `expect(appSource).toContain('client.getCapabilities()')` — the code has always read `selectedClient.getCapabilities()`, a case mismatch that predates this step; not touched). Cameron's own visual pass (`npm run dev`) and the one-host/two-host comparison are still outstanding, per this step's own Verify line.

### P12.11a — Redesign the agent-unreachable screen (+ real `msc service` CLI)
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/App.svelte`, `crates/msc-agent/src/cli/service.rs`, `crates/msc-agent/tests/cli_service.rs` (new)
**Context for whoever executes this (written so it's self-contained — no prior conversation needed):** this screen has no MSC 1 equivalent at all; MSC 1 is one unit with no background agent to babysit, so there is no oracle to port against here, only the antiAIslop checklist and the rest of MSC 2's own already-rebuilt Phase 12 screens to match. P12.11 (multi-host Manage sheet) landed first, commit `1a34ef0` — `App.svelte` now has real host state to build on (`hostStore`, `hosts`, `hostId`, `isDesktopShell`, all declared near the top of the `<script>` block), so this step does not need to invent any of that.

**(1) Backend gap, fix inline (do not split into a separate step — same reasoning P12.9 used for its own inline backend fix).** `crates/msc-agent/src/cli/service.rs`, function `run()` (currently lines 62-74): it builds a real `ServiceManagerCommand` via `into_model()` but then always returns `Err(CliError::internal(format!("service management is modeled but not executable yet ({rendering}); platform adapters land in P4.22-P4.24")))` — that message is stale; P4.22-P4.24 landed months ago and are marked DONE in `rolling-plan-archive.md`. The real platform-specific logic already exists, is hardware-proven, and is called successfully today from a completely different entry point: `clients/desktop-web/src-tauri/src/lib.rs`, function `service_manager()` (currently lines 480-501) picks the right concrete manager by `#[cfg(target_os = ...)]` — `msc_platform_macos::service::MacosLaunchdServiceManager`, `msc_platform_windows::service::WindowsServiceManager`, or `msc_platform_linux::service::LinuxSystemdServiceManager` — each of which implements the `ServiceManager` trait (`crates/msc-infrastructure/src/service.rs:167`) and is driven by calling `.execute(ServiceManagerCommand::Status{..} / Start{..} / Stop{..} / Install(..) / Uninstall{..})`, exactly the enum `cli/service.rs::into_model()` already builds. Give the CLI binary the same `#[cfg(target_os = ...)]` selection (it can live in `cli/service.rs` itself or a small shared helper — `msc-agent` already depends on all three platform crates, confirm in `crates/msc-agent/Cargo.toml` and add the dependency if one is missing) and call `.execute(model)` on the result instead of erroring, then render `describe_command`'s existing formatting (or the manager's own returned status/report type — check what `ServiceManager::execute` actually returns in `service.rs` and adapt) as the command's real output. There is no `repair` verb at the `ServiceManager`/CLI level — confirm by reading `crates/msc-infrastructure/src/service.rs:167` (`ServiceManager` trait) and `ServiceManagerCommand` (same file, around line 80) — Tauri's "Repair" (`src-tauri/src/lib.rs:432`, `manage_agent_service`) is `uninstall` + `install` + `start` composed at the Tauri layer, not a primitive; do not invent a `Repair` variant at the CLI/infrastructure layer to match it.

**Known complication, make an explicit call and document it in this step's own write-up rather than leaving it half-solved:** `ServiceCommand::Install` (`cli/service.rs:9`, `ServiceInstallArgs`) requires `--binary-path`, `--working-directory`, `--log-path`, and `--expected-port` — there is no way to give a bare, memorize-and-type `msc service install --service-name msc-agent` command with no other arguments, because nothing in the CLI computes sensible defaults for those paths (Tauri's own equivalent, `agent_install_request()` in `src-tauri/src/lib.rs:503`, only works because it locates paths *relative to the installed desktop app bundle* — `packaged_agent_path()` at `src-tauri/src/lib.rs:631` walks up from `std::env::current_exe()` assuming a `.app`/installer layout that a headless CLI install has no equivalent of). Recommended resolution (part 2 below depends on this): only ever put `status`, `start`, and `stop` in the frontend's copy-paste panel — each needs nothing but the fixed `--service-name msc-agent` — and for the "not installed at all" / "needs a full reinstall" states, keep the copy pointing at reinstalling via the platform's headless package/installer rather than asking a user to hand-type an `install` command whose argument values they have no way to know. Do not fabricate placeholder default values for those four flags just to make an `install` one-liner look complete.

**(2) Frontend redesign.** Rebuild `AgentSetupSection.svelte` on the same S0 primitives every other already-rebuilt Phase 12 screen uses — `Button`, `Card`, `Badge`, `StatusDot`, `Field`, all under `clients/desktop-web/src/lib/components/base/` — replacing the pre-redesign components it currently imports: `ActionButton`, `StatePanel`, `SurfaceCard` (all in `clients/desktop-web/src/lib/components/`, top-level, *not* the `base/` ones — do not confuse the two families) and `ScreenHeader`/`StatusBadge` (in `clients/desktop-web/src/lib/sections/shared/` and top-level `components/` respectively). Read `docs/msc2/antiAIslop.md` before touching markup, same as every other Phase 12 screen. Head it with the host's name — `"‹Host name› agent"` — matching the breadcrumb pattern already visible elsewhere in this shell; the host's `label` is available wherever `hostLabel` is threaded (see prop-wiring below).

**Wiring this screen needs that it does not have today:** `AgentSetupSection` is instantiated in `App.svelte`'s `<svelte:component this={activeComponent} ... />` block (search for `onAgentRetry={() => void initializeClient()}` to find it) and currently receives only `api`, `hostId` (a bare string id), `serverId`, `permissions`, `readiness`, `onAgentRetry`. It needs two more props threaded through from state `App.svelte` already has: the connected host's base URL (available via `hostStore.getState(hostId).host.baseUrl` — `hostStore` is already a module-level instance in `App.svelte`, added by P12.11) and whether this is the desktop shell (`isDesktopShell`, already computed in `App.svelte` via `getPlatform()`, also added by P12.11 — reuse it directly rather than calling `getPlatform()` again inside the section). Add both as new props, e.g. `hostBaseUrl` and `isDesktopShell`, passed alongside the existing ones.

**Branch the screen's body three ways**, using those two new props:
- **`isDesktopShell` true (Tauri):** keep today's real Install/Start/Stop/Repair buttons and their existing `agentServiceStatus()`/`manageAgentService()` calls (`clients/desktop-web/src/lib/platform`) — restyle only, no behavior change.
- **`isDesktopShell` false and `hostBaseUrl` is loopback:** detect loopback by parsing `hostBaseUrl` with `new URL(...)` and checking `hostname === '127.0.0.1' || hostname === 'localhost' || hostname === '::1'` — this is an unspoofable same-machine signal (a browser can only load a page from loopback if it's running on that literal machine), not a heuristic that can be wrong. Replace the current dead-end "This browser cannot install a local background service… Install the headless package" message (see `clients/desktop-web/src/lib/platform/browser.ts`'s `agentServiceStatus`/`manageAgentService` stub text — that stub itself is in the KEEP-untouched `src/lib/platform/` and must not change) with a copy-pasteable terminal-command panel offering exactly `msc service status --service-name msc-agent`, `msc service start --service-name msc-agent`, and `msc service stop --service-name msc-agent`, each with its own Copy button — no `install`/`repair` commands here, per the complication noted above; if the status genuinely is "not installed," say so and point at the headless package documentation instead of a copy box.
- **`isDesktopShell` false and `hostBaseUrl` is not loopback (a different machine entirely):** show distinct copy — this client is not running on the machine that hosts this agent, so nothing here is actionable from this browser at all — with no service-control content, buttons, or commands of any kind.

**Verify:** Backend: `cargo nextest run -p msc-agent --test cli_service` — a new test file asserting `msc service status/start/stop/install/uninstall` actually execute (using `msc_infrastructure::service::FakeServiceManager`, `crates/msc-infrastructure/src/service.rs:172`, already used the same way by `src-tauri/src/lib.rs:863`'s own tests — inject it in place of the real per-platform manager for the test) instead of returning the old "not executable yet" error; also add/keep a `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` pass on the touched crate only. Frontend: `npm run dev` in `clients/desktop-web/`; stop the local agent and confirm the Tauri-branch buttons still work unchanged; simulate the loopback-browser branch (open the built web client at `localhost`/`127.0.0.1` with the agent stopped) and confirm the copyable command panel appears and that the printed commands actually work when pasted into a real terminal against this machine's own agent; simulate the non-loopback branch (point the client at a different host's address) and confirm the "not this machine" copy with no command content. No MSC 1 comparison applies — compare against `docs/msc2/antiAIslop.md`'s checklist and the rest of the already-rebuilt Phase 12 screens for visual consistency instead.
**Commit:** `P12.11a: redesign the agent-unreachable screen and wire the service CLI`
**Batch:** solo

### P12.11b — Make the agent screen reachable from the host picker
**Status:** DONE
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/components/ApplicationShell.svelte`, `clients/desktop-web/src/lib/components/shell/ControlSidebar.svelte`, `clients/desktop-web/tests/visual/shell.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Keep MSC 1's fixed seven server-detail tabs exactly as they are; the agent is host-scoped, not an eighth server tab. Instead, add one explicit `Agent…` action to the existing host/server picker beside `Manage…`. In `App.svelte`, give that action one callback that selects the already-registered `agent-setup` section for the active host, then thread it through `ApplicationShell.svelte` to `ControlSidebar.svelte`. The picker must expose it in both the one-host and multi-host menus, so browser and Tauri load the same reachable screen (D-003) and the selected host's label/base URL continue to drive P12.11a's three truthful states. Do not add backend calls, platform branching, a new primary-tab entry, or a second navigation model. Extend the existing source-level shell test to prove the callback is threaded and the picker contains `Agent…`; retain its anti-slop guard. Manually confirm `Agent…` takes an otherwise healthy running agent to the redesigned page in both the browser and Tauri window, then use the existing picker/server controls to return to a normal server tab.
**Verify:** `cd clients/desktop-web && npm run test:visual-shell && npx tauri dev` — in the running Tauri app, open the host/server picker, choose `Agent…`, and confirm the redesigned page opens while the agent is healthy; repeat against the browser UI at the same Vite origin and run the anti-slop checklist.
**Commit:** `P12.11b: make the agent screen reachable from the host picker`
**Batch:** solo

### P12.11c — Use macOS elevation for the agent Stop control
**Status:** DONE
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `crates/msc-platform-macos/src/service.rs`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11a verification finding: the desktop app was sending `launchctl stop <label>` from its unprivileged GUI process, which targets the GUI launchd context and fails with exit 3 for MSC's system LaunchDaemon. Add the matching elevated macOS stop helper beside `start_elevated`, wait until the shared status model reports `Stopped`, and route Tauri's existing Stop action through it. Keep the shared `ServiceManager` contract and non-macOS behavior unchanged. Include a focused unit check that protects the bare-label command shape (the privileged process resolves that label in the system domain).
**Verify:** `cargo nextest run -p msc-platform-macos --lib && cd clients/desktop-web && npx tauri dev` — open **Agent…**, click **Stop agent**, approve the macOS prompt, and confirm the page reports stopped; then click **Start agent** and confirm it returns to running.
**Commit:** `P12.11c: elevate macOS agent stop control`
**Batch:** solo

### P12.11d — Scope the agent screen to its selected host
**Status:** DONE
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/tests/agent-install/agent-install.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11a/P12.11b host-scoping finding. A Tauri desktop app must expose native Install/Start/Stop/Repair controls and local service status only when the selected host is its own Local agent; selecting a paired remote host must never operate or report the desktop computer's service under that remote host's name. On a browser, register the one reachable host using the page's actual origin, not the desktop loopback constant: loopback pages retain the copyable Terminal commands, while pages reached over a host address show the explicit “run service controls on ‹host›” non-actionable state. Keep browser multi-host unavailable as already required by D-003/D-013's current transport boundary; the Tauri picker remains the multi-host switcher. Add regression assertions covering the actual-origin, local-host, loopback, and remote-host gates. No remote service-control API is being invented: a remote host's service remains controlled on that host.
**Verify:** `cd clients/desktop-web && npx vitest run tests/agent-install/agent-install.test.ts -t "keeps service controls scoped" && npm run build && npx tauri dev` — in Tauri, add/select a remote host, open **Agent…**, and confirm it names that host but offers no service buttons or local PID/status; switch back to Local agent and confirm controls/status remain. In a browser at `localhost`, confirm the Terminal commands appear; open the same client through a non-loopback host address and confirm the remote-host message has no commands or buttons. (The broader `npm run test:agent-install` currently includes a pre-existing stale assertion for copy removed before this corrective step; it is not used as this step's verifier.)
**Commit:** `P12.11d: scope the agent screen to its selected host`
**Batch:** solo

### P12.11e — Restore the Agent screen after host scoping
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/tests/agent-install/agent-install.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11d regression found during Cameron verification. Its new reactive local-service refresh reads the host ID passed by `App.svelte`, but `AgentSetupSection` did not declare that prop, so opening **Agent…** throws at component initialization. Declare the existing `hostId` input with a safe default and extend the host-scope regression assertion to require it. Do not change routing, service behavior, or the selected-host policy from P12.11d.
**Verify:** `cd clients/desktop-web && npx vitest run tests/agent-install/agent-install.test.ts -t "keeps service controls scoped" && npm run build && npx tauri dev` — open **Agent…** from the host/server picker and confirm the page opens; repeat after selecting a remote host and then Local agent, confirming the P12.11d control boundary still holds.
**Commit:** `P12.11e: restore the agent screen after host scoping`
**Batch:** solo

### P12.11f — Open the local agent in a browser from desktop chrome
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/src/lib/components/ApplicationShell.svelte`, `clients/desktop-web/src/lib/components/shell/TopBar.svelte`, `clients/desktop-web/src/lib/components/shell/ShellIcon.svelte`, `clients/desktop-web/tests/visual/shell.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Add one neutral external-link action beside the existing sidebar and console controls in the desktop top bar. It opens the local agent-served browser UI at `http://127.0.0.1:48001` through the existing platform `openExternal` boundary. Extend the existing desktop external-link policy from HTTPS-only to allow that one loopback HTTP case (`127.0.0.1`, `localhost`, or `::1`) while continuing to reject every other HTTP and non-web URL; cover the policy with a unit test. Render the action only for Tauri; browser users are already in that surface. The action always means this desktop computer’s Local agent and must not suggest that MSC can open a browser on a paired remote host. Extend the existing shell source checks to cover the callback, desktop gate, label, and icon.
**Verify:** `cargo test --manifest-path clients/desktop-web/src-tauri/Cargo.toml external_url_policy_allows_https_and_local_agent_loopback_only && cd clients/desktop-web && npm run test:visual-shell && npm run build && npx tauri dev` — in the desktop window, click the new button between the console and Help controls; confirm the default browser opens `http://127.0.0.1:48001` and the desktop window stays open. Confirm the button does not render in the browser UI.
**Commit:** `P12.11f: open the local agent in a browser from desktop chrome`
**Batch:** solo

### P12.11g — Refresh the agent-served browser bundle
**Status:** awaiting verification
**Files:** `crates/msc-agent/web-ui/`, `docs/msc2/rolling-plan.md`
**What:** Correct the browser verification finding: the agent's embedded `web-ui/` snapshot had fallen behind the current Vite `dist/`, so desktop and browser delivery no longer shared one frontend. Rebuild the finished desktop-web client, replace the packaged agent bundle with that exact output using the existing packaging script, and verify every packaged file is byte-identical to `dist/`. Rebuild the development agent binary from the refreshed embedded bundle and place it at the already-installed desktop resource path; Cameron then uses the existing **Repair service** action once to install/start that exact resource binary. No browser-only fork or second frontend is introduced.
**Verify:** `cd clients/desktop-web && npm run bundle:package-agent && cd ../.. && python3 tools/phase11/bundle-identity-check.py && cargo nextest run -p msc-agent --test web_ui` — then in Tauri click **Repair service**, wait for the agent to return to running, use the top-bar browser button, and confirm `http://127.0.0.1:48001` renders the same current Svelte shell rather than the old disconnected page.
**Commit:** `P12.11g: refresh the agent-served browser bundle`
**Batch:** solo

### P12.11h — Authorize the browser opened by desktop
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/auth/browser-handoff.ts` (new), `clients/desktop-web/src/lib/platform/` (`types.ts`, `tauri.ts`, `browser.ts`), `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/tests/auth/browser-handoff.test.ts` (new), `clients/desktop-web/tests/visual/shell.test.ts`, `crates/msc-agent/web-ui/`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11f/g browser verification finding. The button successfully opens the agent-served shared bundle, but it opens it anonymously; it cannot transfer the desktop bearer token into a browser. Reuse P11.21's existing browser-pairing/session contract instead: the Tauri backend uses its stored local credential to create a full-permission, one-use browser pairing; it opens only the local loopback origin with that code in the URL fragment (never sent in HTTP); the shared browser startup immediately exchanges the fragment for the existing revocable httpOnly cookie and removes the fragment from browser history before normal API loading. The bearer token and pairing code never enter Svelte state. Repackage the changed shared frontend into the agent bundle. No MSC 1 equivalent exists: this is MSC 2's D-012 browser-cookie boundary, and it must retain D-003's one shared frontend.
**Verify:** `cd clients/desktop-web && npm run test:auth-browser && npx vitest run tests/auth/browser-handoff.test.ts && npm run test:visual-shell && npm run bundle:package-agent && cd ../.. && python3 tools/phase11/bundle-identity-check.py && cargo test --manifest-path clients/desktop-web/src-tauri/Cargo.toml local_browser_handoff_keeps_the_pairing_out_of_the_http_url && cargo nextest run -p msc-agent --test web_ui` — then run `npx tauri dev`, click the top-bar browser button, and confirm the new browser tab connects to the Local agent and reaches the normal Home screen without a pairing code in its address bar; hard-refresh the tab to confirm its cookie session persists.
**Commit:** `P12.11h: authorize the desktop browser handoff`
**Batch:** solo

### P12.11i — Retry stale local credentials in the browser handoff
**Status:** awaiting verification
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11h verification finding: a stale desktop credential makes the first browser-button click silently fail. The shared authorized-request helper correctly deletes a bearer record after a `401 Unauthorized`, but the handoff then stopped instead of using the now-cleared state to bootstrap a replacement. Keep the existing local bootstrap and browser-cookie contract unchanged; retry exactly once after a `401` by re-running the local bootstrap and browser-pairing creation. Do not retry permission denials or any other agent error. No MSC 1 equivalent exists: this is narrow recovery within MSC 2's D-012 local-desktop credential boundary.
**Verify:** `cargo fmt --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path clients/desktop-web/src-tauri/Cargo.toml local_browser_handoff && npx tauri dev` — with a stale local credential (or after the agent rejects one), click the top-bar browser button once and confirm it opens an authenticated browser tab; confirm a non-admin/forbidden pairing response is reported rather than retried.
**Commit:** `P12.11i: retry stale local credentials in browser handoff`
**Batch:** solo

### P12.11j — Show browser-handoff failures in the Agent screen
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/tests/visual/shell.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11h/i verification finding that the desktop browser icon can appear to do nothing: its native handoff error was retained only in App's shell message, which is not visible while the main shared shell is active. Preserve the existing secure handoff and stale-credential retry. On a handoff failure only, navigate to the already-reachable Agent screen and render the exact error using that screen's existing alert treatment; a successful click leaves the user where they are and opens the browser as before. No MSC 1 equivalent exists: this is truthful recovery copy for MSC 2's D-012 handoff boundary, not a second status pattern.
**Verify:** `cd clients/desktop-web && npm run test:visual-shell && npm run build && npx tauri dev` — click the browser icon with the agent reachable and confirm a browser tab opens; temporarily make the local handoff fail and confirm the desktop switches to **Background agent** with a readable error rather than silently doing nothing; restore the agent and confirm a successful click does not navigate away.
**Commit:** `P12.11j: show browser handoff failures in agent screen`
**Batch:** solo

### P12.11k — Survive Tauri hot reload in the browser handoff
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/platform/` (`index.ts`, `tauri.ts`), `clients/desktop-web/tests/tauri/platform-boundary.test.ts`, `clients/desktop-web/tests/visual/shell.test.ts`, `crates/msc-agent/web-ui/`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11j verification evidence: the Agent screen reports `openLocalAgentBrowser is not a function`. The Tauri adapter received the native handoff dependency but did not expose it on the platform interface, so even a fresh desktop shell could not call it. Add that delegation. At the shared platform boundary, also detect a development hot-reload's already-cached older adapter shape and dynamically invoke the registered native handoff command; a fresh shell uses the adapter normally, and a browser never gains a desktop action. Repackage the shared frontend so Tauri and agent delivery remain byte-identical. No MSC 1 equivalent exists: this is MSC 2 development-reload resilience at the D-003 shared-client boundary.
**Verify:** `cd clients/desktop-web && npm run test:tauri-boundary && npm run test:visual-shell && npm run bundle:package-agent && cd ../.. && python3 tools/phase11/bundle-identity-check.py && cargo nextest run -p msc-agent --test web_ui && npx tauri dev` — while `npx tauri dev` is already running, make the browser-handoff caller hot-reload and click the browser icon once; confirm it opens an authenticated tab rather than showing the stale-adapter error. Restart the desktop shell and repeat, confirming the normal adapter path also opens the tab.
**Commit:** `P12.11k: survive Tauri hot reload in browser handoff`
**Batch:** solo

### P12.11l — Return the pairing contract's created response
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/routes/browser_session.rs`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11k verification evidence: the desktop reaches `POST /v1/auth/pairings`, but rejects the agent's successful HTTP 200 response because the frozen OpenAPI contract requires HTTP 201 Created. The route already records a created audit event and the desktop correctly requires 201 before opening a one-use browser session; make the route send the matching 201 response and add a focused route-level regression test. Do not weaken the desktop handoff to accept a contract-incorrect success status. No MSC 1 equivalent exists: this is an MSC 2 D-012 contract enforcement repair.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc browser_pairing_creation_returns_created && cargo build -p msc-agent` — replace the installed development agent resource with the rebuilt binary, restart the local service, run `npx tauri dev`, click the browser icon once, and confirm an authenticated browser tab opens with no pairing code retained in its address bar.
**Commit:** `P12.11l: return the pairing contract's created response`
**Batch:** solo

### P12.11m — Pin the desktop to its packaged agent and restore browser navigation
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/navigation/serverSelection.ts` (new), `clients/desktop-web/tests/navigation/browser-context.test.ts` (new), `clients/desktop-web/src-tauri/src/lib.rs`, `crates/msc-agent/web-ui/`, `docs/msc2/rolling-plan.md`
**What:** Correct the combined P12.11l verification evidence instead of adding another status-code workaround. The installed service currently points directly at the desktop bundle's mutable agent resource, so replacing that file during development can leave launchd running the previous process while the desktop assumes it is current. Stage the packaged agent under a SHA-256 content-addressed path in MSC 2's application-data directory, install the service from that immutable path, and compare every local service report with the current package digest. A missing/mismatched service definition must report `unavailable` with a Repair instruction; local desktop bootstrap, browser handoff, and authorized loopback requests must fail closed before contacting a mismatched process. Apply the same definition check through the shared macOS/Windows/Linux service model rather than a launchd-only workaround. Also fix the browser's post-pairing navigation race in `App.svelte`: choose a server ID that actually exists (active server, retained selection, then first server), resolve the initial route from the freshly assigned capabilities/permissions instead of waiting for Svelte's reactive flush, and remove the internal `Acknowledge shell` placeholder so a successful browser handoff opens the real Overview/tabs. Repackage the shared bundle so the agent-served browser uses the same corrected frontend as Tauri. No MSC 1 equivalent exists for service generation pinning or browser pairing; the visible shell remains the locked Phase 12 design.
**Verify:** `cargo fmt --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path clients/desktop-web/src-tauri/Cargo.toml packaged_agent && cd clients/desktop-web && npx vitest run tests/navigation/browser-context.test.ts && npm run bundle:package-agent && cd ../.. && python3 tools/phase11/bundle-identity-check.py && cargo nextest run -p msc-agent --test web_ui` — then run `cd clients/desktop-web && npx tauri dev`; the old service definition must be refused before any API request, **Repair service** must install/restart the packaged generation, and one browser-icon click must open an authenticated tab with a real selected server and working Overview/Players/Worlds/etc. tabs, with no `Acknowledge shell` page.
**Commit:** `P12.11m: make local browser handoff deterministic`
**Batch:** solo

### P12.11n — Keep agent verification out of the request hot path
**Status:** awaiting verification
**Files:** `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/src/lib.rs`, `docs/msc2/rolling-plan.md`
**What:** Correct the P12.11m live verification regression. A process sample of the frozen Tauri shell showed concurrent `desktop_authorized_request` calls spending all available CPU inside `verify_staged_agent`: P12.11m re-read and SHA-256-hashed both 60 MB binaries before every local API request. Preserve the fail-closed service-definition comparison, but cache the successfully staged-and-verified content-addressed path for the lifetime of this desktop process. The packaged resource is immutable for a running release, and `tauri dev` restarts the Rust process when its package changes, so one verification per process preserves the generation guarantee while removing hashing from the request hot path. Optimize the `sha2` dependency in the development profile as well: a live stack sample after caching proved the one required initial verification still occupied an unoptimized development build for roughly a minute, while release builds already optimize it. Add a focused regression test proving repeated path resolution performs one staging operation.
**Verify:** `cargo fmt --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path clients/desktop-web/src-tauri/Cargo.toml packaged_agent` — then run `cd clients/desktop-web && npx tauri dev`; confirm the splash plays, the shell becomes responsive, CPU returns to idle, the real server/tabs load, and the browser button still opens an authenticated browser tab.
**Commit:** `P12.11n: cache packaged agent verification`
**Batch:** solo

### P12.11o — Authorize browser mutations after desktop handoff
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/platform/index.ts`, `clients/desktop-web/tests/auth/browser.test.ts`, `crates/msc-agent/web-ui/`, `docs/msc2/rolling-plan.md`
**What:** Correct the next browser-handoff verification finding. The one-use pairing exchange successfully creates the httpOnly browser session, and `BrowserSessionAuth.credentialAdapter()` already implements the frozen D-012 CSRF flow, but `createAgentTransport()` discards that adapter and wires the browser client to `cookieCredentialAdapter()`, which sends no `X-MSC-CSRF` header. As a result every browser mutation is rejected with `csrf_invalid`: first-run servers-root/Java saves, Xbox-helper download, Start/Stop, settings, and all other POST/PUT/DELETE actions. Construct the browser transport with a `BrowserSessionAuth` bound to the selected agent origin and use its credential adapter; keep safe GET requests cookie-only and keep Tauri's native credential bridge unchanged. Add a regression assertion at the production transport boundary and repackage the shared bundle. **Design finding recorded, not expanded into this repair:** `FirstLaunchGate` currently mixes host-owned setup with per-browser `localStorage`; the correct follow-up is agent-persisted host setup state plus a separate per-client welcome/tour, with native path pickers shown only in Tauri and browser users operating on explicitly labeled host paths/defaults.
**Verify:** `cd clients/desktop-web && npx vitest run tests/auth/browser.test.ts && npm run bundle:package-agent && cd ../.. && python3 tools/phase11/bundle-identity-check.py && cargo nextest run -p msc-agent --test web_ui` — then open from the desktop browser button, proceed past **Server Setup** using the agent-reported defaults, and confirm later browser mutations (including Start/Stop) succeed without `csrf_invalid`.
**Commit:** `P12.11o: authorize browser mutations`
**Batch:** solo

### P12.11p — Separate host setup from client onboarding
**Status:** awaiting verification
**Files:** `crates/msc-domain/src/app_config_schema.rs`, `crates/msc-domain/tests/app_config_schema.rs`, `crates/msc-api/src/dto/versions.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/src/routes/versions.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/help/FirstLaunchGate.svelte`, `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`, `clients/desktop-web/tests/screens/help.test.ts`, `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `clients/desktop-web/tests/e2e/tauri-linux/native-renderer.test.ts`, `crates/msc-agent/web-ui/`, `docs/msc2/rolling-plan.md`
**What:** Implement the host/client ownership split approved after P12.11o. Reuse `AppConfig.initial_setup_done`, already ported from MSC 1 `AppConfig.swift:537,623,687,749-750,854` and set by `AppViewModel+ServerSettings.swift:28`, instead of adding another persisted flag. Treat any config containing a registered server as setup-complete even when an older MSC 2 config explicitly persisted `initial_setup_done: false`, so an already-configured host migrates without replaying setup. Add `HostSetupStateDTO { complete: boolean }`, `GET /v1/config/host-setup` (permission `none`), and bodyless `POST /v1/config/host-setup/complete` (permission `settings`); the POST persists `initial_setup_done = true`, returns the same DTO, and reports save failures as HTTP 500 `set_failed`. `FirstLaunchGate` and the Handbook's first-launch reader must obtain setup completion from that agent route, while Concept Guide and tour completion remain per-client `localStorage` values and are never written into host config. Finishing `SetupIntro` must persist host completion before dismissing, with an inline retryable error if the save fails. On the server-path page, continue using the agent's existing `GET /v1/config/servers-root`, `GET /v1/config/java-runtime`, and `GET /v1/java-runtimes` defaults; label browser-entered values explicitly as paths on the agent host, and render the native `Browse…` controls only when `PlatformKind` is `tauri`. Repackage the shared frontend so Tauri and the agent-served browser remain byte-identical. This intentionally sharpens MSC 1's single-process ownership: host prerequisites remain durable with the host, while each browser/desktop client gets its own welcome and tour.

**Picked up (2026-08-27) after the executing agent (Codex) ran out of budget mid-step**, everything above this paragraph is Codex's own write-up and code, verified rather than rewritten: the backend (`HostSetupStateDto`, the two routes, the `initial_setup_done` migration OR-fix) and the frontend (`FirstLaunchGate`/`HelpSection`/`SetupIntro` reading and persisting host-setup state through the new route, `platformKind`-gated `Browse…`) all matched the plan and passed their own targeted checks unchanged. `npm run api:generate -- --check` (in place of `npm run api:check`, per Codex's own note about that checker's pre-existing `Record<string, never>` blind spot) and `npx vitest run tests/screens/help.test.ts` both passed as committed.

**Three real, pre-existing bugs surfaced while finishing this step's own Playwright evidence — not introduced by this step, but living in files this step already touches, and directly blocking the one check that actually exercises the new host-setup flow — so fixed forward rather than worked around:**
1. `contract-harness.mjs` never served `GET /v1/auth/csrf`. P12.11o's `BrowserSessionAuth.headersForMutation()` (`src/lib/auth/browser.ts:50`) fetches that route before every browser POST/PUT/DELETE and throws if the agent doesn't serve it — so in this test harness, *every* browser mutation has been silently failing client-side (never even reaching the network) since P12.11o landed, including this step's own `POST /v1/config/host-setup/complete` and the pre-existing `POST /v1/config/servers-root` save. Added the route, matching the real agent's shape (`crates/msc-agent/src/routes/browser_session.rs`'s `CsrfTokenResponse { csrfToken, expiresAt }`), plus `x-msc-csrf` to the harness's CORS allow-headers list (needed for the cross-origin Tauri-window case `tests/e2e/tauri-linux/native-renderer.test.ts` exercises) and `DELETE` to allow-methods (`BrowserSessionAuth.logout()` needs it).
2. The harness's `/v1/help/catalog` fixture and topic objects used a field named `id` (and `markdown` for body text); the real, frozen contract (`crates/msc-agent/src/help.rs`'s `HelpCatalogEntry`/`HelpTopic`, both `#[serde(rename_all = "camelCase")]` on `help_id`/`body`) and the frontend's own `HelpTopic`/`HelpCatalog` types (`src/lib/help/types.ts`) both use `helpId`/`body`. Both fixture topics therefore had `item.helpId === undefined` — a real Svelte `each_key_duplicate` runtime error in `HelpSection.svelte`'s topic-list `{#each catalog.topics as item (item.helpId)}` (harmless there, since real key-diffing doesn't blast the page), but load-bearing for `HelpSection.svelte`'s own `topicId = ... ?? catalog.topics[0]?.helpId ?? ''` fallback, which silently resolved to `''`, 404'd against `GET /v1/help/`, and left the reader permanently on "That topic is not available on this agent" instead of the real Overview topic — exactly the state the tour-completion handoff lands on. Renamed both fixture fields to match the real DTO shape.
3. `tests/e2e/browser/workflows.spec.ts`'s `walks a fresh profile...` test asserted `gate.getByRole('heading', { name: 'playit.gg' })` (and the same pattern for Xbox Broadcast/Tailscale/You're All Set) with no `level` given; every one of those setup pages also renders an h3 subsection whose text contains (or, for "You're All Set", exactly repeats) the page's own h2 title, so Playwright's default substring match resolves to multiple elements once the assertion is actually reached (a strict-mode violation, not a "not found"). Added `level: 2` to each of these page-title assertions, since `SetupIntro.svelte`'s own page-title heading (`src/lib/help/SetupIntro.svelte:379`) is always an `<h2>` regardless of which sub-card headings share its text.

With all three fixed, `npx playwright test tests/e2e/browser/workflows.spec.ts --project=chromium -g "walks a fresh profile"` passes end to end (setup wizard through every optional page, Concept Guide, guided tour, real Overview handoff, tour restart) — this is the test that actually proves this step's host-setup behavior, and it had never once passed in this repository's history before today, masked by bug 1 (from P12.11o onward) and, before that, by the harness's `/v1/status` first-call-always-503 behavior described below.

**Fourth pre-existing bug, narrowly worked around rather than fully fixed:** `contract-harness.mjs`'s `/v1/status` handler (`if (count === 1) return json(response, {...}, 503)`) deliberately 503s the *very first* `/v1/status` call any client makes, to simulate a reconnect-pending state for the separate `keeps the local host identity and presents reconnect fallback` test. Because the counter is keyed only by User-Agent (shared across every test in the file, not reset per test), whichever test happens to make the first-ever call pays for it with a spurious "Agent unavailable" screen — confirmed present and already broken on bare `main` HEAD before this step's changes (reproduced by hand against commit `5ccea89`), independent of host setup. That made `walks a fresh profile...` fail whenever run alone or first, which is exactly how this step's own Verify line needs to run it. Rather than touch the shared counter's design (risking the reconnect-fallback test it exists for), extended the `/__test/host-setup` reset this test already calls at its own first line to also mark that request's own User-Agent as already past the first call (`statusRequests.set(client, 1)`), so a fresh-profile walk — which has nothing to do with reconnect simulation — no longer eats it. This does not touch any other test's behavior (confirmed: `keeps the local host identity and presents reconnect fallback` and `names destructive targets and completes bounded upload and download workflows` still fail exactly as before, unrelated to host setup, when run first/alone — they don't call `/__test/host-setup` and are untouched by this step). Those two, plus `renders the production bundle` when it runs first, still need the counter's underlying design fixed (e.g. reset per test some other way, or keyed off something test-scoped) in a future corrective step; flagged, not fixed, here.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-domain -p msc-api -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-domain --test app_config_schema && cargo nextest run -p msc-agent --bin msc host_setup && cd clients/desktop-web && npm run api:generate -- --check && npx vitest run tests/screens/help.test.ts && npm run bundle:package-agent && npx playwright test tests/e2e/browser/workflows.spec.ts --project=chromium -g "walks a fresh profile" && cd ../.. && python3 tools/phase11/bundle-identity-check.py && cargo nextest run -p msc-agent --test web_ui` — then restart `npx tauri dev`, use **Repair service** to install the repackaged agent, and confirm: the configured local host opens without Server Setup in both Tauri and a newly opened browser; clearing only browser storage replays the Concept Guide/tour but not Server Setup; an unconfigured empty host receives agent defaults, browser copy says the paths are on the host with no `Browse…` buttons, and Tauri retains both native pickers.
**Commit:** `P12.11p: separate host setup from client onboarding`
**Batch:** solo

### P12.4k — Add Import ZIP / Replace World / Duplicate Slot to the Worlds tab
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/worlds/` (`WorldsSection.svelte`, `model.ts`, new sheets alongside the existing `CreateWorldSheet.svelte`/`RenameWorldSheet.svelte`/`WorldRepairSheet.svelte`/`WorldConversionWizard.svelte`)
**What:** **Design decision (2026-08-27):** Cameron's call, sharper than P12.12's first draft below — world actions should live in exactly one place, not split across Details and Editor. P12.4's own write-up had already carved Import ZIP / Replace World / Duplicate Slot out of the Worlds tab and provisionally assigned them to Server Editor's `World` sub-tab; that assignment is reversed here. Port `ServerEditorWorldTab.swift`'s three actions directly into the already-built Worlds tab instead: Import ZIP (external archive → new slot), Replace World (overwrite the live world from an external source, using the existing `WorldReplaceRequestDTO`/`WorldReplaceActiveRequestDTO` split already wired in `crates/msc-agent/src/routes/worlds.rs` per P12.4's own finding), and Duplicate Slot. This makes the Worlds tab the single, complete home for every world behavior MSC 1 has (Create, Save Current, Activate, Rename, Delete, Convert, Repair, Import, Replace, Duplicate) and removes any World-shaped tab from Server Editor entirely (see P12.12 below). Land this step before or alongside P12.12 so world actions are never briefly homeless.
**Verify:** `npm run dev`, open Worlds, exercise Import ZIP / Replace World / Duplicate Slot on a real server; compare each against MSC 1's `ServerEditorWorldTab` + the World screenshot; confirm nothing else in the tab regressed. Structural: `npm run test:screen-worlds-backups`.
**Commit:** `P12.4k: bring Import/Replace/Duplicate into the Worlds tab`
**Batch:** solo

### P12.4l — Collapse world slot actions into a menu
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldSlotCard.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`
**What:** Cameron's own follow-up call on P12.4k's landed layout: a persistent 5-button grid per world-slot card (Activate/Convert/Rename/Duplicate/Delete) read as cluttered. Collapse those per-card actions into the same anchored `Menu` overlay `ComponentsSection.svelte`'s addon rows and `ManageSheet.svelte`'s server rows already use — a small "more actions" trigger opens a floating list, the destructive item is styled rather than separately colored, and the menu itself is one shared instance owned by `WorldsSection.svelte` (matching those two screens' own pattern), not one per card. The inline-confirm flow for Activate/Duplicate/Delete (P12.3g's expand-in-place pattern) is unchanged; only the entry point moves from always-visible buttons to the menu. Selecting a card for its Backups panel is unchanged, just recolored to the shared `--msc2-selection` token those rows use for their own selected state.
**Verify:** `npm run dev`, open Worlds, open a card's "World actions" menu and exercise Set as Active/Convert/Rename/Duplicate/Delete; confirm the destructive item styling and single shared overlay match `ComponentsSection`/`ManageSheet`. Structural: `npm run test:screen-worlds-backups`.
**Commit:** `P12.4l: collapse world slot actions into a menu`
**Batch:** solo

### P12.12 — Server Editor sheet (General, Broadcast)
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/server-editor/` (new), `clients/desktop-web/src/lib/sections/fleet/ManageSheet.svelte` — `App.svelte` turned out not to need any change: `ServerEditorSheet` is a self-contained child of `ManageSheet` (same pattern `ProjectDetailSheet` uses inside `ComponentsSection`), so no shell wiring was required.
**What:** **Design decision (2026-08-27, replacing this step's original "7 sub-tabs" scope):** reading the MSC 1 oracle side by side with the already-built Details tabs (P12.2–P12.10) showed the two view families genuinely duplicate each other in MSC 1 itself — not a porting artifact, real accretion. `ServerEditorSettingsTab.swift` and `DetailsSettingsTabView.swift` embed the literal same `ServerSettingsView` component with two independent Save-Changes drafts; `ServerEditorJarsTab.swift`'s server-jar/mod/Geyser-Floodgate management is the same territory the already-built Components tab (P12.7) now owns live; the Editor's `Backups` tab (global auto-backup policy + manual backup/prune) duplicates the per-slot auto-backup toggle+interval the already-built Worlds tab's `BackupsPanel` (P12.4) already has; and per Cameron's own further call, `World` isn't split between the two view families at all any more — P12.4k moves the remaining Import ZIP/Replace World/Duplicate Slot actions into the Worlds tab, so every world behavior lives in exactly one place. The organizing rule throughout: keep in Details whatever is day-to-day/live-server operation, keep in Editor only what's genuinely setup-time and reached exclusively from Manage (never from inside the live Details workspace, confirmed via `grep -rn "ServerEditorView(" MSCmacOS` — the oracle only ever opens it from `ManageServersView.swift:123`).

Reached via a new `Edit…` action on each server card in `ManageSheet.svelte` (P12.11) — add this trigger if it isn't already there — separate from that sheet's existing simple "Add Server…" create form (P11.9/P12.11 already established that form as its own thing, not this sheet; this step is for editing an existing registered server, not initial creation). Depends on P12.4k landing first (or alongside), so a server being edited never has its world actions missing from both places at once. Build exactly two tabs:
- **General** — `displayName`, `serverDir`, min/max RAM, EULA accept, auto-restart-on-crash, notes, the headless-script generator, **Delete Server** (`ServerEditorGeneralTab.swift`, 1:1 with `ConfigServer`), **plus** the four notification toggles (start/stop/join/leave) carried over from the dropped Settings tab — they're per-server MSC-level config, the same character as auto-restart.
- **Broadcast** — `ServerEditorBroadcastTab.swift`'s full Xbox broadcast config (enable, IP mode, alt-account email/gamertag/password notes, helper JAR download, reset sign-in; Components' Crossplay row keeps its existing quick enable-toggle mirror, unchanged) **plus** Playit tunnel setup/start-stop, DuckDNS setup, and resource-pack config. This resolves P12.11's own flagged-open "connectivity" question: those are per-server network/hosting concerns, not host-level, so they belong here rather than in the Manage sheet — Manage stays host identity + server list only.

**Explicitly not rebuilt as separate tabs, not silently dropped:** `Settings` (Details' Settings tab, P12.8, already covers server.properties fully, reachable even for a stopped/just-created server — no reason for a second independent draft), `JARs` (Details' Components tab, P12.7, already owns live server-jar/mod/Geyser-Floodgate management; initial jar/version choice for a new server stays in the existing Add Server form), `Backups` (Details' Worlds tab's `BackupsPanel`, P12.4, already has per-slot auto-backup + Back Up Now — no separate global-policy tab), `World` (fully absorbed into the Worlds tab by P12.4k — no World-shaped tab exists in Editor at all). `Docker` (`ServerEditorDockerTab.swift`) is excluded entirely for now — D-008 is still **Proposed**, not Approved, so this step builds no Docker surface at all; revisit only once D-008 resolves.
**Amended (2026-08-27):** reopened by **P12.12a** below — Cameron reviewed MSC 1's actual Java placement and decided the Server Editor should gain a third **Java** tab (global Java executable/Detect/Install, shown only for Java servers) rather than leaving it for P12.14's MSC Settings sheet. See P12.12a for the full decision and scope; this step's own General/Broadcast scope above is unchanged.
**Verify:** `npm run dev`, open Edit on an existing server, walk General/Broadcast; compare General against MSC 1's equivalent, Broadcast against MSC 1's Broadcast tab plus its Playit/DuckDNS setup flows (wherever they currently render), and confirm Settings/JARs/Backups/World do *not* reappear as Editor tabs. Run the `antiAIslop.md` checklist.
**Commit:** `P12.12: rebuild the server editor`
**Batch:** solo

### P12.12a — Add a Java tab to the Server Editor
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/server-editor/JavaTab.svelte` (new), `clients/desktop-web/src/lib/sections/server-editor/ServerEditorSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/model.ts`, `docs/msc2/rolling-plan.md`
**What:** **Design decision (2026-08-27):** reopens P12.12's closed scope — see the top-of-file "Java tab decision" note for the full reasoning. In MSC 1, the Java executable path, Detect (`JavaRuntimeManager.detectInstalledJavaRuntimes` → `JavaRuntimePickerSheet`), and Install Java… (`JavaInstaller`'s Minecraft-version-framed major picker → Adoptium Temurin download) all live in one place, `PreferencesJavaSection` (`MSCSettingsSections.swift:4-108`) inside the app-wide `MSCSettingsView`, reused once more by the Setup Wizard's Java step. The value it edits (`AppConfig.javaPath`) is genuinely global — one JVM launches every Java server on the host, never a per-server value. Cameron's call: surface the same global value here instead, as a third `SegmentedControl` option in `ServerEditorSheet.svelte` labeled **Java**, rendered only when `currentServer.serverType === 'java'` (Bedrock servers keep today's two-tab General/Broadcast layout unchanged) — because editing a Java server is the moment this setting is actually relevant, not because it becomes per-server data.

Build `JavaTab.svelte` against routes that are already frozen in the contract and already backed by real agent code (no contract or backend work needed this step):
- `GET`/`POST /v1/config/java-runtime` (`JavaConfigResponseDto`/`JavaConfigSetRequestDto`, `executablePath` only) — read/save the manual path field.
- `GET /v1/java-runtimes` (`JavaRuntimesResponseDto` → `JavaRuntimeDto[]` with `name`/`executablePath`/`majorVersion`) — Detect button opening a picker list; port `JavaRuntimePickerSheet`/`JavaRuntimePickerRow`'s shape (name, path, major version, an explicit "no runtimes found — you can still paste a path manually" empty state) rather than inventing a new one.
- `POST /v1/java-runtimes/install` (`JavaRuntimeInstallRequestDto{major}` → `JavaRuntimeInstallResultDto{success,message,operationId}`) — Install Java… button opens a picker over the four Minecraft-framed majors (8/17/21/25, mirroring `msc_domain::java_runtime::MINECRAFT_INSTALL_OPTIONS`'s Minecraft-version ranges and recommended flag — hardcode the same four rows client-side; not worth a new route for a static four-item list), then polls the returned `operationId` with this sheet's own already-existing `pollOperation` helper (`model.ts`) until it succeeds or fails, the same pattern already used elsewhere in this sheet.
- `SetupIntro.svelte`'s `probeJava`/`browseJava` functions are a working reference for this exact detect/browse/save sequence against the same routes — reuse the pattern rather than re-deriving it. Show the native `Browse…` file picker only when `PlatformKind` is `tauri`, matching P12.11p's precedent for host-path fields on a browser client.

Because the value is host-wide, the tab must say so plainly (e.g. "This Java executable runs every Java server on this host") — word it as host config being edited from a convenient place, not as if it only affects the server being edited.

**Explicitly excluded, not silently dropped:** MSC 1's "Extra JVM flags" field (`AppConfig.extraFlags`) has no backing DTO or route anywhere in the contract — `JavaConfigResponseDto`/`JavaConfigSetRequestDto` carry only `executablePath`, even though `crates/msc-domain/src/app_config_schema.rs:891`'s `extra_flags` already exists in the domain model. Same shape as P12.12's own General/Broadcast gaps: left out rather than faked. Add it later as its own small contract-amendment step if wanted.
**Amended (2026-08-27):** corrected by **P12.12b** below after Cameron's visual review of the built tab — adds the "Extra JVM flags" field this step deferred, removes the host-wide banner, and fixes the sheet resizing per tab.
**Verify:** `npm run dev`, open Edit on a Java server and confirm the Java tab appears with working Detect / Install Java… / manual-path save, each checked against a real host; open Edit on a Bedrock server and confirm no Java tab appears. Compare shape and copy against MSC 1's `PreferencesJavaSection` and `JavaInstallerSheet`. Run the `antiAIslop.md` checklist.
**Commit:** `P12.12a: add a java tab to the server editor`
**Batch:** solo

### P12.12b — Fix the Java tab per Cameron's visual review
**Status:** awaiting verification
**Files:** `crates/msc-api/src/dto/versions.rs`, `crates/msc-agent/src/routes/versions.rs`, `crates/msc-agent/src/cli/mod.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/components/base/Field.svelte`, `clients/desktop-web/src/lib/sections/server-editor/JavaTab.svelte`, `clients/desktop-web/src/lib/sections/server-editor/ServerEditorSheet.svelte`, `docs/msc2/rolling-plan.md`
**What:** Corrects P12.12a against Cameron's screenshots of the built tab. Four changes:
1. **Adds the "Extra JVM flags" field P12.12a explicitly deferred** for lacking a route. Adds `extraFlags: Option<String>` to `JavaConfigResponseDto`/`JavaConfigSetRequestDto` (contract + Rust DTO + regenerated `generated.ts`), and wires `get_java_config`/`set_java_config` to read/write `AppConfig.extra_flags` alongside `java_path`. Found and fixed a real bug while doing it: `set_java_config` previously applied `executable_path` unconditionally, defaulting to `"java"` whenever the field was absent from the request body — harmless while it was the route's only field, but it would have silently reset a configured path the first time a flags-only save landed on this now-shared route. Both fields now apply independently (`if let Some(...)` per field), matching the tab's own per-field Save-button pattern; a regression test (`set_java_config_route_saves_each_field_independently`) covers both directions. `crates/msc-agent/src/cli/mod.rs`'s `msc java set` command updated for the new DTO field (`extra_flags: None`, unchanged CLI behavior).
2. **Removes the host-wide-scope banner** ("This Java executable runs every Java server on this host…") — Cameron's call, not needed.
3. **Executable path stays first, Java Arguments (Extra JVM Flags) is a new zone directly underneath it** — same `Card`/row visual language as the executable-path zone, its own Field + Save.
4. **Stops the sheet resizing between tabs** — `ServerEditorSheet.svelte`'s General/Broadcast/Java content now renders inside a fixed-height (`560px`), independently-scrolling `.tab-panel`, instead of letting the sheet's own outer frame grow or shrink with each tab's content. `Field.svelte` gained an additive `multiline` mode (opt-in textarea rendering) for the flags field; every existing single-line caller is unaffected.

Pre-existing, unrelated gaps noticed and left alone (out of scope for this step): `tools/api-contract-check.py --v1-summary` reports 133 routes against an expected-131 baseline, and `npm run api:check` reports `OnboardingGuideDTO` missing from generated output — both reproduce identically on the pre-P12.12b commit, so neither was caused by this step.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-domain -p msc-api -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc -E 'test(java_config)' && cd clients/desktop-web && npm run api:check && npm run check` — then `npm run dev`, open Edit on a Java server's Java tab and confirm: no banner, Extra JVM Flags saves and persists independently of the executable path, and switching General/Broadcast/Java no longer changes the sheet's size.
**Commit:** `P12.12b: fix java tab per visual review`
**Batch:** solo

### P12.13 — First-time setup / Prerequisites / Setup wizard
**Status:** not started
**Files:** `src/lib/sections/setup/`, `src/lib/help/` (first-launch)
**What:** Rebuild the first-launch flow — setup sheet, prerequisites/Java check, Bedrock disclosure, helper links, first-server handoff — to MSC 1's shape and ordering. Reference MSC 1 `SetupWizardView`, `PrerequisitesView`, `FirstStartSheetView` and `~/Documents/MSCSS/First Time Setup`. Preserve the agent-owned vs client-owned initiation boundary from P11 scope.
**Verify:** `npm run dev` on a fresh profile, walk setup start → first-server handoff; compare to MSC 1 + checklist.
**Commit:** `P12.13: rebuild first-time setup`
**Batch:** solo

### P12.14 — MSC Settings (app settings)
**Status:** not started
**Files:** `src/lib/sections/settings/` (app-level) or a dedicated app-settings section
**What:** Rebuild the app-wide MSC Settings sheet. Reference MSC 1 `MSCSettingsView` + `~/Documents/MSCSS/MSC settings`. **Java excluded (decided 2026-08-27):** `PreferencesJavaSection` (Java executable path, Detect, Install Java…) moved to Server Editor's new **Java** tab instead (P12.12a) — Cameron's call to surface that global setting in the per-server-editing context rather than draft the same global state from two independent screens. This step covers the rest of MSC 1's Preferences sections only (`MSCSettingsSections.swift`'s Process Cleanup, Remote API, Shared Access, Data Folders, Storage, Config Recovery, Ports, Archive, etc.).
**Verify:** `npm run dev`, open MSC Settings, exercise each pane; compare to MSC 1 + checklist. Confirm no Java section reappears here.
**Commit:** `P12.14: rebuild MSC settings`
**Batch:** solo

### P12.15 — Onboarding tour / contextual help
**Status:** not started
**Files:** `src/lib/help/`, onboarding overlay components
**What:** Rebuild the guided tour + contextual help anchors/overlay to MSC 1's ordering and anchors. Reference MSC 1 `OnboardingOverlayView`, the contextual-help system, and `~/Documents/MSCSS/Onboarding Tour`. Respect reduced-motion.
**Verify:** `npm run dev`, run the tour start→finish, check anchors + skip/reopen; compare to MSC 1 + checklist.
**Commit:** `P12.15: rebuild the onboarding tour`
**Batch:** solo

### P12.16 — Guides / Handbook / How MSC Works
**Status:** not started
**Files:** `src/lib/sections/handbook/`, `src/lib/help/`
**What:** Rebuild the Server Handbook, Concept Guide ("How MSC Works"), and router/port-forward guides, consuming the `GET /v1/help/{helpId}` content contract (no hardcoded divergent text). Reference MSC 1 `ServerHandbookView`, `ConceptGuideView`, `RouterPortForwardGuideSheet` and `~/Documents/MSCSS/{Guides,Server Handbook,How MSC Works}`.
**Verify:** `npm run dev`, open each guide, confirm content comes from the help contract; compare to MSC 1 + checklist.
**Commit:** `P12.16: rebuild guides and handbook`
**Batch:** solo

### P12.17 — Consistency sweep + parity gate
**Status:** not started
**Files:** `docs/msc2/clients/`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/renderings/`
**What:** Run after P12.0–P12.16. Sweep every rebuilt screen for design-system consistency (uniform sheet sizes, spacing, one card language, no drift), run the `antiAIslop.md` checklist across all screens, and do the screen-by-screen MSC 1 parity comparison (shape + behavior). Confirm the D-003 corollary (no desktop-only screen) and that the kept data layer's contract is unchanged. **This step also absorbs the deferred Phase 11 gate (P11.28k + P11.29), now evaluated against the redesigned client:** reconcile the capability matrix; prove D-003 bundle/screen identity, generated DTO drift, D-013 host isolation, capability/permission routing, D-026 served-content use, browser/Tauri auth, browser responsive behavior, native Linux WebKitGTK rendering, tri-platform packaging, and headless independence; and satisfy the first-launch preservation contract (real-agent/auth path, fresh-profile setup → Concept Guide → guided tour → Handbook, step/anchor coverage, skip/reopen, splash/reduced-motion, and the agent-owned vs client-owned initiation boundary). Record evidence. The other agent decides in REVIEW whether the combined Phase 12 gate holds.
**Verify:** `cd clients/desktop-web && npm run test:unit && npm run check`, then `python3 tools/phase11/phase11-check.py --gate && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run --workspace`, then Cameron's full visual parity + anti-slop pass across every screen against `~/Documents/MSCSS/` and MSC 1. (This is Phase 12's only full-workspace run.)
**Commit:** `P12.17: consistency sweep and parity gate`
**Batch:** solo
