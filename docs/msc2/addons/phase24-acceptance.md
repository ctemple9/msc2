# Phase 24 acceptance record

Implementation authorization: Cameron, 2026-11-05. No feature acceptance yet.
Contract: [integration specification](phase24-contract.md).

| Scenario | Owner evidence | Result |
|---|---|---|
| Modrinth/CurseForge search, details, pinned files, install and updates | pending | unverified |
| Both creation mismatch choices; staged-content conflicts | pending | unverified |
| Automatic full-pack/server-only selection with additions, removals and disabled mods | pending | unverified |
| Individual mod player dependencies, unknown/conflicting classification | pending | unverified |
| Existing server recovery without live changes | pending | unverified |
| Update preserves edits/removals; restore upstream per conflict | pending | unverified |
| Base replacement preserves additions and worlds | pending | unverified |
| Immutable dated publication and historical re-export | pending | unverified |
| ZIP/.mrpack equivalent content and launcher installation | pending | unverified |
| Restricted/missing/unidentified and incomplete exports | pending | unverified |
| Local save from remote host and selected-host save | pending | unverified |
| Host switching, permissions/revocation, cancellation/restart/disk full | pending | unverified |
| World-slot datapacks and private-file exclusions | pending | unverified |
| CLI/TUI capabilities and old-agent behavior | pending | unverified |
| Storage references, cleanup safety and large-pack resource behavior | pending | unverified |
| Five launcher guides with supplied screenshots | pending screenshots | unverified |
| macOS/Windows/Linux physical evidence | pending | unverified |

P24.1: production source inventory and compatibility/review/export contracts
recorded. No new endpoint or capability advertised. No tests or live provider
requests run. Later slices add precise artifact/version/platform evidence here.

P24.2: versioned collection persistence and portable metadata validation added.
Infrastructure build and focused Clippy passed; config round-trip and physical
Storage acceptance remain owner-unverified. No retained files are created yet.

P24.3: agent catalog/acquisition endpoints implemented; build and Clippy passed.
API generation and Svelte check passed (10 existing unused-selector warnings).
Live credentialed catalog/download, restricted file, cancellation/revocation,
owner-only redemption, restart and physical platform acceptance remain pending.
No test or release commands run.

P24.4: reviewed individual CurseForge installation, dependency resolution, manual
uploads and durable activation recovery implemented. Build and focused Clippy
passed; generated API/frontend check passed with the same 10 CSS warnings.
Three essential tests in `content_install_safety.rs` protect wrong-byte uploads,
failed metadata-save rollback and owner-edited recovery files; controlled local
inputs, no network/timing assumptions, expected <100 ms each. They were not run.
Live provider installs, manual file admission, credential isolation/revocation,
concurrent actions, restart/recovery and cross-platform behavior remain owner
acceptance items. Player collection consumers are implemented in P24.7.

P24.5: exact-byte CurseForge identification, manual source hints/links, compatible
update discovery, reviewed replacements, preserved dependencies/disabled state
and journaled original rollback are implemented through additive API endpoints.
Diagnostic client integration consumes `content_review_required` in P24.11;
legacy diagnostics will not mutate a known CurseForge file through Modrinth.
Agent Clippy/build, API generation and frontend check passed (10 existing CSS
warnings). Two additional essential tests cover original restoration for
same-name/renamed updates; deterministic local fixtures, expected <100 ms each,
not run. Live fingerprint matches, update/manual flows, permissions, recovery,
physical platforms and source-aware diagnostic UI remain owner-unverified.
Fingerprint API reference: https://docs.curseforge.com/rest-api/#get-fingerprints-matches-by-game-id
Murmur mixing reference: https://github.com/aappleby/smhasher/blob/master/src/MurmurHash2.cpp
Candidate fingerprints alone never establish file identity.

P24.6: original archives, shared/client/server override layers and complete
manifest retention are implemented before destructive server filtering. Player
and server baselines retain distinct bytes at the same logical path. Missing
content stays explicit; retained status/retry/manual endpoints and legacy manual
completion share exact-size/hash checks. Provider-acquired pack identities and
CurseForge source-server-pack flags persist; local archive source type remains
unknown and server-only archives require player recovery. Focused Clippy and
frontend type-check passed (10 existing CSS warnings); generated schema references
resolve. Two essential tests cover rejected retained asset bytes and side-specific
override separation, controlled local files, expected <100 ms each, not run.
Live provider resolution, restricted/manual downloads, cancellation/restart,
disk exhaustion, permissions and all physical platforms remain owner-unverified.
Unsupported mirrors, missing exact hashes and uncertain environment evidence
remain unresolved rather than being silently omitted or called export-ready.

