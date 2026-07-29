# MSC 2 — Decision Register

**Revision:** 1.3 · **Date:** 2026-07-29
**Owner:** Cameron Temple

**Purpose:** the authoritative record of *what was decided, by whom, and why*. The product and engineering documents describe the destination; this document explains how it was chosen, what was rejected, and when a decision should be reopened.

**Read this first** if you are returning to MSC 2 with no memory of the project. Every entry is self-contained.

**Baseline:** MSC 1 at commit `fccd61f0ed743086f1f5db6bef58e228a36010f3`, 246 production Swift files, 97,357 lines.

**Companion documents:** `MSC2-VISION.md` (index and precedence) · `msc2-product.md` · `msc2-engineering.md` · `msc2-port-plan.md`

---

## Status vocabulary

| Status | Meaning |
|---|---|
| **Approved** | The owner personally confirmed this. Do not reopen without new evidence of the kind named in *Revisit if*. |
| **Proposed** | Analysis-derived recommendation. Reasonable, documented, **not yet owner-confirmed.** Safe to build against provisionally; must be approved before it constrains anything expensive. |
| **Open** | Identified, not decided. |

Every entry records **Origin** (where the idea came from), **Approved by**, and **Approval date**. A decision with `Approved by: —` is a proposal wearing a decision's clothes, and is labelled accordingly.

---

## Index

| ID | Decision | Status | Approved |
|---|---|---|---|
| D-001 | MSC 2 is a separate product, not a refactor of MSC 1 | **Approved** | 2026-07-29 |
| D-002 | Rust for the engine | **Approved** | 2026-07-29 |
| D-003 | Tauri shell over a shared Svelte frontend | **Approved** | 2026-07-29 |
| D-004 | The iOS app is evolved, not rewritten | **Approved** | 2026-07-29 |
| D-005 | Behavior is ported, not reimagined | Proposed | — |
| D-006 | MSC 1's API is the compatibility baseline (not the whole API) | Proposed | — |
| D-007 | macOS Bedrock stays Swift behind a sidecar | Proposed | — |
| D-008 | The Docker Bedrock backend is not ported | Proposed | — |
| D-009 | MSC 1 and MSC 2 share nothing; import only | **Approved** | 2026-07-29 |
| D-010 | Version skew: floor with capability degradation | **Approved** (mechanism) / Proposed (N-3) | 2026-07-29 |
| D-011 | Headless is independently installable on all three platforms | **Approved** | 2026-07-29 |
| D-012 | Authentication and session model | **Approved** (cookie + injected local token) / Proposed (full design) | 2026-07-29 |
| D-013 | Multi-host data model from day one | **Approved** | 2026-07-29 |
| D-014 | Minecraft version floor is 1.20 | **Approved** | 2026-07-29 |
| D-015 | v1 non-goals | **Approved** | 2026-07-29 |
| D-016 | Port strategy (principles only; sequence lives in the port plan) | Proposed | — |
| D-017 | Windows validation starts with the substrate | Proposed | — |
| D-018 | Behavioral evidence must be captured before translation | Proposed | — |
| D-019 | MSC 1's named-token permission model is preserved and formalized | Proposed | — |
| D-020 | Where MSC 2's documents and code live | Open | — |
| D-021 | Resource efficiency is a measurable requirement | **Approved** (requirement) / Proposed (targets) | 2026-07-29 |
| D-022 | MSC platform support and Bedrock platform support are separate matrices | Proposed | — |
| D-023 | Full client capability, tracked by an explicit matrix | **Approved** (requirement) / Proposed (mechanism) | 2026-07-29 |
| D-024 | Power management has two policies, by host role | Proposed | — |
| D-025 | Service identity and privilege boundaries | Open | — |

---

## D-001 — MSC 2 is a separate product, not a refactor of MSC 1

**Status:** Approved · **Origin:** Owner · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Owner's words:** *"I currently run MSC on a separate mac. I want MSC 2 to be a completely separate app, new project. It should not touch MSC 1 at all. I should be able to import from MSC 1 though."*

**Context.** MSC 1 is a mature macOS SwiftUI application whose server-management logic lives inside view models. Running it requires a macOS GUI session. That single constraint produces every limitation MSC 2 exists to remove.

**Decision.** MSC 2 is a new project in a new repository. It does not modify MSC 1, does not read MSC 1's configuration, and shares no state, files, or process space with it.

**Rationale.** MSC 1 runs live infrastructure for real servers on a dedicated Mac. That machine must keep working, untouched, for a port measured in months.

