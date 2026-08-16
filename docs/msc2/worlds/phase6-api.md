# Phase 6 API and capability contract

**Status:** P6.8 contract-freeze note, Proposed (carries forward the Proposed status of everything it builds on: operation-model.md, D-023's matrix mechanism) until Cameron confirms during the Read move.
**Source of truth:** `docs/msc2/api-contract/openapi.json` (the file this note reconciles), `docs/msc2/worlds/phase6-scope.md` (P6.1's reconciliation rule), `fixtures/world-mutations/`, `fixtures/world-archive-safety/`, `fixtures/backups/`, `fixtures/backup-restore/`, `fixtures/world-conversion/` (P6.4–P6.7's characterization this contract is built to serve), `docs/msc2/api-contract/operation-model.md`, `docs/msc2/api-contract/versioning-and-errors.md`, `docs/msc2/msc2-decisions.md` D-023.

This note does two things: it reconciles the frozen `openapi.json` baseline with the full set of world/backup operations Phase 6 needs so no client is architecturally blocked, and it explains the methodology behind `docs/msc2/client-capability-matrix.csv`, the D-023 matrix this step finally builds (D-023 called it "Proposed... mechanism" and `rolling-plan.md` calls it "overdue").

---

## 1. What already existed

Six world/backup routes were already in the P2.8 baseline, ported forward unchanged from MSC 1's own surface (`docs/msc2/audit/msc2-symbol-ledger.csv`, `permission-vocabulary.csv`):

| Route | Maps to | Shape |
|---|---|---|
| `GET /v1/worlds` | `WorldSlotManager.loadSlots` + active resolution | `WorldSlotsResponseDTO` |
| `POST /v1/worlds/create` | `WorldSlotManager.createSlot`/`createFreshWorldSlot` | sync `WorldMutationResultDTO` |
| `POST /v1/worlds/rename` | `WorldSlotManager`'s slot rename (metadata only, no file I/O) | sync `WorldMutationResultDTO` |
| `POST /v1/worlds/replace` | `WorldSlotManager.copySlotIntoExisting` (a saved-slot-to-saved-slot copy — **corrected post-P6.21-review**, see §9) | sync `WorldMutationResultDTO` |
| `POST /v1/worlds/repair` | `AppViewModel+WorldRepair.swift` (Bedrock `level.dat` repair) | sync `WorldMutationResultDTO`, Phase 10-gated |
| `POST /v1/worlds/activate` | `WorldSlotManager.activateSlot` | **async**, `{result: "activation_started"}` |
| `GET /v1/backups` | backup listing | `BackupsResponseDTO` |
| `GET`/`POST /v1/backups/config` | auto-backup config | `BackupConfigResponseDTO`/`...UpdateResultDTO` |
| `POST /v1/backups/now` | `AppViewModel+Backups.swift::createBackup` | **async**, `{result: "..."}` |
| `POST /v1/backups/restore` | `AppViewModel+Backups.swift::restoreBackup` | **async**, `{result: "..."}` |

None of these request/response shapes changed. D-006 preserves them; this note only adds fields additively (SS2) and adds the operations this baseline never had (SS3–4).

**A naming trap worth stating explicitly, because it cost real time to untangle:** `POST /v1/worlds/rename` and the new `POST /v1/worlds/rename-active-world` (SS3) are **not the same operation** despite the shared verb. The existing route is `WorldSlotManager`'s own rename — it edits `slot.json`'s `name` field and touches no files (`fixtures/world-mutations/rename-slot-metadata-only-leaves-archive-untouched.json`). The new route is `AppViewModel+WorldManagement.swift::renameWorld` — it moves the *live, currently-active* world's on-disk folders (main/nether/end) to new names, with an all-or-nothing pre-check across all three target names and a `rollbackMovedFolders()` recovery path (`fixtures/world-mutations/rename-world-*.json`).

**A second naming trap, caught only after P6.21 tried to wire `/v1/worlds/replace` to a real service and had to guess (§9 records the correction):** this note originally claimed `POST /v1/worlds/replace` "already correctly mapped to `replaceWorld`'s direct-world semantics (its request shape, `{slotId, sourceSlotId}`, only makes sense as 'replace the active world's content with slot X's')." That claim was wrong. `{slotId, sourceSlotId}` is `WorldSlotManager.copySlotIntoExisting`'s own shape, not `replaceWorld`'s — `slotId` is the existing *destination* slot being overwritten, `sourceSlotId` is the slot supplying replacement content, and neither the live world nor a new level name is involved at all. `rename` still had no direct-live-world counterpart before SS3 added one; `replace` was never that counterpart to begin with.

---

## 2. Additive changes to existing routes

Per the "add operation IDs additively for activation/backup/restore/conversion" requirement:

- `POST /v1/worlds/activate`, `POST /v1/backups/now`, and `POST /v1/backups/restore` each gain an OpenAPI `operationId` (`activateWorldSlot`, `createBackupNow`, `restoreBackup`).
- `WorldActivateResultDTO`, `BackupNowResultDTO`, and `BackupRestoreResultDTO` each gain an optional `operationId: string` field, reusing the exact pattern `SimpleResult` already established in Phase 4 (`docs/msc2/api-contract/openapi.json`, `POST /v1/active-server`'s response schema) rather than inventing a second convention: *"Operation id for progress polling (`GET /v1/operations/{id}`) or `/v1/operations/{id}/stream` and cancellation; optional so older clients can ignore it."* A client that only reads `result` keeps working unmodified; a client that wants a progress bar or a cancel button now can, for all three of Phase 6's slow world/backup mutations, through the one operation mechanism `operation-model.md` already designed in Phase 2 — nothing new to design here, only to wire the reference through. This is also how "restart behavior" is answered for these three routes without new design: `operation-model.md` SS6 already routes operation survival through Phase 3's durable journal; Phase 6 adds no second restart story.
- `WorldSlotDTO` gains `hasThumbnail: boolean`. `WorldSlotManager` already generates a deterministic thumbnail on create/update (P6.12); this exposes whether one exists so a client knows whether `GET /v1/worlds/{slotId}/thumbnail` (SS3) is worth calling. There is no separate thumbnail-write route — generation is an automatic side effect of the mutation routes that already exist, not a capability of its own (decided for you: mechanical, nothing for Cameron to weigh in on).
- `POST /v1/backups/restore` gains an `x-notes` clarifying that its Bedrock-unsupported refusal (first of `fixtures/backup-restore`'s four source-ordered guards) carries `ErrorDTO.code: "capability_unavailable"`, distinct from the plain `"conflict"` the other three guards (running-server, cross-slot, missing-source-file) carry. See SS5.

---

## 3. New routes

Eleven new operations close the gaps `fixtures/world-mutations`, `fixtures/world-archive-safety`, `fixtures/backups`, and `fixtures/world-conversion` characterized but the baseline never exposed. (A twelfth, `POST /v1/worlds/copy`, was proposed here originally but removed post-review — see §9: it duplicated `POST /v1/worlds/replace`'s corrected, real semantics exactly.)

| Route | `operationId` | Maps to | Shape |
|---|---|---|---|
| `POST /v1/worlds/update` | `updateActiveWorldSlot` | `WorldSlotManager.updateSlotFromCurrentWorld` — save the live world into the active slot | sync, no request body |
| `POST /v1/worlds/delete` | `deleteWorldSlot` | slot delete, refused on the active slot | sync `WorldMutationResultDTO` |
| `POST /v1/worlds/duplicate` | `duplicateWorldSlot` | fresh-UUID slot duplicate | sync `WorldMutationResultDTO` |
| `POST /v1/worlds/import` | `importWorldSlot` | import a staged ZIP as a new slot | sync `WorldMutationResultDTO` |
| `POST /v1/worlds/export` | `exportWorldSlot` | stage a slot's archive for download | sync `WorldExportResultDTO` |
| `POST /v1/worlds/rename-active-world` | `renameActiveWorld` | direct live-world folder rename (SS1) | sync `WorldMutationResultDTO` |
| `POST /v1/worlds/convert` | `convertWorld` | start a Chunker conversion, between a `sourceSlotId` on the active server and a separate, required `targetServerId` (§9) | **async only** — `WorldConvertResultDTO`, `operationId` required, not optional |
| `GET /v1/worlds/{slotId}/thumbnail` | `getWorldSlotThumbnail` | fetch a slot's thumbnail bytes | `image/png`, 404 if none |
| `POST /v1/backups/delete` | `deleteBackup` | delete a backup, refusing to drop the sole remaining verified backup | sync `SimpleResult` |
| `POST /v1/staged-uploads` | `beginStagedUpload` | begin a bounded staged upload (SS4) | `StagedUploadBeginResultDTO` |
| `PUT /v1/staged-uploads/{id}` | `uploadStagedBytes` | send bytes into a staging slot | `StagedUploadCompleteResultDTO` |
| `GET /v1/staged-downloads/{id}` | `downloadStagedBytes` | fetch bytes from a prepared export | binary |

Eleven routes, twelve operations (`staged-uploads/{id}` and the others are one operation each — `tools/api-contract-check.py`'s `EXPECTED_TOTAL` counts the operation, not the route: 88 baseline + 5 P2.8 + 12 P6.8 = 105).

**Why `convert` has no synchronous variant, unlike every other new route.** Every other new mutation costs roughly what `create`/`rename`/`replace` already cost (a folder move or a bounded zip operation) and stays in the baseline's existing synchronous `WorldMutationResultDTO` pattern. Chunker conversion shells out to an external process over a real modpack-sized world and can run for minutes (`fixtures/world-conversion`'s characterization of the five-flag CLI invocation with streamed output) — the same category of work `activate`/`backups/now`/`backups/restore` were already async for. `operation-model.md` SS2 already names `world-conversion` as an anticipated future `type` value, so this route creates its operation with `type: "world-conversion"` through the existing mechanism rather than inventing a fourth async convention.

**Why `update` takes no request body.** `updateSlotFromCurrentWorld` always targets the active slot — there is nothing for a caller to name. This mirrors `POST /v1/backups/now`, which likewise has no request body in the existing baseline.

---

## 4. Staged upload/download: bounded, not arbitrary paths

The "What" for this step requires "bounded staged upload/download instead of arbitrary remote paths" for import/export. The shape:

1. `POST /v1/staged-uploads {purpose: "world-import"}` → `{stagedUploadId, uploadPath, expiresAt, maxBytes}`. `purpose` is a closed enum (one value today) so a staging slot can only be redeemed by the route it was created for — a staged upload created for world import cannot later be handed to some other route as bytes for something else.
2. `PUT /v1/staged-uploads/{id}` with the raw bytes → `{stagedUploadId, receivedBytes, sha256}`. Rejects once `maxBytes` is exceeded or `expiresAt` has passed (409, `conflict`).
3. `POST /v1/worlds/import {name, stagedUploadId}` redeems the staging slot. A `stagedUploadId` that was never uploaded to, or was already redeemed, is a plain `404`.

Export mirrors this in reverse: `POST /v1/worlds/export {slotId}` prepares a `stagedDownloadId` (short-lived, per-slot, not a filesystem path) that `GET /v1/staged-downloads/{id}` redeems once. An expired id is `404`, not `409` — SS3's `x-notes` on that route explains why: once expired, the id is indistinguishable from one that never existed.

**What this note does not decide:** the concrete expiry window, `maxBytes` ceiling, and where staged bytes live on disk (a temp dir under the approved server root, presumably) are P6.21 wiring decisions — "Enforce... approved-root staging" is that step's own language, not repeated here. This note only freezes the wire shape so P6.20's DTOs and P6.21's real enforcement have a fixed contract to build against.

---

## 5. `capability_unavailable`: one new error code

`versioning-and-errors.md` SS5 keeps its `code` vocabulary intentionally open ("Small closed-ish vocabulary per failure kind... not exhaustively enumerated... that enumeration happens in P2.8" and later phases). Phase 6 adds one value: **`capability_unavailable`** — the route exists and the request is otherwise well-formed, but this specific operation requires a live runtime Phase 6 doesn't have yet (a running Bedrock server process — repair and Bedrock-side online-consistency both need one, and neither exists before Phase 10, per this phase's own "Not in this phase" note in `rolling-plan.md`). This is deliberately **not** the same concept as D-023's "Intentional exception" (SS6) — that marks a *client screen* nobody will build; `capability_unavailable` marks a *server-side runtime dependency* that doesn't exist yet on any client. Distinguishing the two matters because their remedies differ: an Intentional exception needs owner approval and a decision entry; `capability_unavailable` needs Phase 10, and is expected to disappear on its own once that phase lands, not to be re-approved.

`versioning-and-errors.md` itself is left unedited (it's Confirmed) — this is recorded here, additively, rather than reopening that file.

Used today by exactly one route: `POST /v1/backups/restore`'s Bedrock-unsupported guard (SS2). `POST /v1/worlds/repair` keeps its existing baseline `code: "conflict"` for `bedrock_only` unchanged (D-006: preserve an existing baseline route's behavior) — a future phase may want to reclassify it once repair actually implements the distinction, but that's not this step's call to make.

---

## 6. Permission categories

Every route above carries `x-permission-category: worlds`, matching every existing world/backup mutation route in the baseline, except `GET /v1/worlds/{slotId}/thumbnail`, which is `none` (read-only, same treatment as `GET /v1/worlds` and `GET /v1/backups`). No new permission category was needed — D-019's nine-bucket vocabulary already covers this domain.

---

## 7. The client capability matrix

`docs/msc2/client-capability-matrix.csv` is the D-023 matrix — one row per `openapi.json` operation (105 rows) plus the two WebSocket channels `websocket-v1.json` documents (`console`, `operation-progress`; 107 rows total). Columns: `method, path, operation_id, msc1_capability, permission_category, agent_status, desktop_web_status, ios_status, cli_status, notes`.

**Status values are `Implemented`, `Planned`, or `Intentional exception`** (D-023's own three values), assessed as of this commit — not aspirational, not "eventually":

- **`agent_status: Implemented`** only for the routes `crates/msc-agent/src/main.rs::build_app()` actually mounts against real service logic today: `health`, `operations` (create/get/cancel/stream), `servers` (list/import), `active-server`, `start`, `stop`, `command`, `status`, `performance`, `settings` (get/post), `capabilities`, `console` (stream/tail). Every other row — including every world/backup row this note just designed — is `Planned`: P6.8 freezes the contract; P6.9–P6.19 build the services behind it.
- **`desktop_web_status`** is `Planned` for every single row without exception, per `rolling-plan.md`'s own Phase 6 preamble: "Desktop/web screens stay Phase 11. Their cells are `Planned` in the capability matrix; that is not an exception." Encoding this as an exception would misrepresent a scheduling fact as an approved permanent gap.
- **`ios_status: Implemented`** only for the routes the copied iOS client was actually repointed at the real `msc2` agent for — `status` (P2.19), and `servers`/`active-server`/`start`/`stop`/`command`/`console tail`+`stream`/`performance` (P4.19). Every other row, including `settings` (CLI-only per P5.11, never repointed on iOS) and everything Phase 6 adds, is `Planned`.
- **`cli_status: Implemented`** only for the commands `crates/msc-agent/src/cli/mod.rs` actually sends: `status`, `servers import` (all three import shapes), `start`, `stop`/`restart` (client-side stop-then-start, no dedicated route), `command`, `console tail`, `settings` (get/update), `active-server`. Everything else, `operations` included (no CLI command polls or cancels an operation today), is `Planned`.
- **No row uses `Intentional exception` yet.** Nothing in the current surface is a capability MSC 1 has that some MSC 2 client is *permanently* not getting — every `Planned` row is a scheduling fact (a later phase's job), not an approved permanent gap. The first real exception, if one is ever needed, becomes its own `msc2-decisions.md` entry per D-023's own rule, not a CSV row with no paper trail behind it.

This matrix is a snapshot, not a one-time artifact — `tools/phase6/capability-matrix-check.py` (SS8) keeps it honest against `openapi.json`'s actual operation set on every run, and later phases that wire real service logic, CLI commands, or iOS screens are expected to flip the relevant cells from `Planned` to `Implemented` as part of *that* phase's own commit, the same way P4.18/P4.19 and P5.11 already did for the rows they touched.

---

## 8. `tools/phase6/capability-matrix-check.py`

New checker, dependency-free (stdlib only), mirroring `tools/api-contract-check.py`'s shape. Given the CSV path as its one positional argument, it:

1. Confirms the header matches exactly: `method,path,operation_id,msc1_capability,permission_category,agent_status,desktop_web_status,ios_status,cli_status,notes`.
2. Confirms every `(method, path)` pair in `openapi.json`'s `paths`, plus the two `websocket-v1.json` channels, has exactly one row — no missing operation, no orphan row naming a route that doesn't exist, no duplicate.
3. Confirms every status cell (`agent_status`, `desktop_web_status`, `ios_status`, `cli_status`) is one of the three D-023 values, and non-blank.
4. Confirms every `desktop_web_status` cell reads `Planned` (SS7's blanket rule — a `Implemented` or exception cell there this phase would be a real bug, not a style nit).
5. Confirms any `Intentional exception` cell has a non-empty `notes` value naming a `D-0\d\d` decision — D-023's "becomes its own decision entry" requirement, checked mechanically since there are no exceptions to check against yet, but the rule needs to hold the day there is one.

`--selftest` runs two bundled fixtures (one clean, one violating rules 1–5) the same way `api-contract-check.py --selftest` and `corpus-check.py --selftest` already do.

---

## 9. Post-P6.21-review correction (2026-08-15)

P6.21 (real route wiring) surfaced two shapes this note had gotten wrong, both flagged as open questions to Cameron rather than guessed silently, and corrected here — before either had shipped to a client — per his review:

- **`POST /v1/worlds/replace` is `WorldSlotManager.copySlotIntoExisting`, not `AppViewModel+WorldManagement.swift::replaceWorld`.** §1's original claim that `{slotId, sourceSlotId}` "only makes sense as 'replace the active world's content with slot X's'" was wrong — that shape is `copySlotIntoExisting`'s own (`slotId` = destination slot being overwritten, `sourceSlotId` = source slot), and the operation never touches the live world or needs a new level name. `POST /v1/worlds/copy` — proposed in the original P6.8 pass, with no MSC 1 counterpart — turned out to duplicate this exact behavior once the correction was made, so it has been removed from the contract rather than kept as a redundant second route to the same operation (§3's route/operation counts are updated accordingly: eleven new routes/twelve new operations, 105 total).
- **World conversion needs a separate `targetServerId`, and `targetFormat` is client-chosen, not hardcoded.** `AppViewModel+WorldConversion.swift::performWorldConversion` always takes a `sourceServer`/`targetServer` pair (MSC 1's own wizard restricts `targetServer` to a different, opposite-edition configured server) and a caller-supplied `targetFormat` loaded from `ChunkerManager.supportedFormats(javaPath:)` — the P6.21 implementation had wrongly passed the same active server as both source and target, with a hardcoded placeholder format. `WorldConvertRequestDTO` now carries `sourceSlotId` (active server), `targetServerId` (required, separate), `targetFormat` (required, validated server-side against the installed Chunker jar's real supported-format list), and exactly one of `targetName`/`targetSlotId` (the latter by id, not display name, per Cameron's explicit correction).

Both corrections are implemented in the same commit that updates this note; see that commit and `rolling-plan.md`'s P6.20/P6.21 entries for the full account.