P24.7: independent player blobs, upload admission, stale/owner-bound content
reviews and atomic collection application are wired through the shared API.
Player options/removals and explicit per-item upstream restoration preserve
server bytes. Classification is separate from active server installation;
existing stopped-server mutations remain in use and new reads recompute content
requirements after changes. Known provider dependency/conflict evidence is
retained; ambiguous local requirements remain owner review items. Optional
shader/resource content starts inactive. Focused Clippy, API generation and
frontend check passed (10 existing CSS warnings). Two essential local tests
protect stale reviews, credential ownership and independent player copies,
expected <100 ms each, not run. Live behavior, complete provider/classification
acceptance, cancellations, filesystem errors and physical platforms are pending.

P24.8: existing-server inventory/original-pack recovery review, review retrieval
and application through independent player metadata are wired. Uploaded and
checksum-verified retained originals are supported; active server/world files
are never imported or overwritten. Compatible setup, known base identity,
current bytes, permissions and owner review identity are checked. Conflicts,
removal tombstones, missing client content and unverified local source remain
explicit. Focused Clippy, API generation and frontend check passed (10 existing
CSS warnings). One essential local test protects server config/world bytes and
original/client override retention, expected <100 ms, not run. Live provider,
manual recovery, cancellations, interruptions, credentials and physical platforms
remain pending owner acceptance.

P24.9: three-way pack update review/read/apply and review-bound exact manual
recovery are wired through explicit-server, stopped-server operation boundaries.
Owner edits/removals/options and provider-qualified filename changes are handled
without silently resurrecting or duplicating a mod. Active file plans use private
before/after copies and a durable recovery marker; rollback and startup preserve
subsequent owner edits and block unresolved recovery. Missing bytes/classification
block apply and require a fresh review after manual recovery. Cross-game/loader
migration is explicitly refused by this content update path. Focused Clippy,
generated API/frontend check passed (10 existing CSS warnings); schema references
resolve. Three essential metadata comparison tests (<5 ms each) and two local
generic activation safety tests (<100 ms each) were added, not run. Existing
single-file/JAR tests lack old upstream baselines and generic config activation.
Live providers, manual flows, interrupted large-pack updates, disk exhaustion,
permissions, cancellation, physical platforms and all owner acceptance are pending.

P24.10: replacement review/apply use the shared durable pack transaction with
separate persisted intent. One active base identity/baseline is retained; old
owned files, owner changes/additions and known provider conflicts are reviewed.
World compatibility acknowledgment and exact-hash confirmations for retained
owner mods are required. Worlds remain outside mutation; old retained roots
remain protected. Current legacy pack identity is updated with name/version.
Strict apply bindings remain separate from a prior archive reference used to
prepare a fresh review after edits. Focused Clippy, API generation and frontend
check passed (10 existing CSS warnings); schema references resolve. Existing
P24.9 comparison/activation safety coverage is reused, with no new duplicate
tests and no test commands run. Physical replacement/compatibility, providers,
permissions, cancellations, interruption recovery and platforms are unverified.

P24.15: exact-byte/audience publication review and immutable dated snapshots are
wired with read/history/datapack inventory APIs and capability discovery. Config
review and redistribution evidence are pinned; known restrictions cannot be
attested away, and selected missing content requires incomplete consent. Copies
are independent, manifests are registry-checksummed, sources are revalidated
before registry commit, and current edits can be detected without rewriting old
snapshots. Explicit world-slot datapacks are read without whole-world extraction;
live removals do not resurrect stale archived datapacks. Text-v1 instruction assets
are frozen and explicitly labeled as a development text fallback. Illustrated
launcher guides and owner screenshots remain P24.20. Focused agent Clippy,
generated API and frontend checks passed (10 existing CSS warnings); schema
references resolve. Three essential local safety tests cover source independence,
manifest integrity and restriction/consent boundaries, expected <100 ms each,
not run. Publication/revocation/cancellation flows, large worlds/packs, resource
limits/disk exhaustion, platform behavior and all owner acceptance are pending.

