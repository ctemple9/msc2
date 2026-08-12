# Phase 5 scope: Configuration and migration

**Status:** P5.1 scoping note, amended by P5.33 after the Phase 5 gate review.
**Source of truth:** `msc2-port-plan.md` §3 (Phase 5 gate), `msc2-decisions.md`, `docs/msc2/rolling-plan.md` (Phase 5 section), and the Phase 0 symbol ledger.
**MSC 1 oracle:** `~/Documents/Swift Projects/minecraft-server-controller`, read-only.

This note fixes the boundaries for Phase 5 before code starts, in the same role `phase3-scope.md` and `phase4-scope.md` played for their phases. It does not approve new product behavior on its own; where a choice is still open, it records the recommended working answer and names what Cameron's approval changes.

## Working exit gate

The port plan states no separate Phase 5 exit criterion in `msc2-port-plan.md` beyond the phase gate itself, so `rolling-plan.md`'s own working gate is what this phase is held to:

- At least one sanitized, provenance-recorded `server_config_swift.json` file from a real MSC 1 install (two, from distinct schema eras, was the original bar — genuinely unavailable; see "Evidence required" below), and one real MSC 1-generated `.msctransfer` package, pass the Rust readers.
- The typed `AppConfig`/`ConfigServer` schema reproduces MSC 1's concrete defaulting, rename, malformed/unknown-field, duplicate-ID/path, shared-access normalization, and port-clamping behavior through the existing atomic config repository.
- Corrupt-backup discovery and merge work.
- The explicit legacy-secret migration handles only the plaintext owner token and per-server Xbox passwords MSC 1 actually migrates, under a documented Phase 4 credential-transition contract.
- `GET`/`POST /v1/settings` use the frozen multi-DTO contract and persist then re-read changes.
- MSC 1 transfer packages import end to end through the public API and CLI, with a successful export backup required before `replaceAll`.
- Java and Bedrock folders and ZIPs scan and import with loader, version, worlds, EULA, and settings labelled from evidence.
- Rescan registers untracked directories in place.
- The self-contained CLI smoke covers settings, transfer, and raw import.
- Fixtures pass in macOS, Linux, and Windows CI.
- The formal world-slot model remains Phase 6 — Phase 5 copies and labels world data without creating MSC 2 slots.

## Evidence required

Before any translation work in P5.4 onward, P5.3 must collect:

- At least **one sanitized `server_config_swift.json` file** from a real MSC 1 install, plus a provenance manifest recording source and sanitization. **Update (P5.3):** the original bar was two configs from distinct schema eras, but only one real config exists — Cameron checked other Application Support copies, local Time Machine snapshots, MSC 1's own git history (the config path is correctly gitignored there, since it's runtime data), and iCloud, and confirmed no second-era config survives anywhere. He approved relaxing this bar to one real config rather than inventing a second. The era-diversity coverage a second era would have exercised (defaulting, renamed keys, duplicate-ID/path handling) is carried instead by P5.4/P5.5's dedicated characterization fixtures, extracted directly from MSC 1's own test assertions — this corpus's remaining job, proving the Rust reader actually parses/round-trips a real production file end to end (P5.24/P5.25), is still meaningfully served by one real file. `real-corpus-check.py`'s inventory gate (P5.2) now requires at least one config file, not two.
- **Any real `.corrupt-*` backup** MSC 1 has actually produced, if one exists on Cameron's machines — used by the corruption-recovery steps (P5.6–P5.7). Not required to block P5.3 if genuinely unavailable, but not to be fabricated either.
- **One real MSC 1-generated `.msctransfer` package**, produced with MSC 1's own Export Servers function, supplied through a local environment path (`MSC2_PHASE5_TRANSFER_PACKAGE`) rather than committed to git, because it carries real world/server data. Its format version, source, size, and SHA-256 get recorded in `corpus/configs/README.md`.

Sanitization may replace secret values, absolute paths, addresses, and player identities, but must not change key presence, types, schema version, or nesting — otherwise the corpus stops being evidence of MSC 1's real defaulting/normalization behavior and becomes an invented fixture wearing evidence's clothes. `corpus/configs/README.md` currently records that this directory is empty; `real-corpus-check.py` (P5.2) is the dependency-free gate that fails loudly, not silently, if P5.3 can't produce this evidence before later steps proceed.