**Alternatives rejected.** Incrementally morphing MSC 1 (no intermediate state is both shippable and progress) · shared configuration (two writers on one metadata store; MSC 1 already carries config-recovery code because that file has gone bad before) · one-way migration retiring MSC 1 (removes the fallback exactly when MSC 2 is least proven).

**Revisit if:** never, for the duration of the port.

---

## D-002 — Rust for the engine

**Status:** Approved · **Origin:** Owner (with Codex, in `msc2.md`) · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Decision.** The MSC 2 engine, service, API, and CLI are written in Rust, shipping as a single binary per platform.

**Rationale.** Native Windows support is a firm owner requirement — *"there would be a scenario, for those who run regular Ubuntu or desktoped Linux distro. I want it to also have work in native Windows."* That is the argument that carries the decision. Rust additionally provides single-binary distribution, mature service and process-control ecosystems on all three platforms, strong async I/O, and a terminal-UI ecosystem with no equivalent elsewhere.

**Explicitly NOT the rationale.** Memory savings. The difference between a Rust agent and a Swift agent is a small, unmeasured fraction of the memory actually in contention — tens of megabytes against the gigabytes that decide whether a modpack fits. The gigabytes come from not running a graphical desktop environment on the host, which is a deployment decision, not a language decision. **Do not reopen this on memory grounds.** (The real memory requirement is D-021.)

**Strongest rejected alternative.** Extracting MSC 1's engine into cross-platform Swift (Hummingbird/Vapor on Linux). Viable for macOS and Linux; would have preserved domain code directly and kept shared model types with the iOS client. Rejected solely because Swift on Windows is the weak leg. **If native Windows were ever dropped, this becomes the better answer.**

**Revisit if:** native Windows ceases to be a requirement.

---

## D-003 — Tauri shell over a shared Svelte frontend

**Status:** Approved · **Origin:** Owner (with Codex, in `msc2.md`) · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Decision.** One Svelte frontend. A Tauri shell loads it as the desktop application; the agent serves the same bundle to browsers. The desktop app and the web UI are the same code.

**Rationale.** The largest available scope reduction: four graphical surfaces collapse to one frontend plus a thin shell. Sharing a single codebase substantially reduces drift — it does not make drift impossible, since conditional shell-only paths can still diverge, which is what the corollary below guards against.

**Critical clarification, recorded so the error is not repeated.** Tauri requiring Rust for its shell is **not** what makes the engine Rust. A Tauri shell is a window that loads a frontend. The engine's language was decided independently (D-002). Conflating them is a reasoning error that was made and corrected during planning.

**Consequences.** The desktop app is a web view. Tauri restores native menus, file pickers, notifications, and fast launch, but it is not AppKit and this will be noticeable on macOS. Accepted deliberately.

**Corollary.** No screen may exist in the desktop app that does not exist in the web UI. Native-only behavior belongs in the shell layer, not in a divergent screen.

---

## D-004 — The iOS app is evolved, not rewritten

**Status:** Approved · **Origin:** Owner (with Codex, in `msc2.md`) · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Decision.** The existing SwiftUI iOS client is retained and re-pointed at the MSC 2 API.

**Rationale.** Its UI work is unaffected by the engine's language, and pointing an existing production client at the new agent tests the API contract far more honestly than a client written against it.

**Consequences.** The iOS app must speak both APIs during transition, or be branched. Its models should become generated code (D-006), retiring the hand-maintained mirror tests.

**Important caveat.** Retaining the app does not mean retaining feature coverage automatically. See D-023.

---

## D-005 — Behavior is ported, not reimagined

**Status:** Proposed · **Origin:** Claude + Codex audits (independent, then reconciled) · **Approved by:** —

**Decision.** MSC 1 is the executable specification and compatibility oracle. No domain behavior is redesigned from memory. Each ported domain demonstrates parity against fixtures derived from MSC 1's actual behavior before its Swift implementation is considered superseded.

**Rationale.** MSC 1's value is accumulated edge-case knowledge — loader-specific TPS formats, crash signatures, installer quirks, blocked-mod handling, client-only classification precedence, unknown-field preservation in `server.properties`. None of it is documented outside the Swift and its tests. Re-deriving it means re-encountering the failures that produced it, in front of users.

**Supporting evidence.** Two independent audits agreed at file level on **88.6%** of 246 files.

**Consequences.** MSC 1 must remain runnable and buildable throughout the port for oracle comparison.

---

## D-006 — MSC 1's API is the compatibility baseline, not the whole of MSC 2's API

**Status:** Proposed · **Origin:** Claude + Codex audits (independent convergence) · **Approved by:** —

**Context.** MSC 1's Remote API was initially believed read-mostly. Measurement disproved this: **49 POST and 38 GET routes** across eight files (5,652 lines), with ~55 KB of DTO wire schema exercised daily by the iOS client.

