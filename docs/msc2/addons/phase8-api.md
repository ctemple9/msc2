# Phase 8 API, operation, and capability contract

**Status:** P8.9 contract-freeze note, Proposed (carries forward the Proposed status of everything it builds on: `operation-model.md`, D-023's matrix mechanism, D-027) until Cameron confirms during the Read move.
**Source of truth:** `docs/msc2/api-contract/openapi.json` (the file this note reconciles), `docs/msc2/addons/phase8-scope.md` (P8.1's reconciliation rule, D-027, the pack-managed-guard/create-boundary/rollback findings), `fixtures/addon-providers/`, `fixtures/plugin-source-resolution/`, `fixtures/addon-update-resolution/`, `fixtures/plugin-source-mapping/`, `fixtures/modrinth-dependencies/`, `fixtures/modpack-client-only/`, `fixtures/pack-managed-guard/`, `fixtures/modpack-import/`, `fixtures/modpack-archive-safety/`, `fixtures/client-addon-export/`, `fixtures/curseforge-manual-download/` (P8.4-P8.8's characterization this contract is built to serve), `docs/msc2/api-contract/operation-model.md`, `docs/msc2/api-contract/versioning-and-errors.md`, `docs/msc2/api-contract/permission-vocabulary.csv`, `docs/msc2/msc2-decisions.md` D-006/D-021/D-027, `docs/msc2/families/phase7-api.md` (the async-correction and staged-upload precedents this note reuses).

This note does three things, the same methodology `phase6-api.md`/`phase7-api.md` established: it reconciles the nine routes the frozen `openapi.json` baseline already gives Phase 8 against what P8.4-P8.8's characterization actually found; it adds the staged-upload-backed contract for pack inspection/import and D-027's manual-file completion, since none of MSC 1's capabilities in that area have a baseline route to extend; and it updates `docs/msc2/client-capability-matrix.csv` to each route's real status at this commit.

---

## 1. What already existed

Nine routes named in `rolling-plan.md`'s Phase 8 preamble were already in the baseline (P0.23/P2.8), each with a request/response DTO pair and `x-permission-category` already assigned:

| Route | Maps to (MSC 1) | Shape at baseline |
|---|---|---|
| `GET /v1/addons` | `resolveAddonUpdates` | sync `AddonsResponseDTO` |
| `GET /v1/catalog/search` | `ModrinthAPI.search` | sync `CatalogSearchResponseDTO` |
| `GET /v1/components` | mostly Phase 7's `components-versions` (phase8-scope.md §"GET /v1/components...") | sync `ComponentsStatusDTO` |
| `GET /v1/components/client-export` | `buildClientExportItems` | sync `ClientExportResponseDTO`, corrected §2 |
| `POST /v1/components/install` | `installModrinthAddon` | corrected this step, §2 |
| `POST /v1/components/remove` | `removeMod`/`removePlugin` | sync `AddonRemoveResultDTO`, unchanged |
| `POST /v1/components/update` | `updateAddon`/`updateAddons` + four newly-folded shapes | corrected this step, §2/§4 |
| `POST /v1/components/version` | Phase 7's own, unaffected here | async since P7.9, unchanged |
| `POST /v1/health/repair` | `repairIncompatibleAddon`/`installMissingDependency` for `update`/`install`; Phase 7 owns `disable`/`delete` | corrected this step, §2 |

None of these DTOs' existing fields change meaning; D-006 preserves them. This note adds fields additively (§2/§4) and adds the routes P8.1 found had no baseline home at all (§3).

**Confirmed, not re-decided:** every permission category above already matches `permission-vocabulary.csv` exactly (`none` for the three GETs, `addons` for every POST including `components/version`, `settings` for `health/repair`) — nothing here for this step to decide, except the one correction §5 records for the two staged-upload routes.

---

## 2. Corrections: async installs/updates, and client-export without base64

Three corrections, all D-006 point 3 (a documented behavior change, not a silent one) — the same class phase7-api.md §2 already applied to `servers/create`/`components/version`.

**`POST /v1/components/install` and the two update-triggering shapes of `POST /v1/components/update`.** `installModrinthAddon`/`updateAddon`/`updateAddons` hit Modrinth for real metadata and file bytes; P8.15's dependency-aware batch install can chain several such downloads for one request. The baseline's synchronous `200` blocks the connection for however long that takes, the identical problem phase7-api.md §2 found in `createServerProvider`/`changeVersionProvider`. Both now return `202` once request shape, staged-upload redemption (for a local-file install), and the pack-managed guard (§4) all pass; `CatalogInstallResultDTO`/`AddonUpdateResultDTO` each gain an additive, optional `operationId`. Operation types `"addon-install"`/`"addon-update"`. `POST /v1/components/remove` and the four toggle/link/source-management shapes §4 adds to `/v1/components/update` are **not** corrected this way — none of them touch the network (a filesystem rename/removal or a config write), the same "stays synchronous" bar phase7-api.md §2 applied to `POST /v1/templates`' create-from-template action.

**`POST /v1/health/repair`, `action=update`/`install`.** P7.9 already scoped these two actions out as `action_unavailable` placeholders (phase7-api.md §6: "Phase 7 has no add-on repair machinery at all yet"). P8.23 makes them real, and per `rolling-plan.md`'s own text they "route through the same verified add-on mutation paths" §2's `components/install`/`update` correction just designed — so they inherit the identical timing: `200` stays for `action=disable`/`delete` (Phase 7's existing synchronous rename/removal), `202` is added for `action=update`/`install`, and `HealthRepairResultDTO` gains an additive, optional `operationId`. No schema change to `HealthRepairRequestDTO` — `action` was already an open `string`, not a closed enum (phase7-api.md §6 already noted this).

**`GET /v1/components/client-export`, `exportKind=zip`.** The frozen baseline's `ClientExportResponseDTO` carries an inline `zipBase64` field — but this route has **never been implemented in Rust** (still `Planned` as of this commit, confirmed against §7's own status assessment), so there is no shipped client depending on that shape to preserve compatibility for. P8.22's own working exit criteria explicitly rule out base64 for a new large-payload route: a modded server's client export can be many megabytes, and base64 sends roughly a third more bytes than the ZIP itself. `zipBase64` is removed; `ClientExportResponseDTO` gains `stagedDownloadId`, redeemed via `GET /v1/staged-downloads/{id}` — the exact mechanism `POST /v1/worlds/export` already uses (phase6-api.md §3/§4), reused rather than inventing a second large-payload delivery pattern. `exportKind=links` (Paper-like servers, `shareText`) is unaffected — it was always plain text, never base64, and needs no staged download.

---

## 3. New routes: the staged-upload-backed pack workflow and D-027

`rolling-plan.md`'s own line for this step is deliberately narrow: *"P8.9 may add only the staged-upload-backed inspection/import and manual-file endpoints needed to expose MSC 1 capabilities that have no baseline route."* Three routes meet that bar — pack inspection, pack import (including the pack-managed explicit-replace escape hatch), and D-027's manual-file completion. Everything else Phase 8 needs (toggle, manual Modrinth link, plugin-source management, local-JAR install, pack-driven server creation) rides an **existing** route as an additive field or shape, per §2/§4 — not a tenth new route.

| Route | `operationId` | Maps to | Shape |
|---|---|---|---|
| `POST /v1/modpacks/inspect` | `inspectModpack` | `readMrpackMetadata`/`readCurseForgeMetadata` (read-only sniff, no MSC 1 route — a local wizard step) | sync `ModpackInspectionResultDTO` |
| `POST /v1/modpacks/import` | `importModpack` | `importModpack`/`importExtractedCurseForgeModpack`, plus new pack-managed-replace policy (phase8-scope.md, "Pack-managed guard") | async `ModpackImportResultDTO`, `operationId` always populated |
| `POST /v1/modpacks/{operationId}/manual-file` | `completeModpackManualFile` | D-027's replacement for `CurseForgeManualDownloadSheet`'s folder watch | sync `ModpackManualFileResultDTO` |

**Why inspection is the one deliberate exception to "a staging slot is redeemed once."** `worlds.rs`'s own doc comment establishes the rule every other staged-upload consumer follows: "a staging slot can only be redeemed by the route it was created for" — and, implicitly, only once (phase6-api.md §4: an already-redeemed id is a plain `404` on a second use). A pack workflow needs to look at the same upload twice — once to show the client what it's about to commit to (pinned version, file count, whether any file needs manual completion), and again to actually commit — so `POST /v1/modpacks/inspect` **peeks**: it validates the `stagedUploadId`'s purpose is `modpack-archive` and reads it, but does not mark it consumed. `POST /v1/modpacks/import` (or `ServerCreateRequestDTO.stagedModpackUploadId`, §4) performs the real, one-time redemption. The upload's ordinary `expiresAt` is unextended by inspecting it — a client that inspects and then waits too long to import still gets an honest `404`, not a silently-refreshed token.

**Why the pack-managed decision is a request field, never inferred.** phase8-scope.md's "Pack-managed guard" finding is this phase's most consequential: MSC 1 has no real enforcement to port, so what "refuse individual mutation, allow explicit whole-pack replacement" means is being decided here, not extracted. `ModpackImportRequestDTO.action` (`"import"` | `"replace"`) is **required**, not inferred from the active server's current `packManaged` state, because inferring it would let a client accidentally overwrite a pack-managed server's mods by calling the same route it'd use for a first-time import. `action=import` against an already pack-managed server, or `action=replace` against one that isn't, is refused — `409 conflict`, `ErrorDTO.details` carrying `packName`/`packVersion` so a client can render "this server was installed from *Fabulously Optimized 13.3.0* — replace it, don't try to import a second pack" without guessing.

**Why blocked-file completion pauses the operation instead of a new state.** `operation-model.md` §3's state machine is closed: `queued|running|succeeded|failed|cancelled`, no `paused`. A pack with an author-blocked CurseForge file (D-027) reaches a checkpoint mid-import and needs the client to act before it can finish — represented as `running`, with `ModpackImportResultDTO.pendingManualFiles` (also echoed in the operation's own `statusLine`) naming what's still needed. This is the same "not a new state, a checkpoint within `running`" treatment `operation-model.md` §2 already gives `statusLine` generally ("Waiting for a free download slot"). Each pending file is resolved with its own `POST /v1/modpacks/{operationId}/manual-file` call — chosen as an operation-scoped sub-route (parallel to the existing `POST /v1/operations/{id}/cancel`) rather than a bare `/v1/modpacks/manual-file`, since a manual-file completion is meaningless without naming which paused operation it resumes.

