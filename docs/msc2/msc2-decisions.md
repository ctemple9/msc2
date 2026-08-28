# MSC 2 — Decision Register

**Revision:** 1.7 · **Date:** 2026-08-28
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
| D-026 | Educational content is served data, not client code | **Approved** (requirement) / Proposed (mechanism) | 2026-07-30 |
| D-027 | The CurseForge manual-download workflow has no home once agent and client are different machines | Open | — |
| D-028 | Bedrock macOS support is Intel-only for Phase 10; Apple Silicon is deferred | **Approved** | 2026-08-22 |
| D-029 | Reset this client is separate from reset this host | **Approved** | 2026-08-28 |

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

**Addendum, 2026-08-18 (Phase 7, P7.1):** Cameron answered "Questions before P7.1" in `rolling-plan.md` — MSC 2 installs Java itself rather than only detecting and reporting it. `POST /v1/java-runtimes/install` is added under this decision's "extension" clause: an additive superset route, not a change to any preserved baseline behavior. Full reasoning and consequences for the Phase 7 step list in `docs/msc2/families/phase7-scope.md`, "Java runtime install: Cameron's answer."

---

## D-007 — macOS Bedrock stays Swift behind a sidecar

**Status:** **Approved** · **Origin:** Claude + Codex audits · **Approved by:** Cameron Temple · **Date:** 2026-08-22

**Context.** Bedrock Dedicated Server has no macOS build. MSC 1 solves this with `VMBedrockServerBackend.swift` (451 lines) driving `Virtualization.framework`, verified working with a real device joining on 2026-06-30.

**Decision.** The Swift VZ implementation is retained as a macOS-only sidecar binary supervised by the Rust agent over a narrow process protocol. It is not rewritten in Rust.

**Rationale.** `Virtualization.framework` is bridgeable from Rust via `objc2`, so this is engineering judgement rather than impossibility. It is a delegate-heavy framework with an async VM lifecycle; hand-rolled bridging of working, proven code is high risk for no early product value.