**Decision.** MSC 1's API is the **compatibility baseline**, not the entirety of MSC 2's API. It is captured as a versioned OpenAPI description plus explicit WebSocket event schemas, which become the single source of truth generating both Rust server types and Swift client models.

Three things follow, and conflating them is a mistake:

1. **Baseline** — where MSC 1's API covers a capability, its observable behavior is normative and existing clients must keep working.
2. **Extension** — MSC 2's API is a **superset**. MSC 1 has desktop capabilities its Remote API never exposed; MSC 2 adds endpoints for them. The baseline says what may not break, not what may not be added.
3. **Correction** — documented bugs, security weaknesses, and wrong semantics **may be fixed** rather than preserved. A quirk is not a contract because it shipped. Corrections are explicit, versioned, and never silent.

**Preserved:** field names, optional/default behavior, route meanings, role and permission behavior, rate-limiting intent, request-size limits, 404-vs-405 semantics, audit records, WebSocket authentication and delivery, iOS-visible error semantics.

**Not preserved:** hand-written socket parsing, mutable provider-closure storage, `AppViewModel` as provider owner, DTO nesting that exists only for Swift file organization.

**Consequences.** The seven `AppViewModel+APIWiring*.swift` files (2,701 lines) are deleted once their mappings become contract tests.

---

## D-007 — macOS Bedrock stays Swift behind a sidecar

**Status:** Proposed · **Origin:** Claude + Codex audits · **Approved by:** —

**Context.** Bedrock Dedicated Server has no macOS build. MSC 1 solves this with `VMBedrockServerBackend.swift` (451 lines) driving `Virtualization.framework`, verified working with a real device joining on 2026-06-30.

**Decision.** The Swift VZ implementation is retained as a macOS-only sidecar binary supervised by the Rust agent over a narrow process protocol. It is not rewritten in Rust.

**Rationale.** `Virtualization.framework` is bridgeable from Rust via `objc2`, so this is engineering judgement rather than impossibility. It is a delegate-heavy framework with an async VM lifecycle; hand-rolled bridging of working, proven code is high risk for no early product value.

**Sequencing constraint.** The shared runtime contract is proven on native Linux first, then Windows, then the sidecar — so macOS-specific assumptions cannot leak into the contract.

**Revisit if:** the sidecar IPC proves more troublesome than the ObjC bridge would have been.

---

## D-008 — The Docker Bedrock backend is not ported

**Status:** Proposed · **Origin:** Codex audit (Claude initially misclassified this file as a port target) · **Approved by:** —

**Evidence.** `BedrockServerBackend.swift` (658 lines) contains **116 references to Docker**; it streams output via `docker logs -f` and sends commands via `docker exec`. It is the backend the de-Dockerization work superseded.

**Decision.** Do not port. It remains a behavioral reference in MSC 1 only.

**Consequences.** Any future Linux compatibility-container Bedrock backend is designed fresh against the runtime contract from D-007.

**Recorded because the mistake was silent** — the filename gives no indication it is Docker-based.

---

## D-009 — MSC 1 and MSC 2 share nothing; import only

**Status:** Approved · **Origin:** Owner · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Decision.** MSC 2 has its own application-support directory, configuration schema, and metadata. It never reads or writes MSC 1's state. The only path between them is an explicit, user-initiated import.

**Primary import mechanism.** MSC 1 already contains a server-transfer feature (`AppViewModel+ServerTransfer.swift`) producing an archive with a manifest, handling sanitization, port-conflict detection, and secret handling. MSC 2 reads that format — a versioned export artifact is far more stable to target than a live internal config schema.

*Constraint:* MSC 1's transfer-package format becomes a stable interface for the migration period.

**Secondary import.** A raw server-directory importer, inferring loader, version, worlds, and settings and clearly labelling what it could not determine. Required regardless of MSC 1.

**Not supported.** Reading MSC 1's config in place · sharing an application-support directory · two apps managing one server directory concurrently.

---

## D-010 — Version skew: floor with capability degradation

**Status:** **Approved** (mechanism) · **Proposed** (the specific N-3 value)
**Origin:** Claude proposal, mechanism selected by owner · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Context.** The iOS client ships through App Store review and users do not update promptly. Client and agent will routinely differ.

**Approved mechanism.** A supported-version floor, capability degradation within the window, clear refusal below it, and a new major route namespace for breaking changes. New fields additive and optional. The agent reports API major/minor and its capability set on connect.

**Proposed, not yet approved: the floor is three minor versions.** N-3 is an analysis estimate, not an owner decision. It should be set from real App Store update-adoption data once MSC 2 ships, not guessed now.