If this evidence is genuinely unavailable, the port plan's own instruction (carried into P5.3) is to stop rather than substitute invented fixtures for the required historical corpus.

## Transfer behavior

Pinned against `AppViewModel+ServerTransfer.swift` and `AppViewModel+APIWiringServerMgmt.swift` so later steps don't reinterpret it:

- **`excludedTopLevelDirs` (`AppViewModel+ServerTransfer.swift:41`) is a stale, unused constant.** It is declared once and referenced nowhere else in the file or the codebase. It does **not** suppress `exportServerTransfer`'s live-world export — export is unconditional over whatever's on disk under each server's directory. Do not port a filtering behavior that MSC 1 declares but never wires up.
- **`action == "scan"` is raw-directory scan only** (`RemoteAPIServer+ComponentRoutes.swift:205`, wired to `serverImportScanProvider` in `AppViewModel+APIWiringServerMgmt.swift:449`). It calls `scanExistingServerInfo` directly and never touches transfer-package inspection. Transfer packages are inspected only inside the **import** path (`serverImportProvider`), gated on `action == "importTransfer" || importKind == "transfer" || <ext> == .msctransfer` (`AppViewModel+APIWiringServerMgmt.swift:497-499`), not on the scan route.
- **The HTTP import handler — not `applyTransferImport` itself — owns the pre-`replaceAll` backup and the transfer inspection.** In `serverImportProvider` (`AppViewModel+APIWiringServerMgmt.swift:500-518`): when `transferMode == .replaceAll`, the handler requires a non-empty `backupPath`, calls `exportServerTransfer(to:)` itself, and fails the whole request with `backup_failed:` if that export fails — all **before** it calls `inspectTransferPackage` and `applyTransferImport`. Rust's route handler must reproduce this ordering and failure message shape, not push the backup responsibility down into the transfer-apply primitive.
- `merge` mode takes the same `inspectTransferPackage` → `applyTransferImport` path but skips the backup precondition entirely (`transferMode` only forces a backup when it resolves to `.replaceAll`; anything else, including an absent/unrecognized `transferMode`, defaults to `.merge` per line 501).
- **MSC 1's `replaceAll` wipes broadly, not narrowly** (`KeychainManager.swift:132-152`, called from `AppViewModel+ServerTransfer.swift:530` with `removedIDs = configManager.config.servers.map(\.id)` — every server configured *before* the replace, not just ones actually removed): `deleteAllMSCSecrets` deletes the owner's own Remote API token, the Remote API guest token, the playit secret key, the CurseForge API key, **and** every one of those pre-replace servers' Xbox broadcast alt passwords. It is a "reset this machine's MSC state" operation, not a scoped per-server cleanup — a `replaceAll` transfer import silently signs the owner's own Remote API token out too.
- **P5.16/17 (this port) narrows this in two user-visible ways, both flagged to Cameron during the Read move rather than decided silently:**
  1. **The replaced server set is narrower than MSC 1's.** `msc-agent` has no unified, persisted `AppConfig`/`ConfigServer` list yet — Phase 4's Paper-folder-import registry (`AgentServerRegistry` in `crates/msc-agent/src/routes/lifecycle.rs`) and this phase's transfer-imported-server list (`TransferServerStore` in `crates/msc-agent/src/routes/servers.rs`) are two separate, unconnected stores. A `replaceAll` transfer import backs up and replaces only the transfer-imported list. A server imported from a plain Paper folder (`action: "importExisting"`, no `importKind`) is untouched by a later `replaceAll` — not backed up, not wiped, not even inspected. Unifying the two into one real server list is follow-up work, not done in P5.16/17.
  2. **The secret wipe is not wired to anything real yet.** `deleteAllMSCSecrets`'s target — the owner's Remote API token — lives in a `SecretStore` owned by `AuthState` (`crates/msc-agent/src/auth.rs`), which this route has no reference to (neither `auth.rs` nor `main.rs` are in P5.16/17's file list). The route calls a `wipe_all_secrets` port on every successful `replaceAll`, proven by test to fire in the right order and never on `merge` or a failed backup, but its production implementation is a documented no-op today. No secrets are actually deleted by a Phase 5 `replaceAll` yet.

## Raw import boundary

Pinned against `AppViewModel+ConfigRecovery.swift` and `AppViewModel+ServerImport.swift`:

- **Rescan (`rescanAndImportServers`, `AppViewModel+ConfigRecovery.swift:103-183`) registers folders already under `configManager.serversRootURL` (and its `java`/`bedrock` subdirectories) in place.** It never copies, moves, or unzips anything — it walks one level of subdirectories not already present in `config.servers`, requires a `.jar` or a `bedrock_server`/`bedrock_server.exe` binary to accept a candidate, builds a `ConfigServer` pointing at the existing `dir.path` verbatim, and appends it to the config. This is a *second, separate* import path from the raw-directory import in `AppViewModel+ServerImport.swift`, which copies/unzips external sources into the MSC-owned root — rescan only ever discovers directories that are already inside that root.
- Rescan's `ConfigServer` defaults (`minRamGB: 2, maxRamGB: 4`, empty `notes`, `hasEverStarted = true`) and its Java-flavor/version/loader detection reuse `AppViewModel.detectJavaFlavor`, the same detector raw import uses — so P5.22's port should share code with P5.19/P5.20 rather than reimplement detection.
- Raw import (`AppViewModel+ServerImport.swift`, characterized fully in P5.18) is the copy/unzip-into-owned-root path, detects both Java and Bedrock, and is what P5.19–P5.21 port. It is out of scope for this section beyond noting the boundary: rescan is "notice what's already there," raw import is "bring something in from outside."

## Secret migration

Pinned against `ConfigManager.swift:68-107` (init-time one-time migration) and `ConfigManager.swift:184-250` (save/populate):

- **The migration reads exactly two things out of legacy plaintext JSON, both by literal key name:** a single top-level `"remote_api_token"` string (the app owner's own Remote API auth token — `KeychainManager.shared.writeRemoteAPIToken(oldToken)`), and, per server entry in the `"servers"` array, an optional `"xbox_broadcast_alt_password"` string (`KeychainManager.shared.writeXboxBroadcastAltPassword(oldPassword, forServerId: serverId)`). There is no separate "guest token" in MSC 1's config model to migrate — `remote_api_token` is the one credential MSC 1's Remote API authenticates with, not a two-tier owner/guest scheme. P5.1's earlier drafts must not invent a guest-token migration path that doesn't exist in the source.
- Both migrations are conditional on the key being present and, for the token, non-blank after trimming; a missing or blank value is silently skipped, not treated as an error.
- Migration runs once at config load, immediately followed by `save()`, which is what actually strips the plaintext keys from the JSON on disk — the migration itself only writes forward into Keychain. `populateSecretsFromKeychain()` (called both after migration and on every reload) is the authoritative source for these fields in memory afterward; the JSON file never carries them again once migrated.
- P5.9 ("make the migrated owner credential durable and authenticating") is where this connects to Phase 4's `SecretStore` credential work (P4.2/P4.5) — the migration target is MSC 2's `SecretStore`, not literally macOS Keychain, and the migrated token must actually authenticate subsequent requests, not just be stored inertly. That connection is a Phase 4→5 credential-transition contract this phase must document explicitly (P5.9's own job), not something this scoping note resolves.

## Deferred and homeless

Carried forward from `rolling-plan.md`'s Phase 5 "Not in this phase" list, restated here so this scoping note is self-contained:

- **Per-flavor provisioning and installers** (Vanilla/Fabric/Forge/NeoForge/Purpur download-and-install, args-file launch construction) — stay **Phase 7**. Raw-directory import in this phase only detects, infers, and copies what already exists on disk.
- **The formal world-slot model** — stays **Phase 6**. Raw import copies and labels world data but does not create a slot; transfer import copies MSC 1's `world_slots` data verbatim and may use a narrow migration-only reader for a package's active-slot marker/archive when an older package lacks live worlds. MSC 1's raw importer calls `createInitialWorldSlotIfNeeded`; Phase 6 owns the formal replacement. This is a real sequencing tension between phases, not an oversight.
- **Bedrock settings** (`applyBedrock`) and any Bedrock-specific config schema — stay **Phase 10** (D-022's separate Bedrock matrix). This phase's `/v1/settings` route stays Java-only, matching `settings_schema.rs`'s existing Java-only port and Phase 4's Java-only lifecycle scope. Raw-directory import in this phase **does** still detect a Bedrock server directory (MSC 1 does; excluding it would be a real capability regression, not a scope simplification) even though the settings route can't yet expose Bedrock settings for it.
- **Named-token CRUD HTTP routes** (`POST /users`, `/users/update`, `/users/revoke`, `GET /users`) — **not built in this phase, and currently homeless.** `RemoteAPISharedAccessEntry`'s schema is ported as part of `AppConfig`'s own shape (config round-trip parity needs it), but the routes themselves aren't named in any phase's bullet list in the port plan. Recorded here, the same way P3.3 flagged Phase 3's own gaps, for Cameron to place during the Read move rather than this phase silently building or silently skipping it.
- **`GET /v1/help/{helpId}` content-serving and the handbook/concept-guide/router-guide content itself (D-026)** — likewise **not built in this phase, and homeless.** The DTO-level `helpId` pointer field already exists in the frozen contract (P2.2/P2.8) and this phase's settings route carries it on every field per that contract, but resolving the pointer to real content isn't named in any phase's bullet list either.
- **A standalone, publicly routable transfer-package *export* endpoint** — not built. The frozen v1 contract has no export route, and D-009 only requires MSC 2 to read MSC 1's format for migration. `exportServerTransfer` is still built and used internally in this phase, because the HTTP import handler must complete that backup before calling `applyTransferImport` in `replaceAll` mode (see "Transfer behavior" above).
- **D-027** (the CurseForge manual-download workflow) — stays **Open**, revisited at Phase 8.

## MSC 1 symbol inventory

The primary oracle files/symbols this phase's Rust code is ported against, not designed from memory:

| Area | MSC 1 source |
|---|---|
| Typed config schema, decode-time normalization | `AppConfig.swift` (883 lines) — `ConfigServer.init(from:)/encode(to:)`, `AppConfig.init(from:)/encode(to:)`, `ConfigServer.minRamMB/maxRamMB` |
| Config load/save/migrate lifecycle | `ConfigManager.swift` (308 lines) |
| Corrupt-backup discovery/merge, untracked-folder rescan | `AppViewModel+ConfigRecovery.swift` (184 lines) — two separate paths in one file |
| Transfer-package export/inspect/apply | `AppViewModel+ServerTransfer.swift` (603 lines) — no MSC 1 test file exists for any of it |
| Raw import: copy/unzip into owned root, Java+Bedrock detection, EULA, world discovery/ranking, initial world slot | `AppViewModel+ServerImport.swift` — already partially used by P4.8; real scope is larger than that step's slice |
| HTTP wire contract: `action`/`importKind`/`transferMode`/`backupPath` | `AppViewModel+APIWiringServerMgmt.swift` — `serverImportProvider`/`serverImportScanProvider` |
| Settings GET/POST wiring (not just the pure schema) | `AppViewModel+APIWiringSettings.swift` |
| Keychain migration target, secret deletion | `KeychainManager.swift` — `deleteAllMSCSecrets`, and the target of `ConfigManager.init`'s legacy plaintext migration |
| Frozen wire-level DTOs this phase must match byte-for-byte | `RemoteAPIServerDTOs.swift` |
| DTO-building half of the settings route (left unported in P1.6) | `RemoteAPIServer+Settings.swift` |

Phase 5 also absorbs one item deliberately deferred from Phase 4: P4.8's own scope note states plainly, "Transfer-package import and raw ZIP import stay Phase 5" — this phase is where that boundary resolves, broadening P4.8's Paper-only registration into the two D-009 import paths.

## Not resolved by this note

This scoping note pins source behavior and evidence requirements; it does not itself decide open Proposed items (e.g., D-022's Bedrock matrix, D-026's mechanism) or invent product behavior MSC 1 doesn't have. Where later steps hit a genuine judgment call, they raise it as a question in the format `CLAUDE.md` requires rather than deciding it here.