**Approval note (2026-08-22).** Confirmed with the owner directly, weighing the Rust-bridge alternative concretely: hand-declaring the `VZVirtualMachineDelegate` protocol from Rust via `objc2`, replicating the framework's one-queue-only access rule without Swift's `DispatchQueue` ergonomics, re-discovering undocumented quirks MSC 1 already paid for once (the `Pipe`-not-a-plain-file requirement noted in `VMBedrockServerBackend.swift`'s own comments), and bridging async completion blocks by hand — all with no compiler-enforced safety net at the `objc2` FFI boundary, unlike Swift calling this framework directly. Given the owner is still learning Rust, that failure mode (a runtime crash inside a closed-source framework, no Swift source to step through) is a worse trade than the sidecar's IPC surface. Confirmed sound; approved as originally proposed.

**Sequencing constraint.** The shared runtime contract is proven on native Linux first, then Windows, then the sidecar — so macOS-specific assumptions cannot leak into the contract.

**Revisit if:** the sidecar IPC proves more troublesome than the ObjC bridge would have been.

**See also:** D-028 scopes this sidecar's Phase 10 delivery to Intel Macs only.

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

**Phase 3 addendum (P3.2).** "A minimal Debian install" (above) didn't pin a release. P3.2's choice of `systemd-creds` for Linux secret storage (§8) requires `systemd` ≥ 250, which Debian 11 "bullseye" (systemd 247) doesn't have. **Confirmed by Cameron Temple, 2026-08-01: MSC 2's Linux minimum is Debian 12 "bookworm"** (systemd 252) or any distribution with `systemd` ≥ 250 — over building a weaker root-owned-file fallback to also cover older releases. Full reasoning in `docs/msc2/substrate/secret-storage.md`.

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

**Phase 2 scope (P2.3).** Phase 2's own gate (`msc2-port-plan.md` §3) needs only "the existing iOS app connects and reads status against a stub agent" — one client, one loopback transport, no real mutation behind it. Read against MSC 1's actual mechanism (`RemoteAPIServer+HTTP.swift`'s bearer lookup, `MSCSettingsView.swift`'s `mscremote://pair` deep link, `KeychainTokenStore.swift`), the token in that flow is not the product of a cryptographic pairing exchange — it's the same string created on the Mac side and embedded directly in the QR/link. What Phase 2 genuinely lacks is the token-issuance and persistent-storage machinery around it, which needs the `SecretStore` trait (Phase 3) and does not exist yet. Phase 2 therefore implements bearer-token *verification* only — a single fixed dev token from an environment variable, checked by `msc-agent`'s middleware, clearly commented as a placeholder — and points the re-hosted iOS client at it directly rather than through a real pairing flow. None of the six numbered gaps above are closed by this: items 2–6 don't apply to a loopback-only, iOS-only dev loop, and item 1 (local automatic authorization) is untouched either way. Full scoping and source citations in `docs/msc2/api-contract/auth-scope-phase2.md`.

**Phase 4 scope (P4.2).** Phase 4's Java lifecycle slice mutates a real imported Paper server from the CLI and existing iOS app, so P2.3's `MSC_DEV_TOKEN` stand-in is retired before those routes accept real mutation. The scoped design in `docs/msc2/lifecycle/pairing-phase4.md` preserves MSC 1's named-token model (admin, guest, named tokens with permission categories and optional expiry), stores server-side token verifier records in `SecretStore` under `remote-api.token.<credential-id>`, uses bearer tokens shaped as `msc2_<credential-id>_<secret>` so lookup does not require listing the secret store, stores only a hash/verifier rather than the raw bearer token, and keeps client-side tokens under per-host keys (`client.host-token.<agent-host-id>` for the CLI, `host-token.<agent-host-id>` in the iOS Keychain). Pairing for this phase is CLI-admin-created and iOS-exchanged: an already-authenticated admin CLI command creates a short-lived one-use pairing challenge, iOS exchanges it for a durable bearer token, and the challenge is immediately invalidated. Auth failures keep MSC 1's rate limit shape (10 failures per 60 seconds from one IP, then 429), sensitive POSTs keep MSC 1's 10-per-5-seconds shape, and audit records attribute auth failures, forbidden/rate-limited requests, token creation/revocation, and lifecycle mutations. P4.2 also records the P2.20 copied-iOS bug fix requirement: a missing Keychain item must mean "not paired," not an empty token that bypasses fallback behavior. This closes only the CLI/iOS credential path needed for Phase 4; local Tauri automatic authorization, remote desktop pairing, LAN TLS, Tailscale posture, browser cookies, origin policy, CSP, and CSRF remain open D-012 work.

**Phase 9 access-posture addendum (P9.3).** **Approved by Cameron Temple,
2026-08-22:** Phase 9 keeps management loopback-only by default and permits
only an explicit Tailscale management path; it does not permit a general-LAN
management bind, off-loopback HTTP, or TLS certificate provisioning. Tailscale
membership never replaces bearer authentication and permission checks. The
Phase 4 per-host CLI/iOS credential storage is retained, and Phase 9 adds
durable named-token administration, but remote desktop pairing, desktop-local
automatic authorization, browser cookie issuance, allowed origins/CSP, and
CSRF are deferred to Phase 11. The required invariant is that an unconfigured
agent accepts management traffic only on loopback; with Tailscale configured,
only that path may reach management and every request remains bearer-authenticated.
Player-facing listeners never provide a management path.

**Phase 11 contract addendum (P11.21).** The proposed implementation contract
is now frozen in `docs/msc2/clients/phase11-auth.md` and its additive `/v1`
surface is checked by `crates/msc-api/tests/phase11_auth_conformance.rs`.
It resolves the six design gaps without changing the approved transport
boundary: local desktop bootstrap is a package-identity- and installation-key-
bound local-IPC proof, never an open loopback exception; remote desktop pairing
returns a one-time bearer credential only to the Tauri backend, which stores it
under `msc.desktop.host-token.<agent-host-id>`; browser pairing exchanges a
one-use code for a durable, revocable httpOnly `SameSite=Strict` session; exact
same-origin checks, a restrictive CSP, and `X-MSC-CSRF` protect cookie-authenticated
mutations; and bearer requests remain CSRF-exempt. The contract deliberately
does **not** enable general-LAN management, certificate provisioning, a local
CA, or browser certificate bypass. This is a proposed technical resolution,
not a retroactive owner approval of the previously Proposed D-012 details.

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

**Validation against all 88 baseline routes (P2.1).** Read directly from `RemoteAPIServer+HTTP.swift`'s `respond(to:clientFD:)` — the `adminOnlyPOSTPaths` set, the `pathPermissions` map, and the three hardcoded `guard case .admin = requestRole` checks (`GET /files`, `GET /files/read`, `GET /users`) — against every path in `docs/msc2/api-baseline/openapi.json`. Full per-route table in `docs/msc2/api-contract/permission-vocabulary.csv` (88 rows). Findings:

1. **The four-category list in this entry's own "Verified in MSC 1" block was incomplete.** MSC 1 enforces **eight** named-token permission categories on POST routes, not four: `serverControl` (4 routes — `/start`, `/stop`, `/command`, `/active-server`), `players` (3), `settings` (7), `addons` (8 — this entry said `mods`; the code's actual string is `addons`, covering both components *and* resource packs), `worlds` (7), `broadcast` (7), `networking` (2 — playit start/stop), `fleet` (6 — server CRUD + templates).
2. **A ninth bucket is real but isn't a named-token permission at all.** Six routes are gated to the `admin` token role directly, bypassing the named-token permission map entirely: `POST /users`, `POST /users/revoke`, `POST /users/update` (admin-only by omission — in `adminOnlyPOSTPaths` but absent from `pathPermissions`, so no permission string can ever unlock them for a named token) plus `GET /users`, `GET /files`, `GET /files/read` (admin-only via an explicit role guard ahead of the normal GET-is-always-allowed rule). Recommend formalizing this as a real fifth category, `admin`, rather than leaving it as an implicit "absent from the map" convention — the implicit form is exactly what let this entry undercount in the first place.
3. **One correction to this step's own starting premise:** `/health/repair` does **not** need a new bucket — it already carries `settings` in `pathPermissions`. The `admin`-only routes are specifically the user-management and file-browser paths, not health.
4. **A likely-unintentional gap, not a vocabulary question:** `POST /watchdog/enable` and `POST /watchdog/disable` sit in neither `adminOnlyPOSTPaths` nor `pathPermissions` — any authenticated token, including a guest, can toggle the watchdog today. Flagged for Cameron; not fixed here (D-019 is about the vocabulary, not about re-gating a specific route — that's a product/security call, not this step's to make).
5. **38 of the 88 routes require no permission category at all** — every GET except the three `admin`-gated ones, plus the two watchdog POSTs — accessible to any authenticated token today.