P24.16: streaming audience-specific ZIP generation and permission-bound staged
binary delivery are wired. Output contains frozen bytes, README text/escaped
HTML, manifest and frozen instruction files; optional cosmetics are included
inactive and disabled JARs preserve disabled state. Archive layout zip-v1 is
pinned for historical exports. Space, hashes, paths, collisions, cancellation,
cleanup errors and transfer-admission bindings are checked. Disabled server
publication paths and content-operation cancellation permissions were corrected;
new staged exports require addons permission again at download admission.
Focused agent Clippy/API generation/frontend checks passed (10 existing CSS
warnings); schema references resolve. Two essential local ZIP tests cover
scope/frozen-byte integrity/inactive assets/HTML safety and deterministic cancel
cleanup, expected <150 ms each, not run. Large-file/resource limits, cleanup
failures, revoked credentials, transfer disconnects and physical platforms remain
unverified. Launcher screenshots/illustrated guide acceptance remains P24.20.

## Final implementation candidate — 2026-11-06

This is a handoff for owner verification, **not Phase 24 acceptance or closure**.
P24.1–19 and P24.21–23 have implementation candidates. P24.20 has text/HTML
preparation only and remains incomplete without owner screenshots and exact
launcher-build verification. P24.24 records this evidence, not invented results.
The owner explicitly deferred intermediate UI reviews to this final handoff.

Only these physical results are supported by the conversation: Cameron approved
provider discovery/shared project details (“ui passes”), rejected routine
CurseForge install confirmation, and confirmed the reported 302 download repair
(“nice it worked”). These do not establish all-provider/all-platform acceptance.

### Start the candidate

From `clients/desktop-web`, run `npx tauri dev`. The updated development agent is
built and staged, but the assistant did not install/restart the running service.
If the connected service is older, stop its servers and use **Agent… → Repair
agent**, then reconnect. Update a remote host's agent separately before testing
new capabilities. Never treat a running service's version label alone as proof
that it is this build.

Local candidate: macOS Intel, MSC 0.1.43 development build. SHA-256 of both
`target/debug/msc` and the staged desktop `Resources/agent/msc`:
`08fc7d712e5a4aea19a4fb33356fa7eb52b29bf43828e5134b12c02e7cef38b2`.
No beta/release was published. Native desktop type-check, frontend build and
agent/sidecar development staging succeeded. Existing CSS/chunk and Xcode/Swift
warnings remain warnings; they are not physical behavior evidence.

### Requirement-to-candidate review map

Use disposable servers, packs and destinations. Record exact agent hash, host OS,
launcher build, selected project/release IDs, date and observed outcome for each
scenario. Leave failures and unexercised cases visible.