**Sizing the manual-file upload to the actual file, not a flat ceiling.** D-027's own decision text (msc2-decisions.md) requires "expected file identity/name, size ceiling." A flat constant (`worlds.rs`'s `MAX_STAGED_UPLOAD_BYTES`, 10 GiB, sized for a whole world archive) would be far too permissive for one blocked mod jar. `StagedUploadBeginRequestDTO` gains `operationId`/`fileId` — required together, only for `purpose: "curseforge-manual-file"` — so the agent looks up that file's own CurseForge-reported byte size (already captured in `corpus/addons/mods-files-blocked-entityculling.json`'s real evidence: `fileLength` is present on a blocked file's metadata even though `downloadUrl` is null) and bounds the upload to exactly that file, not a generic number. This is a wiring detail (P8.14/P8.20 enforce it); this note only fixes that the wire shape carries enough information to do so precisely.

---

## 4. Additive changes to existing routes

**Toggle, manual link, and plugin-source management fold into `POST /v1/components/update` rather than becoming new routes**, per §3's own scoping rule. `ComponentUpdateRequestDTO` gains four optional fields, each naming a distinct shape when combined with `jarStem`: `enabled` (togglePlugin/toggleMod, sync), `linkProjectId` (manuallyLinkAddon, sync), `sourceUrl` (setPluginSource, sync — parsed the same way P8.10's ported `PluginSourceDetector.detect` classifies any URL), `removeSource` (removePluginSource, sync). Seven request shapes now share one endpoint total (three pre-existing — `updateAll`, `jarStem` alone, the legacy `component=` path — plus these four), extending the exact "N request shapes share one endpoint" convention the baseline's own `x-notes` already used for three. All six add-on-touching shapes are refused against a pack-managed server (`409 conflict`, `code: "conflict"`, `details.packName`/`packVersion` — reusing the code, not minting a new one, per §6) — the escape hatch is `POST /v1/modpacks/import {action: "replace"}`, never any shape of this route. `AddonUpdateResultDTO` gains the same optional `operationId` §2 gives its two async shapes; the other five reuse the same DTO with `operationId` simply absent, rather than a second result type per shape.