**Revised decision (still Proposed, pending Cameron's confirmation — not promoted to Approved by this step):** the formal MSC 2 vocabulary should be the nine buckets above — `serverControl`, `players`, `settings`, `addons`, `worlds`, `broadcast`, `networking`, `fleet`, `admin` — carrying MSC 1's `mods` category forward as `addons` to match the code, and making the previously-implicit "absent from the permission map" admin gate an explicit category instead.

**P2.8 addendum — the watchdog gap (finding 4) is resolved.** `POST /v1/watchdog/enable` and `POST /v1/watchdog/disable` are assigned the `settings` category in `docs/msc2/api-contract/openapi.json`. Reasoning: watchdog toggles host-level auto-recovery of the whole agent process (`WatchdogRunner.swift` — a launchd relaunch mechanism for the MSC application itself, not a per-Minecraft-server action), the same operational-config flavor as `/config/ram`, `/config/java-runtime`, and `/health/repair` — all already `settings` — rather than `serverControl` (which gates actions against a specific running Minecraft server) or `admin` (which MSC 1 reserves for identity/file-access routes it denies outright even to permission-granted named tokens, not general host config). `permission-vocabulary.csv` itself is left unchanged — it is P2.1's factual record of MSC 1's actual (gapless) baseline behavior, not the v1 decision. Still Proposed, pending Cameron's confirmation, per this phase's pattern for judgment calls.

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

**Status:** **Approved** · **Origin:** Codex review · **Approved by:** Cameron Temple · **Date:** 2026-08-22

**Context.** Revision 1.0 said "native Linux Bedrock," which implies that any Linux distribution MSC supports is automatically a supported Bedrock Dedicated Server environment. That does not follow. BDS ships with its own distribution and library expectations, which are narrower than the set of systems on which the MSC agent can run.

**Decision.** MSC 2 publishes **three separate compatibility matrices**:

1. **MSC agent** — where the agent itself is supported (broad: macOS, Windows, mainstream Linux including minimal Debian).
2. **Java servers** — where Java Minecraft servers are supported (essentially wherever a supported JRE runs).
3. **Bedrock runtime** — where Bedrock Dedicated Server is supported, per backend: native Linux (specific distributions and library requirements), native Windows, and the macOS VZ sidecar (per D-028, Intel Macs only for now).

**Consequence.** A host may be a fully supported MSC agent host and a fully supported Java server host while not being a supported Bedrock host. The interface must say so plainly rather than failing at runtime.

**Approval note (2026-08-22).** Approved alongside D-007 — this decision only settles the reporting structure (three matrices, not one), which doesn't turn on anything Phase 10's implementation work would still need to discover.

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

**Phase 3 addendum (P3.3).** `msc2-port-plan.md` §3's own Phase 3 prose lists eight substrate items and doesn't name this one, but §4B's separate acceptance-test table placed "cross-platform sleep inhibition and the two power policies (D-024)" at "Phase 3" anyway — a contradiction between two sections of the same document, flagged rather than silently resolved by `docs/msc2/substrate/phase3-scope.md`. **Confirmed by Cameron Temple, 2026-08-01: D-024 lands in Phase 4**, not Phase 3, alongside real service registration — its verification needs live OS power APIs (`IOPMAssertion`/`SetThreadExecutionState`/`systemd-inhibit`), not the fixture-comparison shape every other Phase 3 item uses, and its purpose (remote-starting a stopped server) isn't attemptable until Phase 4's real service lifecycle exists.

---

## D-025 — Service identity and privilege boundaries

**Status:** **Confirmed** (questions 1, 2, 3, 6, the Windows/Linux half of question 4, and the macOS System-keychain default for question 4) by Cameron Temple, 2026-08-01 · **Open** (the underlying macOS login-vs-System-keychain reachability question; question 5 — TCC) · **Origin:** Codex review · **Approved by:** —

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

**Phase 3 scope (P3.1).** Answered as far as reading each platform's own documented service-account behavior responsibly allows — recommended direction, **confirmed by Cameron Temple, 2026-08-01**: on all three platforms, **the agent runs as the account that installed it** by default (macOS `LaunchDaemon` `UserName`, Windows Service "Log on as", Linux `systemd` `User=`/`Group=` — all pointed at the installing user, not `root`/`SYSTEM`/a dedicated account). This answers question 2 by construction (server-directory ownership matches the installing user, so no ACL dance for a desktop user opening those files directly) and question 3 (routine operation needs no escalation; only writing the daemon/service/unit definition at install time does, gated by the OS's own installer-elevation prompt). Question 6 (update crossing the privilege boundary) splits into binary updates, which follow install location, and daemon/service/unit-definition updates, which need the same elevation installation did. A dedicated-service-account mode is named as a deferred v1.1 option for a true multi-admin host, not built now — D-011's own rationale is a single-owner machine.