| Contract item | Implementation evidence / navigation | Owner evidence still required |
|---|---|---|
| Both providers across discovery/detail/releases | Shared ProjectDetailSheet and catalog adapter; Components → Browse mods/plugins; legacy Add-ons Catalog now opens that same browser | Same sheet geometry, icons, paging, stable-only, errors and missing-key-only controls |
| Exact installation/dependencies | content_install, addon_dependencies, ProviderCatalogBrowser; compatible Add runs directly | Required/shared/cyclic/conflicting dependencies, actual bytes, restricted/manual files and mismatch consent |
| Identification/manual hints/updates | content_sources; Content sources and updates, explicit hash/release review | Exact matching vs ambiguous hints, disabled state, update/rename/manual files and owner edits |
| Diagnostics counterpart | health repair returns provider review requirement; Health now opens the shared CurseForge browser or source/update screen on that response | Actual diagnosed CurseForge offender; no Modrinth substitution; original host/server retained |
| Creation/local import | Add Server → Add-ons → Browse modpacks; catalog acquisition and setup review; existing archive import remains | Both mismatch choices, exact loader/version change, staged conflicts kept/removed, retries and one-base-pack limit |
| Complete retention | pack_retention, retained pack status/retry/manual APIs | Client/shared/server overrides with different bytes, missing client content and server-only pack evidence |
| Separate player collection | Player content; content_management, content_recovery | Player-only additions/options/removal/restore; unchanged server bytes; audience and unknown evidence |
| Server mod player checks | Components post-install/update checks; legacy Add-ons and diagnostic repairs now read requirements too | Required dependency missing/different/disabled; optional classification and honest review prompt |
| Existing server recovery | Player content → Recover/update/replace pack | Original client archive and retained recovery; no live mod/config/world mutation; missing/uncertain origin |
| Update preserves customization | PackChangesSheet; pack_review, pack_updates, pack_activation | Edit/removal tombstones, custom config, per-path restore; stale review rejected |
| One-base replacement | Same pack comparison, distinct replacement intent | Old owned removal, retained additions and exact compatibility acknowledgment; worlds untouched |
| Java world datapacks | WorldPackBrowserSheet, WorldStep; world-slot scoped routes | Both providers, explicit slot, edited content restoration, manual ZIP, dependencies; Bedrock unchanged |
| Immutable dated publication | ContentExportSheet; publication_review/content_publications | Date/revision collision handling, permission/config review, outside edits and historical independence |
| Player/Server content ZIP | content_archives; Export content audience | Only eligible audience folders, private/runtime/world exclusions, manifest/missing reasons, optional inactive content |
| Optional nested .mrpack | content_mrpack eligibility and export | Same frozen revision/files/setup, imports in actual compatible launchers; known restriction cannot be overridden |
| Local and remote destinations | Native transfer and content_export_saving; Export content destination | Remote agent → this computer; host exports folder; changed destination hash/review; partial writes/disconnect |
| CLI/TUI | phase24-cli.md; phase24-tui.md; `msc tui` → server workspace → Shift-P | Host-local/SSH behavior, creation staging, exact manual files, review choices, stale/old-agent states, export paths |
| Storage accounting/cleanup | storage discover rules and Storage view; existing transfer-owner exact cleanup | Parent/child/hard-link totals, active readers/reservations, unknown/rollback/reference protection, no history deletion |
| Offline launcher guides | Frozen text-v3-launchers HTML/text drafts; launcher-guides/README.md | **Missing:** owner screenshots, exact five launcher builds, illustrated integration and offline render/installation review |
| Cross-cutting safety/resources | Existing operation/auth/admission/journal boundaries and bounded copies | Revocation, cancellation/restart, disk full, concurrent edits/host switching, large packs and memory/disk measurements |
| Supported platforms | Build evidence on macOS Intel only | Actual macOS/Windows/Linux behavior, local/remote paths, launcher and terminal behavior; no platform pass inferred |

### Final UI route checklist

1. Add Server → Add-ons: both-provider individual browsing and modpack setup.
2. Components → Browse: same shared project sheet for both providers; no routine
   install-confirmation sheet. Exercise the conditional manual-file exception.
3. Components → Content sources and updates; diagnostic repair; legacy Add-ons
   Catalog: verify these use the same provider-aware flows.
4. Components → Player content: independent items plus retained recovery,
   update/replacement and individual upstream restore choices.
5. Worlds → slot → Datapacks: provider catalog and explicit modified-file restore.
6. Components → Export content: publication review, dated history, incomplete
   consent, eligibility and both save destinations.
7. Settings → Storage: retained content/publications/rollback and transfer cleanup.
8. Terminal → server workspace → Shift-P, plus creation's catalog action.
9. Open the exported README and instructions/INDEX.html offline. Current assets
   are explicitly text drafts; do not approve the illustrated-guide gate yet.

### Check record and remaining prerequisites

Permitted checks passed: `cargo fmt --all`, focused `cargo clippy -p msc-agent --
-D warnings`, `npm --prefix clients/desktop-web run check` (0 errors, 10 existing
CSS warnings), frontend build, native desktop `cargo check --locked`, and
`npm --prefix clients/desktop-web run prepare:agent` including local sidecar
staging. No test, smoke-suite, live content mutation, cleanup, service restart,
release workflow, tag or publication was run by the assistant in this handoff.

P24.22 added terminal counterparts; P24.23 added disjoint content accounting and
reference protection. Its existing tiny storage discovery fixture was extended
(expected <1 second, not run). P24.20 prepared five local-only draft guides while
keeping screenshots/build verification missing. Final integration inspection
found and repaired the older Add-ons catalog and diagnostic navigation gaps;
these reuse existing sheets and retain host/server-scoped API snapshots. Old
Modrinth-only help/TUI copy was corrected. No duplicate UI tests were added for
these bindings; physical navigation remains in the owner checklist.