**Local-JAR install folds into `POST /v1/components/install`.** `CatalogInstallRequestDTO` gains an optional `stagedUploadId` (redeems a `purpose: "addon-local-file"` upload — `addPluginFromFilePicker`/`addModFromFilePicker`'s port); its three catalog fields (`projectId`/`slug`/`title`) move from all-required to none-required, since exactly one of {catalog fields} or `stagedUploadId` must be present, not both, not neither — the same "no fixed required set, request shape decides which fields matter" pattern `ComponentUpdateRequestDTO` already established at baseline. `CatalogInstallResultDTO` also gains `installedDependencies` (jarStems of required dependencies P8.15's bounded installer pulled in alongside the requested add-on) — additive, empty when there were none.

**Pack-driven server creation folds into `POST /v1/servers/create`.** phase8-scope.md's "Modpack create/import boundary" finding is explicit: MSC 1 has no dedicated create-from-pack primitive, only `createNewServer` followed by `applyStagedAddOn` calling the same in-place `importModpack` used for importing into an existing server — and P8.21's Files list (`crates/msc-application/src/provisioning.rs`/`modpacks.rs`) has no `openapi.json` entry, so this contract has to be complete now or P8.21 has nothing to build against. `ServerCreateRequestDTO` gains an optional `stagedModpackUploadId` (an already-inspected `modpack-archive` upload); when present, the same durable create operation pins the loader/Minecraft version from the pack and applies its mod list, and does not publish the registry entry until pack application succeeds too — extending create's existing rollback (P7's own directory-rollback-on-failure) to a failure class MSC 1's own creation flow could structurally never roll back from (phase8-scope.md's own finding: `applyStagedAddOn` never `throw`s). `type` stays `"server-create"`; no new operation type for this shape.