Question 4 (machine-scoped secret storage) is answered for Windows (DPAPI user-scope) and Linux (`systemd-creds`, per P3.2). For macOS, whether a `LaunchDaemon` with `UserName` set to a real user can actually reach that user's *login* Keychain is not answerable from this audit corpus (MSC 1 has never run as a daemon) and **stays genuinely Open** — not guessed at here. **Cameron Temple confirmed, 2026-08-01, that P3.9 should design the macOS `SecretStore` implementation against the System keychain** (machine-scoped, not session-scoped) now, rather than block on a live LaunchDaemon test that can't happen until Phase 4; documented plainly as such, revisited if Phase 4's real `LaunchDaemon` shows login-keychain access does work instead. This confirmation unblocks P3.9 — it does not close the underlying platform question. Question 5 (TCC) is likewise recorded as unverifiable from docs alone, deferred to a real Phase 4 `LaunchDaemon` test. Full reasoning and the per-question breakdown in `docs/msc2/substrate/service-identity.md`.

**Amendment, 2026-08-01 (P3.11 implementation finding):** the Linux half of question 4, above, said `systemd-creds` answers it — building P3.11 against that found it doesn't, for a reason deeper than provisioning: both encrypting *and* decrypting via `systemd-creds` require root on any machine without a TPM2 chip, which is not an edge case (it includes this project's own CI). **Cameron Temple confirmed, 2026-08-01: the real target is a small privileged helper, deferred to build alongside Phase 4's real service registration; P3.11 ships an explicitly-labeled v1 stand-in instead** — a file encrypted with a key owned by the agent's own installing-user account, needing no root at any point. Full finding, evidence, and the two-track decision in `docs/msc2/substrate/secret-storage.md` §12.