Before phase acceptance: supply the five sets of screenshots listed in
launcher-guides/README.md, integrate approved assets under a new guide version,
record the physical scenarios above, and request the other agent's phase-gate
review. The implementing agent has not performed that independent review and
has not marked a step or phase accepted.


### P24.32 — Automatic full/server modpack export candidate (2026-11-07)

Owner amendment replaces the separate Player content action with Export modpack.
Full mode derives retained originals plus owner edits; server mode derives live
content. The export action freezes the dated revision automatically. Missing
content/known distribution restrictions remain explicit; unknown sharing rights
and changed configuration use one exact-byte owner attestation instead of a
classification action on every file. Previous dated exports remain immutable.
The normal UI no longer asks owners to classify supporting files. Pack maintenance
is still available from Components → action menu → Manage modpack.

Passed non-test checks: `cargo fmt --all`, `cargo clippy -p msc-agent -- -D warnings`,
`cargo check -p msc-application --test content_publication_safety` (compile only),
API generation, frontend check (0 errors, 10 existing CSS warnings), frontend
production build (existing CSS/chunk-size warnings), and development agent/helper
staging including the Intel BedrockSidecar build. New regression source covers
filtered client content versus intentional deletion, disabled/added/updated files,
coexisting active/disabled copies and read-only original preservation. It was not
run. No tests, service restarts, live server mutations or releases were performed.
Native desktop Rust code is unchanged; this step did not repeat its earlier check.

Development agent and staged copy SHA256:
`ad9e93ef5846b0adb0bba252d4b5e81ccea84d88734271a0f376049f63c96c73`.
Staged at `clients/desktop-web/src-tauri/target/Resources/agent/msc`; running
local/remote agents still need the owner's normal update/repair action. New clients
check `contentPackExports` before offering the revised derivation to an old agent.

Owner verification remains pending: export full/server modes after adding,
removing and disabling content; inspect organized ZIP bytes/instructions and an
eligible .mrpack; check active-world datapacks, local/remote save destinations and
historical re-export. A pack containing world-slot datapacks still cannot be
represented as a launcher-only .mrpack; the UI reports its existing eligibility
reason and the organized ZIP remains available. Older servers without retained
client bytes must still supply the original pack for missing client content.
Illustrated launcher guides and phase acceptance remain open.


### P24.33 — Export control theming and preparation progress (2026-11-07)

All export dropdowns now use MSC's shared Select control, including Save to;
the export action uses the shared primary button at the trailing edge. The
preparation operation reports percentage, stage and the current verification
file. Percentages count inspected work within weighted phases, not elapsed-time
predictions. Unknown-duration settings/world discovery reports its actual stage.

Opening the sheet reads persisted collection metadata rather than hashing the
entire server for its revision, and history comparison is deferred until Previous
exports opens. Immediate post-review and GET-preview rehashes are removed.
Preview still checks owner/server identity, metadata/setup/revision and expiry;
publication commit still recomputes the current fingerprint and verifies every
prepared source's exact bytes. Cancellation is checked between snapshot files.
No measured timing claim is made for the owner's physical pack.

Passed: Rust formatting and focused agent Clippy; essential publication regression
source compilation only; API generation; frontend check (0 errors/10 existing
CSS warnings) and production build (existing CSS/chunk-size warnings). The new
regression distinguishes display preview from commit and protects wrong-owner
refusal and stale-byte rejection; tiny controlled local inputs, expected <100 ms,
not run. No test suite, live mutation, service restart or release was performed.
Native desktop Rust is unchanged.

Agent build and staged copy SHA256:
`70371ce9050927a1ea0c970b8f40f7b2eb209a3a4dd3681124a4b6222fdfe589`.
The final development helper-staging outcome is recorded in the execution ledger.
The running agent still needs the owner's normal update/repair action for numeric
progress and the lighter metadata read. Owner visual/interactive verification is
pending for themed Save to, both preparation modes, cancellation, history and
changed-content refusal at export commit. No phase acceptance is inferred.