**Alternatives rejected.** Lockstep versioning (App Store review latency would routinely brick the phone client) · indefinite compatibility (shims accumulate forever; old clients fail in confusing partial ways).

**Consequences.** Old-client/new-agent and new-client/old-agent compatibility fixtures are required test assets.

---

## D-011 — Headless is independently installable on all three platforms

**Status:** **Approved** · **Origin:** Owner — `msc2.md`, *"Every platform supports both a full graphical application and a complete headless mode"* · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Approved.** On macOS and Windows the user installs the desktop application normally; on first launch it offers to install and start the background service. The macOS bundle includes the Swift Bedrock sidecar (D-007).

**Amendment.** Revision 1.0 described standalone headless installation for Linux only. That was a drafting error, not a decision — the owner's original vision document already required complete headless mode on every platform. **Headless is a first-class installation mode everywhere:**

| Platform | Headless requirement |
|---|---|
| **macOS** | Agent + CLI installable and runnable with no GUI ever launched. Registered as a **`launchd` LaunchDaemon** — a LaunchAgent requires a login session and is insufficient. No AppKit or window-server dependency at runtime. The standalone headless package **includes the Swift VZ sidecar** where Bedrock support is expected. |
| **Windows** | Agent + CLI installable as a Windows Service without the desktop app; runs with no user signed in. |
| **Linux** | Agent + CLI package with **zero desktop dependencies**; `systemd` unit; installs on a minimal Debian with no X/Wayland present. |
| **All** | The Tauri GUI is optional everywhere and is never a prerequisite for any capability. |

**Rationale.** The owner already runs MSC on an always-on spare Mac managed mostly from iOS. Treating headless as a Linux-only concern would fail the existing deployment on day one.

**Consequences.** Two distribution artifacts per platform: an application bundle and a headless package. Self-update must handle app, agent, and sidecar as a coordinated set on macOS/Windows and defer to the package manager on Linux. Headless packages must be verified to link no GUI frameworks (D-021).

---

## D-012 — Authentication and session model

**Status:** **Approved** (browser cookie + injected desktop-local token) · **Proposed** (everything else below)
**Origin:** Claude proposal, transport model selected by owner; expansion from Codex review · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Approved core.** Browsers authenticate via a pairing code exchanged for an **httpOnly, SameSite session cookie** — not JS-readable, revocable server-side, surviving refresh. The Tauri shell injects a local token so a desktop app controlling its own machine never presents a login. iOS keeps QR pairing → durable keychain token → bearer header.