**Phase 4 implementation decision (P4.3, 2026-08-02):** build the Linux
privileged credential helper in P4.23, alongside real `systemd` service
registration. The helper is a hidden mode of the same `msc` binary, installed
during the same elevated window that writes the service units, reached by the
unprivileged agent over a Unix socket restricted to the installing user's UID,
and is the only Linux component that touches `systemd-creds` or the root-owned
encrypted credential blobs. The P3.11 file-based `LinuxSecretStore` remains for
development/tests/non-service runs, but is not the accepted backend for the
Phase 4 Linux headless-service gate. Full design:
`docs/msc2/lifecycle/linux-credential-helper.md`.

**P4.40 credential amendment, 2026-08-12:** the Phase 4 credential record is
amended without changing the owner-approved service identity or macOS
System-keychain target. Review found two implementation gaps. First, production
`msc serve` still constructs `AuthState` with `FakeSecretStore`, so P4.5 proved
the bearer-token model and registry shape, not durable platform credential
storage in the installed service. Second, P4.3 selected the Linux privileged
helper, but the Phase 4 implementation did not yet provide the callable helper
server/client path that the installed units name. The approved target remains:
installing-user service identity, install-time-only elevation for privileged
setup, macOS System-keychain material as the machine-scoped credential root
unless live LaunchDaemon evidence proves a simpler path, Windows Credential
Manager under the service account, and Linux helper-backed service storage.
P4.41-P4.43 must prove those pieces before the credential portion of Phase 4 is
closed.

---

## D-026 — Educational content is served data, not client code

**Status:** **Approved** (that MSC 2 teaches, everywhere) · **Proposed** (serving it as data)
**Origin:** Owner — `msc2.md`: *"The Server Handbook remains available inside every interface. Help is contextual… The application teaches without forcing the user to leave the interface and search for terminology."* Mechanism raised by the owner 2026-07-30. · **Approved by:** Cameron Temple · **Date:** 2026-07-30

**Context.** MSC 1's teaching material is one of its largest and most distinctive assets — a 31-topic Server Handbook across 6 categories (each with a "think of it like this" analogy), a concept guide with diagrams, an onboarding tour over the live UI, ~18 files of router port-forwarding guides with brand matching and a troubleshooting decision tree, and contextual help sheets throughout. An external review named it *"the product's personality… rare for a hobby-scale project."*

**The problem it currently has.** All of it is Swift compiled into the macOS app. iOS could not use any of it, so a second educational surface was written — `QuickGuideView`, 706 lines — with its own separate content. That is precisely the duplication MSC 2 exists to remove, applied to prose instead of logic. It also means correcting a typo in a handbook topic requires an App Store release.

**Decision.** Educational content is **structured data owned by the agent and served over the API**, not code compiled into any client.

1. **Handbook topics, concept explanations, and guide content** live as structured content files in the repo, served by the agent. Clients render; they do not author.
2. **Every explainable thing carries a `helpId`.** Settings fields, health cards, diagnostics, performance metrics, and connection methods each reference their explanation. Clients resolve the ID through the API rather than wiring help to a screen.
3. **Router guides keep their rule engine.** The catalog, router records, and step content are data; the matcher, fallback resolver, composer, and troubleshooting decision tree are executable behavior and are translated to Rust (see the port plan).
4. **The onboarding tour splits.** Step content and ordering are data; only the *anchoring* to specific UI elements is client-side, because it is inherently per-client.
5. **The CLI is a first-class consumer.** `msc explain <topic>` renders the same content as the handbook. Any surface that can display text can teach.

**Rationale.** Write a topic once, and it appears on desktop, web, phone, and terminal. Content updates ship with the agent rather than with four separate clients. And a new setting arrives with its explanation already attached on every surface — which is the same leverage the schema-driven settings contract already demonstrated when Bedrock settings reached iOS with zero iOS changes.

**Consequences.**
- The content model and `helpId` must be in the API contract **before Phase 2 freezes it.** Retrofitting a help pointer onto every DTO afterwards is far more expensive than including it now.
- Content becomes reviewable and diffable in git rather than buried in view code.
- Localization, currently a declined non-goal, becomes tractable later without touching any client.
- Someone must decide the content format (Markdown with front-matter is the obvious candidate) and whether content is embedded in the agent binary or read from disk — the latter permits updates without a release, the former guarantees it is always present.