**Honest provider/not-yet-implemented notes, additive.** `AddonsResponseDTO` and `ComponentStatusDTO` each gain an optional `note`. `GET /v1/catalog/search`'s existing `note` field already carries the "why is this degraded" explanation for an unsupported add-on kind (`supportsAddons=false`); this note is documented (via the route's own `x-notes`, not a schema change — the field already existed) to also cover a reachable-server-but-unreachable-provider failure, still `200`, `results` empty. `GET /v1/addons`'s new `note` is the same idea for the resolve pass: a Modrinth outage doesn't fabricate a fresh answer, it reports last-known persisted state and says so. `GET /v1/components`'s `ComponentStatusDTO.note` documents the still-open Phase 9 gap honestly (`rolling-plan.md`'s own working exit criterion: "`GET /v1/components` may report their installed state but must label unavailable updates honestly") instead of a silently-wrong `isUpToDate`/`updatable` pair.

---

## 5. Permission-category correction: staged-upload begin/upload routes

`POST /v1/staged-uploads` and `PUT /v1/staged-uploads/{id}` carried `x-permission-category: worlds` at baseline — correct when Phase 6 was the only domain using them, wrong the moment a second domain needs the same primitive. **Corrected here to `none`.** Reasoning: staging bytes commits nothing by itself — every purpose's actual redemption route already enforces its own correct category (`worlds` for `world-import`/`active-world-replace` via `POST /v1/worlds/import`/`replace-active-world`; `addons` for `modpack-archive`/`addon-local-file`/`curseforge-manual-file` via `POST /v1/modpacks/import`/`components/install`/`modpacks/{id}/manual-file`) — so an addons-only token, a real, valid D-019-scoped credential, could not have begun a modpack-archive upload at all under the old fixed category, despite being fully entitled to finish the import it was for. This is flagged prominently, not silently changed: it corrects a route two phases already shipped as `Implemented` on the agent (§7), so the real permission check in `crates/msc-agent/src/routes/worlds.rs` still requires `worlds` until Phase 8's wiring (P8.13/P8.14/P8.20) updates it — a documented, expected gap between this contract and today's shipped behavior, the same kind of gap phase7-api.md §2's `operationId` corrections left for `servers/create`/`components/version` until P7.13+ closed it.

---

## 6. Pack-managed conflicts and provider errors: one code reused, one code added

**Pack-managed refusal reuses `conflict`, not a new code.** `versioning-and-errors.md` §5 keeps its vocabulary intentionally tight ("not one-off per route"). A pack-managed refusal is route-specific enough (only the add-on-mutation family) that it doesn't meet the bar `capability_unavailable` did in Phase 6 (a failure class spanning multiple routes and multiple phases' worth of missing runtime). `message` plus `details.packName`/`packVersion` (§3/§4) carries everything a client needs to render the right explanation without a dedicated code to branch on.