**Rejected.** Bearer token in browser storage (readable by any script on a page that manages people's worlds) · open-on-loopback (lets anything running locally drive the agent, and headless hosts are browsed remotely anyway).

**Gap identified by Codex review.** "Tauri injects a local token" only covers a desktop app talking to its own computer. Given multi-host from day one (D-013), a desktop app connecting to *remote* hosts is a first-class case and was unspecified. The following must be designed before this entry can be Approved in full:

1. **Local automatic authorization** — how the agent proves a request originates from the same machine, and what stops another local process from impersonating the shell.
2. **Remote desktop pairing** — the desktop app's equivalent of the iOS QR flow, for each remote host.
3. **Per-host credential storage** — one credential per host in the platform secret store, keyed to match the multi-host client model.
4. **LAN encryption expectations** — whether plain HTTP is permitted off-loopback at all; certificate provisioning and trust for a locally managed TLS certificate.
5. **Tailscale connections** — whether tailnet membership may relax any requirement (default position: no; token authentication remains mandatory over Tailscale).
6. **Browser origin policy** — allowed origins, CSP for the served frontend, and CSRF protection for cookie-authenticated mutating requests (bearer-authenticated requests exempt).

**Until these are specified, treat the auth design as incomplete.**

---

## D-013 — Multi-host data model from day one

**Status:** Approved · **Origin:** Claude proposal, selected by owner · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Decision.** All client state — connection, credentials, capabilities, cached server lists, console buffers — is keyed by host from the first line of client code. v1 ships a minimal host picker.

**Rationale.** A data-model decision disguised as a feature. Retrofitting host scoping onto a single-host client means touching every store, cache, and screen.

**Consequences.** The UI must always display which host it controls, prominently enough to prevent a destructive action against the wrong machine. Credential storage is per-host (see D-012 item 3).

---

## D-014 — Minecraft version floor is 1.20

**Status:** Approved · **Origin:** Claude raised the question; value chosen by owner · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Owner's words:** *"floor at 1.20."*

**Decision.** MSC 2 supports Minecraft 1.20 and later. Older versions are out of scope — not blocked with a hostile error, but not tested, not supported, and not carried in provisioning logic.

**Rationale.** Every additional supported generation costs loader quirks, Java-version mappings, args-file shapes, and installer variations. A version floor is among the cheapest scope levers available.

**Consequences.** Java runtime mapping simplifies (1.20+ is Java 17/21). Fixtures need only cover loader behavior from 1.20 forward. Below-floor servers may still be imported and run if their files work, but MSC promises no version-aware correctness for them.

**Revisit if:** real users turn out to run older packs in meaningful numbers.

---

## D-015 — v1 non-goals

**Status:** Approved · **Origin:** Claude proposal, confirmed by owner · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Owner's words:** *"keep all those non-goals."*

| Non-goal | Kind | Reasoning |
|---|---|---|
| **Terminal TUI** | Deferred to v1.1 | A third client surface on the same API. Built during the port, it must be maintained through every contract change. The scriptable CLI and interactive prompts still ship in v1. |
| **Third-party plugin / extension API** | Deferred indefinitely | No third party exists yet. A stable extension surface with no consumer is pure cost and freezes internals prematurely. |
| **Per-person user accounts with identity** | Deferred | MSC 1's existing named-token model with permission categories carries forward unchanged — see D-019, which corrects an earlier misdescription of this baseline. What is deferred is *human identity*: invitations, per-person login, account recovery. |
| **Any TempleTech-hosted account, relay, telemetry, subscription, or cloud backend** | **Permanent** | MSC is local-first. Recorded as permanent so it is never casually revisited. **This does not exclude optional third-party integrations** — Tailscale, Playit.gg, DuckDNS, Modrinth, CurseForge, Adoptium, and Xbox services remain supported. |
| **Proxy / multi-server network orchestration** | **Permanent** | Velocity and BungeeCord networks are the most predictable request once MSC 2 manages multiple servers. MSC is not a network orchestration system. |
| **Android client** | Non-goal | Nothing in the plan assumes it. Recorded to stop it creeping in. |

**Explicitly NOT a non-goal: the Windows GUI.** Windows is a firm requirement and the reason the engine is Rust (D-002). Only its *sequencing* is later (D-017, port plan).

---

## D-016 — Port strategy (principles only)

**Status:** Proposed · **Origin:** Claude, informed by both audits · **Approved by:** —

**This entry deliberately contains no sequence.** Detailed phasing lives in `msc2-port-plan.md`, which is an execution document and may change without touching the vision.

**Principles.**

1. **Vertical slices, not subsystems.** Each stage cuts through engine, API, and clients together. The failure mode for a rewrite this size is a long period with an engine and no working software — easy to fall into when the specification is organized by subsystem, which the original vision document was.
2. **Behavioral evidence precedes translation** (D-018).
3. **Extraction precedes translation, per domain.** Parsers and policies embedded in MSC 1's views and view-model extensions are pulled out *in Swift*, where the compiler still checks the refactor, before that domain is translated.
   *Amended per Codex review:* this is **not** a blanket "before any Rust is written" gate, and **not** a mechanical grep rule. Extraction is driven by a symbol ledger — **which does not exist yet.** The two audit CSVs are *file-level* dispositions; they identify which files need symbol-level review, not which symbols inside them must be preserved. Building the ledger, one row per parser/policy/workflow inside a Mixed or UI file, is real unstarted work. Only behavior that must be preserved is extracted. Client-side concerns — file pickers, image cropping, window presentation — legitimately remain in UI code and are not extraction targets.
4. **UI never gates correctness.** The graphical clients are built against a proven API; their completion is not a prerequisite for headless agent correctness.
5. **Highest data-loss domains are ported while the codebase is still small enough to review carefully.**

---

## D-017 — Windows validation starts with the substrate

**Status:** Proposed · **Origin:** Codex review of Claude's port order · **Approved by:** —

**Decision.** Windows CI for the agent — path handling, process ownership, service integration, config and secret storage — begins with the filesystem/config/security substrate, not with the GUI.

**Rationale.** Windows is why the engine is Rust, but the Windows product arrives late. Deferring all Windows validation risks discovering path-separator, path-length, service-lifecycle, and file-locking assumptions after most of the engine is written against POSIX semantics.

---

## D-018 — Behavioral evidence must be captured before translation

**Status:** Proposed · **Origin:** Codex audit (Claude initially understated this) · **Approved by:** —

**Context.** MSC 1 has 21 test files, 4,888 lines, 270 test methods — concentrated on parsing and API contracts. Coverage is strong where failure is cheap and weak where failure is expensive.

**Decision.** Before a Rust implementation becomes authoritative for user data, two things must exist:

1. **Extracted fixtures.** The 270 tests encode expectations as inline Swift string literals and cannot be consumed by Rust. They become language-neutral fixtures (input + expected output).
2. **Characterization tests that do not exist yet** for the destructive workflows: world mutation and rollback, backup while running, interrupted restore, retention with one good backup, archive traversal, process tree cleanup, agent restart with a live server, real modpack archives and interrupted installs, historical config migrations, per-route authorization, and version-skew compatibility.

**Rationale.** This is the only thing that makes "same behavior" a checkable claim. It is cheap while MSC 1 runs and can be observed, and impossible afterwards.

**This is the highest-value finding of the audit process.** The initial reading — "a 270-test corpus exists, extract it" — understated the work substantially.

---

## D-019 — MSC 1's named-token permission model is preserved and formalized

**Status:** Proposed · **Origin:** Codex review, correcting a factual error in the previous revision · **Approved by:** —

**Correction to revision 1.0.** The previous entry described MSC 1 as having only "admin/guest" access and proposed building a per-capability permission shape so identities could be added later. **That understated the existing system.** Verified in MSC 1:

```
RemoteAPIServer.swift:91    enum TokenRole { case admin(label: String); case guest(label: String) }
AppConfig.swift:470         RemoteAPISharedAccessEntry {
                              id, label, token, role,
                              permissions: [String]?, expiresAtISO8601
                            }
RemoteAPIServer+UserRoutes  POST /users        → (label, role, permissions, expiresInDays)
                            POST /users/update → (userId, label, role, permissions, expiresInDays)
                            POST /users/revoke
```

Permission category strings (`players`, `settings`, `worlds`, `mods`) are enforced in the HTTP dispatcher. **MSC 1 already has named tokens, per-category custom permissions, and expiry.**

**Decision.** MSC 2 preserves this model and formalizes it: the permission category vocabulary becomes part of the versioned API contract rather than ad-hoc strings, and every route declares the capability it requires.

**What is genuinely deferred** (and belongs in D-015) is *human identity* — per-person accounts, invitations, login, recovery — not permissions, which exist.

**Open work.** The current category vocabulary has not been validated against all 87 routes; it may need to be finer or coarser. Determine before the contract is frozen.

---

## D-020 — Where MSC 2's documents and code live

**Status:** **Open** · **Origin:** Claude · **Approved by:** —

MSC 2 is a new project in a new repository (D-001), but that repository does not exist. These documents currently live in `~/Desktop/visionmscclaude/` alongside audit artifacts in `~/Desktop/`.

**Open:** repository name and location · whether audit artifacts move in as historical appendices · whether these documents are version-controlled there from the start.

**Recommendation:** create the repository early, before code, and move all planning and audit artifacts into `docs/`. "Source of truth" and "untracked Desktop file" do not coexist for long.

---

## D-021 — Resource efficiency is a measurable requirement

**Status:** **Approved** (the requirement) · **Proposed** (the specific benchmark values)
**Origin:** Owner — `msc2.md` establishes the 8 GB constraint as the project's founding motivation; made measurable following Codex review · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Context.** The originating motivation for MSC 2 is that an 8 GB 2019 MacBook Pro cannot safely give a large modpack the 5–5.5 GB it needs while macOS consumes the rest. Revision 1.0 discussed bounded memory in passing but made the central objective unmeasurable.

**Decision.** Resource efficiency is a stated requirement with acceptance criteria, not an aspiration:

1. **No GUI dependencies in headless packages.** Headless artifacts must be verified — mechanically, in CI — to link no GUI framework on any platform (D-011).
2. **Bounded memory by construction.** Console buffers, metric history, catalog caches, operation journals, and task data all have explicit bounds. No unbounded growth is acceptable in a long-lived agent.
3. **Idle-agent benchmark targets, established by measurement.** A published target for agent resident memory when idle with one stopped server, and one with a running server, measured per platform. Targets are set from first measurement, then defended by regression tests — not guessed in advance.
4. **Safe Java allocation guidance.** MSC distinguishes Java heap from total machine memory and reports installed memory, available memory, configured heap, estimated non-heap overhead, process resident memory, and swap usage. It recommends a safe allocation and warns before unsafe ones — while permitting an informed override.
5. **Swap detection and classification.** MSC distinguishes healthy unused swap, brief emergency swap activity, sustained memory pressure, and imminent OOM risk, and reports sustained swapping as a performance problem.
6. **A representative acceptance scenario.** An 8 GB minimal-Linux host running a demanding modpack at approximately 5 GB heap, with the agent, must remain within safe headroom under normal play. This scenario is a release gate, not a demo.

**Rationale.** Without this, nothing in the document set measures the reason MSC 2 exists.

---

## D-022 — MSC platform support and Bedrock platform support are separate matrices

**Status:** Proposed · **Origin:** Codex review · **Approved by:** —

**Context.** Revision 1.0 said "native Linux Bedrock," which implies that any Linux distribution MSC supports is automatically a supported Bedrock Dedicated Server environment. That does not follow. BDS ships with its own distribution and library expectations, which are narrower than the set of systems on which the MSC agent can run.

**Decision.** MSC 2 publishes **three separate compatibility matrices**:

1. **MSC agent** — where the agent itself is supported (broad: macOS, Windows, mainstream Linux including minimal Debian).
2. **Java servers** — where Java Minecraft servers are supported (essentially wherever a supported JRE runs).
3. **Bedrock runtime** — where Bedrock Dedicated Server is supported, per backend: native Linux (specific distributions and library requirements), native Windows, and the macOS VZ sidecar.

**Consequence.** A host may be a fully supported MSC agent host and a fully supported Java server host while not being a supported Bedrock host. The interface must say so plainly rather than failing at runtime.

---

## D-023 — Full client capability, tracked by an explicit matrix

**Status:** **Approved** (full iOS capability is required) · **Proposed** (the matrix as the tracking mechanism)
**Origin:** Owner — `msc2.md`, *"The phone is not a reduced 'status-only' remote"*; tracking mechanism from Codex review · **Approved by:** Cameron Temple · **Date:** 2026-07-29

**Context.** Revision 1.0 claimed the phone "can't fall behind" and that all four interfaces "do the same things." **Those claims are too strong.** A single API eliminates duplicated *engine* logic; it does not build an iOS screen. Someone must still implement each surface.

Given that MSC 1's iOS parity gap was itself a months-long project, overclaiming here risks repeating exactly the mistake MSC 2 is meant to prevent.

**Decision.** Parity is tracked, not asserted. MSC 2 maintains a capability matrix with one row per capability:

```
MSC 1 capability → MSC 2 agent operation → Desktop/Web → iOS → CLI
```

Every cell is Implemented, Planned, or **Intentional exception**.

**Full iOS capability is the owner's requirement, not a stretch goal.** The exception path covers behavior that is meaningless or impossible on a platform — revealing a file in Finder from a phone — and is **not** a route for skipping a hard iOS screen.

- An Intentional exception **requires owner approval** and becomes its own decision entry.
- "Difficult on a small screen" is not a valid reason; reshaping the workflow for mobile is the expected answer.
- Exceptions are re-reviewed each release rather than inherited.

**Restated guarantee, accurately.** MSC 2 guarantees that *no capability is architecturally unavailable to a client* — the API exposes everything the agent can do. It does not guarantee that every client has shipped every screen. The matrix is where the difference is visible.

---

## D-024 — Power management has two policies, by host role

**Status:** Proposed · **Origin:** Codex review · **Approved by:** —

**Context.** Revision 1.2 carried a single rule — *the host must not sleep while a server is running* — while the product document's day-in-the-life story has the owner **starting a stopped server** from a phone, on a closed MacBook. That only works if the host stays awake and reachable when nothing is running. The two documents contradicted each other.

**Decision.** Two policies, selected by an explicit per-host role setting rather than inferred:

| Host role | Policy |
|---|---|
| **Dedicated / headless host** with remote management enabled | Prevent sleep **whenever remote management is enabled**, running or not. Being reachable is the machine's purpose. |
| **Normal desktop** | Prevent sleep only while servers or critical operations are running. MSC does not hold a personal machine awake for nothing. |

**Detection and warning.** MSC identifies configuration that would defeat the policy — clamshell/lid-close suspend, aggressive sleep timers, hibernation, network interfaces that drop on sleep, platform power settings that override application inhibition — and warns **before** a user relies on remote start, not after it fails silently.

**Consequences.** Host role becomes part of host configuration and must be surfaced during setup. The macOS closed-lid case specifically needs verification: preventing sleep and staying network-reachable with the lid shut has real platform constraints.

**Revisit if:** the dedicated-host policy proves unacceptable on battery-powered hardware.

---

## D-025 — Service identity and privilege boundaries

**Status:** **Open** · **Origin:** Codex review · **Approved by:** —

**Context.** A macOS LaunchDaemon, a Windows Service, and a `systemd` service all run **outside the logged-in user's session**. MSC 1 has never faced this — it is a user-session GUI application, so its files, its Keychain items, and its processes all belong to the user running it. Nothing in the audit corpus answers what changes when the agent is a system service.

This is unresolved design work with wide blast radius, recorded as Open rather than papered over.

**Questions that must be answered before the substrate is built:**

1. **Which OS account runs the agent?** A dedicated service account, `root`/`SYSTEM`, or the installing user? Each has different consequences for file ownership and attack surface.
2. **Who owns server directories?** If the agent runs as a service account but a desktop user needs to open, edit, or back up those files directly, ownership and group membership must be deliberate — not an accident of who ran the installer.
3. **When is privilege escalation permitted?** Installing the service and binding privileged ports are plausible cases. Routine operation should require none.
4. **Machine-scoped secret storage.** A LaunchDaemon cannot reach a user's login Keychain; Windows DPAPI has user versus machine scope. The `SecretStore` trait (§8) must specify which scope it uses on each platform, and what that means if the machine is compromised.
5. **How does a desktop user grant the agent access to files?** On macOS this includes TCC — a daemon touching a user's Documents or an external volume triggers consent flows that have no obvious UI when there is no GUI.
6. **How do updates cross the privilege boundary?** A user-space application updating a system service is exactly the pattern that produces privilege-escalation vulnerabilities. The update path must be explicit.

**Why this is Open, not Proposed.** Answering it requires platform-specific investigation that has not been done. Guessing now would produce a decision with no evidence behind it, which is the failure this register exists to prevent.

**Blocks:** Phase 3 (safety substrate) and the D-012 authentication design, which assumes a local-authorization story that depends on these answers.

---

## Appendix A — corrections made during planning

Recorded because each produced a confident wrong answer, and each is the kind of mistake likely to recur.

1. **"The Remote API is read-mostly."** False — 49 POST routes. The claim came from a stale memory written before the API-expansion work landed. *Verify API surface by measurement.*

2. **"Tauri needs Rust, therefore the engine is Rust."** A non-sequitur. The shell is a few hundred lines; the engine's language is independent. It landed on Rust anyway, for a different reason.

3. **"Import counts reveal entanglement."** Comment text inflated every reference count in the first pass — headers reading *"Pure service — no AppViewModel dependency"* registered as coupling. *Strip comments before any classification.*

4. **"The `AppViewModel` extensions use it as a namespace."** Wrong. `@Published` reference counts badly understated coupling. Counting distinct symbols each extension uses that are declared in *other* `AppViewModel` files gives a family mean of **27.6**, with `AppViewModel+ServerControls.swift` at **109**.

5. **"Whole-file classification is sufficient."** The most consequential error. It placed `OverviewChatCardView.swift` in the discard bucket — a file containing a complete console chat/advancement/join/leave parser under a SwiftUI card. Five such files were found. *Files require symbol-level disposition before deletion.*

6. **"The router subsystem is all static data."** Over-generalized from the one file that is data. The matcher, fallback resolver, composer, and troubleshooting engine are executable behavior.

7. **"MSC 1 has admin/guest access only."** Wrong — named tokens with per-category permissions and expiry already exist (D-019). Caused by trusting a stale memory instead of reading the code.

8. **"No cloud service, ever."** Imprecise to the point of being false — MSC integrates Tailscale, Playit, DuckDNS, Modrinth, CurseForge, Adoptium, and Xbox services. The correct principle is *no TempleTech-hosted backend* (D-015).

9. **Marking analysis-derived proposals as "Settled."** Revision 1.0 gave owner-confirmed decisions and Claude-generated recommendations the same status label. Fixed in 1.1 by adding Origin / Approved by / Approval date to every entry.

## Appendix B — revision history

| Rev | Date | Change |
|---|---|---|
| 1.0 | 2026-07-29 | Initial register, 20 entries. |
| 1.3 | 2026-07-29 | Third Codex review: symbol-ledger contradiction removed everywhere; Phase 0 scoped to what MSC 1 can demonstrate and split from per-domain characterization; product permissions description corrected to name scoped tokens; D-024 (power management, two policies) and D-025 (service identity and privilege boundaries, Open) added; unmeasured memory figure and the "can never drift" claim softened. |
| 1.2 | 2026-07-29 | Second Codex review: D-011/D-021/D-023 promoted to Approved with `msc2.md` as origin; D-006 restated as baseline + extension + correction; symbol-ledger overclaim corrected; LaunchDaemon and headless sidecar specified; capability-exception path tightened. |
| 1.1 | 2026-07-29 | Codex review incorporated. Added Origin / Approved by / Approval date to all entries and split *Approved* from *Proposed*. Corrected D-019 (MSC 1's permission model is richer than stated). Expanded D-011 to tri-platform headless. Expanded D-012 with the six unspecified auth areas. Corrected the "no cloud" principle in D-015. Removed sequencing from D-016 to `msc2-port-plan.md` and relaxed the extraction rule. Added D-021 (resource efficiency), D-022 (separate compatibility matrices), D-023 (capability matrix). |