**Open.** Whether the concept-guide diagrams are assets or generated · how content is versioned against API versions when a topic describes a feature an older client lacks. (Content format and embedded-vs-on-disk, formerly open here, were confirmed 2026-07-31 — see the Phase 2 addendum below.)

**Revisit if:** the content model proves too rigid for the router guides, which are the most structurally complex educational surface and the natural stress test.

**Phase 2 addendum (P2.2).** The two mechanism questions this entry left open — content format, and embedded vs on-disk — are now confirmed, not just recommended: Markdown with YAML front-matter, embedded in the `msc-agent` binary for v1 (on-disk override deferred, not foreclosed). **Confirmed by:** Cameron Temple · **Date:** 2026-07-31. Also confirmed: `SettingFieldDTO`'s existing free-text `help` field is *replaced* by `helpId` in the v1 contract, not kept alongside it. Full reasoning, and the precise `helpId` shape (`<namespace>.<name>`, resolved via `GET /v1/help/{helpId}`) with every DTO field from §18's list mapped to a concrete MSC 1 field, is in `docs/msc2/api-contract/helpid-contract.md`. Diagram format and cross-version content degradation remain genuinely open — not addressed by this addendum.

**P2.8 addendum.** `helpid-contract.md` §4 left one open item for this step: how `PerformanceSnapshotDTO`'s bare scalar fields (`tps1m`, `cpuPercent`, `ramUsedMB`, `ramMaxMB`, `worldSizeMB`) carry a `helpId`, since unlike the other four DTO categories they have no natural sub-object to attach one to. Resolved as option (b): each field wraps into `{ value, helpId }` (`PerformanceMetricNumberDTO` in `docs/msc2/api-contract/openapi.json`), not option (a) (a separate static client-side metric-name-to-`helpId` map). Reasoning: every other `helpId`-bearing field in this contract attaches the pointer directly on the field's own object; a separate static map would be a second place that mapping has to be kept in sync, which is the exact duplication-and-drift failure D-026 exists to eliminate — this trades a DTO-shape change (bare scalar to object) for staying consistent with that principle. `ramUsedMB` and `ramMaxMB` both resolve to the same `performance.ram` topic; `playersOnline`, `serverType`, and `ts` are left as bare scalars — none of the three has a natural help topic. Still Proposed, pending Cameron's confirmation, per this phase's pattern for judgment calls.

---

## D-027 — The CurseForge manual-download workflow has no home once agent and client are different machines

**Status:** **Approved**, option 1 (client-side download, then explicit upload to the agent) · **Origin:** Symbol-ledger audit (P0.27, flagged UNSURE; formalized in P0.31) · **Approved by:** Cameron Temple · **Date:** 2026-08-21

**Context.** Some CurseForge mods disable third-party API distribution, so MSC 1 can't download them itself. `CurseForgeManualDownloadSheet.swift` handles this by opening each mod's direct download page in the user's default browser (correct loader/version pre-selected), then watching a local folder — `~/Downloads` by default, user-changeable — for newly-appeared files. It matches new files against the pending list by exact filename, then by macOS's " (1)" duplicate-suffix pattern, then by a last-resort single-remaining-file/single-new-file fallback, and moves each match straight into the server's `mods/` directory.

Every step of that sequence — the browser, the watched folder, and the server's `mods/` directory — is required to be the same machine MSC 1 is running on. That has always been true for MSC 1, a single-Mac GUI app. It is not guaranteed for MSC 2: D-011 makes headless independently installable on all three platforms, which means the agent can be a box with no browser and no relationship to whatever machine the user's browser runs on. The deletion test (`msc2-port-plan.md` §1) doesn't resolve this the way it resolves most Mixed-bucket behavior, because the question isn't which side — agent or client — already owns the behavior; it's that the behavior as written doesn't have a coherent home on either side once they can be different computers.

**Open — options, not a decision:**