**Provider failure during a mutation adds one new code: `provider_unavailable`.** Unlike a read (§4's `note` treatment, still `200`), a mutation that genuinely cannot proceed because Modrinth/Hangar/GitHub/CurseForge is unreachable is a real failure — but it is a *different kind* of failure than `internal_error` (nothing is broken on this end) or `invalid_body` (the request was fine). This is exactly `capability_unavailable`'s own justification in phase6-api.md §5, reapplied: a real, multi-route failure class (any of `components/install`, the two async `components/update` shapes, `modpacks/import`, and the async `health/repair` actions can hit it) that a client benefits from being able to branch on distinctly ("retry later" vs. "fix your request"). Surfaces as the terminal `ErrorDTO.code` on the operation's `failed` state for every route above except `catalog/search` (which never errors for this reason at all, per §4's `note` treatment).

---

## 7. `helpId` and audit: nothing new to design

Every non-2xx response across all twelve routes this note touches already resolves to `ErrorDTO` (`tools/api-contract-check.py --v1-summary` confirms `non-errordto-responses: 0`, §9's own Verify). None of the fields this note adds are in `helpid-contract.md` §4's table, and none needed to be: no new `SettingFieldDTO`/health-card/startup-problem surface is introduced — Phase 8's health-repair additions (§2) reuse `HealthRepairResultDTO`'s existing `updated: HealthProblemsResponseDTO`, whose `StartupProblemDTO.helpId` P7.8 already populated for all five kinds — so `missing-helpid: 0` holds unchanged. Every mutation route this note designs writes through the existing `audit_log` primitive (`crates/msc-infrastructure`, built before Phase 6) the same way every other phase's mutation routes already do; there is no wire-level audit contract to design, since audit records are never returned to a client on this API.

---

## 8. The client capability matrix

Same methodology phase6-api.md §7/phase7-api.md §8 established, re-applied rather than re-derived.

- **`agent_status`.** Four of the twelve routes this note touches were already `Implemented` before this commit — `POST /v1/components/version` (untouched here), `POST /v1/servers/create`, `POST /v1/health/repair`, and the staged-upload begin/upload pair — and stay `Implemented`: the route exists and answers today, even though (per §2/§5) some of what this note just froze isn't real yet. Every other row this note touches, including all three new `/v1/modpacks/*` routes, is `Planned`: this step freezes the contract, P8.13-P8.26 build the services and route wiring behind it.
- **`desktop_web_status`** stays `Planned` on every row without exception, per `rolling-plan.md`'s own Phase 8 preamble (desktop/web screens stay Phase 11).
- **`ios_status`/`cli_status`** stay `Planned` on every row this note touches — P8.25/P8.26 repoint the CLI and copied iOS client, not this step.
- **No row uses `Intentional exception`.** Every `Planned` row here is a scheduled later step in this same phase, not an approved permanent gap.

Notes are added to the four already-`Implemented` rows this step's corrections affect, naming the specific gap between the now-frozen contract and today's shipped behavior (§2's `operationId`/`202` additions for `servers/create`/`health/repair`; §5's permission-category correction for the staged-upload pair), the same "flag the timing gap in `notes`" convention phase7-api.md §8 used for its own `operationId` corrections. Three new rows are added in full for `/v1/modpacks/inspect`, `/v1/modpacks/import`, and `/v1/modpacks/{operationId}/manual-file`. `python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv` (unchanged tool, generic over any `openapi.json`) confirms shape and coverage: 112 rows now (110 `openapi.json` operations + 2 WebSocket channels) — see §9 for the count's own derivation.

---

## 9. Route/operation count

Three new operations: `POST /v1/modpacks/inspect`, `POST /v1/modpacks/import`, `POST /v1/modpacks/{operationId}/manual-file`. `EXPECTED_TOTAL` in `tools/api-contract-check.py` moves from 107 to **110**. `docs/msc2/client-capability-matrix.csv` moves from 109 rows (107 operations + 2 WebSocket channels) to **112** (110 + 2).

## Verify

```
python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run -p msc-api --test phase8_conformance
```