1. **Client-side download, then explicit upload to the agent.** The client (not the agent) drives the browser and the folder watch, since it's the client's machine that has both — then pushes the finished file to the agent over the API (a new upload route; `POST /components/install` and friends install from something the agent can already reach, not from an arbitrary local file). Closest behavioral match to MSC 1, but only works when the client happens to be a desktop, and adds a file-upload code path that doesn't exist anywhere in the API baseline today.
2. **Agent-side fetch, if CurseForge ever exposes a directly fetchable URL for the specific case that's blocked.** Would keep the workflow entirely server-side like every other component install, but depends on CurseForge's distribution restriction having a loophole — unconfirmed, and probably mod-author-specific rather than something MSC can rely on.
3. **Degrade gracefully to same-machine-only.** Keep something like today's mechanism, but explicitly scoped to "works when your client and your agent are the same box" (e.g. a local desktop client managing a `localhost` agent), and tell the user plainly when it isn't available otherwise. Simplest to build; narrows a capability MSC 1 users have today.
4. **Do nothing special — tell the user to place the file manually.** The agent exposes a "drop the file at this server-visible path" instruction (relevant given `GET /files` already exists as an admin-only server-side file browser, P0.30) and the user is responsible for getting it there by whatever means their setup allows (SFTP, a shared folder, physically local). Least engineering, worst experience, and pushes a solved-for-MSC-1 problem back onto the user.

**Why this was Open, not Proposed, before Phase 8.** It's a product-shape decision — how much of this convenience MSC 2 keeps, and for which client/agent topologies — not something that falls out of reading MSC 1's code or applying the deletion test. It surfaced while doing ledger bookkeeping (P0.27/P0.31), which is exactly the kind of call that work should record and hand up, not make quietly.

**Decision, 2026-08-21 (P8.1).** Cameron chose **option 1**: the client (not the agent) downloads the blocked file and uploads it through MSC's bounded staged-upload path (`POST /v1/staged-uploads`, Phase 6); the agent verifies the expected file identity against the pending pack operation and resumes it. This is the closest behavioral match to MSC 1's own convenience (open the file's page, get it onto the right machine, don't make the user hunt for a manual path) while working uniformly whether the client is a phone, a laptop, or a headless CLI against a remote agent — it does not require the client and agent to share a filesystem the way MSC 1's Downloads-folder watch does. Consequences for Phase 8's step list are recorded in `docs/msc2/addons/phase8-scope.md`'s "D-027: the CurseForge manual-download workflow, decided" section — in short: one or more new purpose-bound `StagedUploadPurposeDto` cases (P8.9), each upload bound to its own pending operation, expected file identity, and a one-use size ceiling (P8.20), never a general arbitrary-path upload.

**Revisit if:** a future client/agent topology makes even a client-side download infeasible (e.g., a CLI-only client with no browser of its own) — not expected to arise in v1's supported clients (desktop/web, iOS, CLI all run somewhere with a browser reachable to the user).

---

## D-028 — Bedrock macOS support is Intel-only for Phase 10; Apple Silicon is deferred

**Status:** Approved · **Origin:** Owner, surfaced during the Phase 10 cross-check · **Approved by:** Cameron Temple · **Date:** 2026-08-22

**Context.** `VMBedrockServerBackend.swift` bundles exactly one kernel/initramfs pair (`vmlinuz-kata` / `appliance-initramfs.gz`) with no architecture branching anywhere in the file — contrast `JavaInstaller.swift:74-77`, which does branch on `#if arch(arm64)` for its own downloads. Bedrock Dedicated Server ships only as an x86_64 Linux binary, so that bundled appliance is x86_64. `Virtualization.framework` does not emulate a foreign CPU architecture: on an Apple Silicon host it can only boot an arm64 guest kernel — an x86_64 kernel simply will not boot, not "boots slowly" or "boots untested." Running BDS in a VM on Apple Silicon would require a new arm64 kernel/initramfs appliance plus Apple's Rosetta-for-Linux (`VZLinuxRosettaDirectoryShare`, macOS 13+) shared into that guest so its userspace can translate the x86_64 `bedrock_server` binary. MSC 1 has none of this — no Rosetta-for-Linux code anywhere in the Bedrock backend. The owner has no Apple Silicon hardware to build or validate that path.

**Decision.** Phase 10's macOS Bedrock backend (D-007's Swift sidecar) is built, verified, and published as **Intel-only**. Apple Silicon Mac support for Bedrock is explicitly deferred, not silently unsupported: the D-022 compatibility matrix records it as **unavailable — no test hardware**, distinct from "unsupported" and never simply omitted. The `BedrockRuntime` trait and the sidecar protocol boundary must stay free of any x86_64-specific assumption that would need revisiting later — only the appliance and its packaging are Intel-only for now.

**Consequences.** No CI job, smoke test, or evidence step (P10.13/15/18/24/25) may claim Apple Silicon Bedrock support. `docs/msc2/bedrock/compatibility-matrix.csv` carries an explicit Apple Silicon row/cell distinct from the Intel Mac cell. Native Linux and Windows Bedrock support (D-007's sequencing) are unaffected — this decision is scoped to the macOS sidecar backend only.

**Revisit if:** the owner acquires Apple Silicon test hardware, or a contributor with such hardware can produce reproducible evidence for the arm64 appliance + Rosetta-for-Linux path.

---

## D-029 — Reset this client is separate from reset this host

**Status:** **Approved** · **Origin:** Owner · **Approved by:** Cameron Temple · **Date:** 2026-08-28

**Context.** MSC 2 stores two different kinds of state. A client stores its remembered hosts, credentials, preferences, and onboarding progress. The agent's host owns the server registry, host configuration, credentials, host identity, and managed server files. Treating both as one reset would make a client-only cleanup unexpectedly destructive, while treating a host reset as local client bookkeeping would leave a remote host unchanged when the owner intended to recover it.

**Decision.** MSC 2 has two explicit reset operations:

1. **Reset this client** is local-only. It clears the selected device's host records, credentials, preferences, and onboarding state. It makes no agent request and never changes a host.
2. **Reset this host** is an authenticated, host-owned, operation-backed agent action. It is available only to an administrator, refuses while any managed Minecraft server is running, requires an explicit host-specific confirmation, revokes credentials, rotates the host identity, and supports `configuration` (preserve managed Minecraft files) and `everything` (remove them too).

The HTTP route never installs or uninstalls an operating-system service. A local desktop may uninstall its own service after a successful full reset; a remote client must never control the service on the computer running the agent. After a reset, old credentials are invalid and remote recovery requires a new one-use pairing code created locally on the host. First-time setup may lead to first-server creation, but never creates a server silently.

**Rationale.** The boundary follows D-013's host-scoped state model and D-011's independently installable headless agent. It makes the destructive target visible, preserves server files when the owner only wants to clear MSC's configuration, and prevents a remote browser or desktop from becoming an operating-system service controller.

**Revisit if:** the host-owned agent state or local service boundary changes.

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
| 1.7 | 2026-08-28 | D-029 added: client-only reset is separate from authenticated host reset, with configuration-only/full-delete modes and local-only service teardown. |
| 1.6 | 2026-08-22 | D-007 and D-022 promoted to Approved, confirmed with the owner during the Phase 10 cross-check. D-028 added: Bedrock macOS support is Intel-only for Phase 10; Apple Silicon is deferred pending test hardware, recorded as "unavailable" in the D-022 matrix rather than omitted or claimed unsupported. |
| 1.5 | 2026-08-21 | D-027 moved from Open to Approved (P8.1): Cameron chose option 1 (client-side download, agent-side staged-upload verification) for the CurseForge manual-download workflow. Detail moved to `docs/msc2/addons/phase8-scope.md`. |
| 1.0 | 2026-07-29 | Initial register, 20 entries. |
| 1.4 | 2026-07-30 | Added D-026 (educational content is served data, not client code) after the owner identified that MSC 1's teaching material — its largest distinctive asset — had no home in the MSC 2 architecture. |
| 1.3 | 2026-07-29 | Third Codex review: symbol-ledger contradiction removed everywhere; Phase 0 scoped to what MSC 1 can demonstrate and split from per-domain characterization; product permissions description corrected to name scoped tokens; D-024 (power management, two policies) and D-025 (service identity and privilege boundaries, Open) added; unmeasured memory figure and the "can never drift" claim softened. |
| 1.2 | 2026-07-29 | Second Codex review: D-011/D-021/D-023 promoted to Approved with `msc2.md` as origin; D-006 restated as baseline + extension + correction; symbol-ledger overclaim corrected; LaunchDaemon and headless sidecar specified; capability-exception path tightened. |
| 1.1 | 2026-07-29 | Codex review incorporated. Added Origin / Approved by / Approval date to all entries and split *Approved* from *Proposed*. Corrected D-019 (MSC 1's permission model is richer than stated). Expanded D-011 to tri-platform headless. Expanded D-012 with the six unspecified auth areas. Corrected the "no cloud" principle in D-015. Removed sequencing from D-016 to `msc2-port-plan.md` and relaxed the extraction rule. Added D-021 (resource efficiency), D-022 (separate compatibility matrices), D-023 (capability matrix). |
