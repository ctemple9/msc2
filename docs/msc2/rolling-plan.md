# MSC 2 — Rolling Plan

## Conversation goal — finish mod assets, verify, publish

**Owner instruction (2026-10-05):** By the end of this conversation, finish the feature implementation, test it, and publish the next release. Guide Cameron one step at a time. This goal persists through compaction; successful builds, completed batch conversations and collected receipts do not close it. Release publication is authorized as the final action after the feature gate holds. Select the next unused release version from authoritative remote release/tag state then; v0.1.23 is only the latest local tag observed during planning.

**Continuation state:** P18.15–P18.19 contain implemented resource acquisition/import/repair work awaiting owner acceptance. P18.20 contains a building isolated Forge fixture, not a production remedy. P18.21 contains a collector with zero production observations. The NeoForge isolated appearance proof is owner-confirmed. Remaining work is captured below; earlier records remain historical and are not silently marked Done.

### P18.22 — Plan the production completion and release sequence

**Status:** Written; awaiting Cameron's review.
**Files:** `docs/msc2/rolling-plan.md` only.
**What:** Record the persistent goal, inspect actual production gaps and define six detailed continuation steps in three batches. Source inspection confirms resource-only bundle import, no adopted supplemental geometry, no production runtime capture remedy, no Fabric production helper, and outstanding native/platform evidence. Preserve all original fixture/repair/platform requirements. No source code, test execution, game launch or release action in this planning move.
**Verify:** `git show --check --stat --oneline --grep='P18.22' HEAD`
**Batch:** Planning — P18.22 only; owner reads this plan before execution.
**Commit:** `P18.22: plan production mod assets completion and publication`

### Implementation decisions and risks

- Extend the existing host-bound resource workflow with a separately versioned supplemental mesh manifest. Keep baseline terrain decoding and resource-only bundle imports compatible. Transfer checked data only; headless agents never execute client mods or require a GPU/login. Reuse the proven client renderer capture mechanism, not the fixture's hardcoded coordinates or mod identities.
- A capture needs independently checked correspondence to an authorized saved snapshot, dimension, bounded area, game/loader/mod/resource/configuration identities and context. A client merely asserting a server/world name or hashing its own unrelated save is insufficient. For online-only private context, describe the limitation and provide a matching isolated snapshot route; never relabel current online appearance as an older saved snapshot.
- Adding captured geometry over unresolved opaque cubes can hide it or duplicate faces. Adoption must suppress only the verified replaced geometry and rebuild its necessary neighbor faces. Preserve intentional empty models, fluids, unrelated blocks, depth/cutaway and dimension behavior. Equal block states with different contents require position/context-specific keys.
- The fixture only covers a subset of materials. Explicitly encode and validate topology, tint/light, alpha/culling/depth/emissive properties and saved-frame time; refuse unsupported shader behavior with its exact cause. A missing or unsupported required fixture keeps the gate open. Required Forge/Fabric/NeoForge examples need working remedies, not a global unsupported label.
- Existing synthetic resource adaptation and custom-dimension handling require appearance checks. Inspect them before extending them; source-level namespace resolution does not prove correct weighted variants, multipart, pack priority, transparency or connections. Correct observed defects within these steps, without modifying original worlds or official helper checksums.
- The 18 transport and 222 coverage cells are evidence bookkeeping, not instructions for 240 independent runs. Automate preparation, shared receipts and capture reuse; group observations from the same pinned session. Preserve every required cell's meaning and require owner confirmation for appearances. Missing access to a native platform remains a concrete pending requirement.
- Broad permission to finish and test does not name an exact test command. Under the testing policy, propose the smallest relevant exact commands once targets exist and obtain specific authorization before running them. Non-test builds, formatting, Clippy and type checks are permitted. Keep release publication build-only.

### P18.23 — Validate, import and adopt contextual geometry in production

**Status:** Implemented; awaiting Cameron’s verification. No production visual acceptance or test execution claimed.
**Files:** `crates/msc-domain/src/map_assets.rs`; `crates/msc-infrastructure/src/map_assets/` including a new supplemental-format module; `crates/msc-agent/src/routes/worlds/map_assets.rs`; `crates/msc-agent/src/routes/worlds/map_terrain.rs` and `map_terrain/preparation.rs`; existing contract and generated API types; map viewer integration; essential controlled regression source; design/usage/acceptance records and this plan.
**What:** Define a versioned mesh/material/texture manifest and a bounded request receipt carrying authoritative binding, snapshot, input identities, dimension/area and required neighbor context. Reject unknown versions, stale or different-world receipts, unsafe/case-colliding paths, executable payloads, duplicate references, nonfinite/out-of-bounds geometry, malformed indices/images/materials, missing hashes and excessive bytes/counts. Verify real bytes and context against the agent's saved snapshot before publication. Keep resource-only bundles readable. Store immutable candidates, validate outside shared locks, and publish coherent terrain plus supplemental geometry atomically through existing cancellation/revision guards. Expose authenticated generation-bound artifacts and capability negotiation. Integrate supplemental draw objects into the actual map with the correct position and material semantics; remove verified replaced fallback geometry and recover neighbor faces without hiding unrelated terrain. Retain the previous scene on malformed/import/render failure, supersession or cancellation; dispose retired GPU resources. Add essential controlled regressions for forged binding/context, malicious geometry/paths and failed-candidate preservation; explain concrete risk and expected runtime, but do not run them without exact authorization.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && npm --prefix clients/desktop-web run check`
**Batch:** Completion A — P18.23–P18.24; proceed only while current-step non-test checks pass. Owner may choose single-step execution.
**Commit:** `P18.23: adopt validated contextual map geometry`
**Acceptance:** Import a real previously confirmed NeoForge capture through production staging and render it inside MSC at named saved positions. An unrelated/stale capture must fail with the existing scene intact. Owner visual confirmation remains separate from compilation. Do not promise this command until the importer exists.

**Implementation (2026-10-05):** Added the msc-contextual-mesh-1 domain/contract, read-only capture-request endpoint, exact saved-snapshot/input comparison, bounded ZIP/mesh/PNG validation, independently observed source identities, immutable scene-owned capture artifacts and guarded terrain/capture/report adoption. The private palette adapter replaces only validated positions, repacks width changes, preserves waterlogged fluid and fingerprints the neighbor apron. The production viewer loads capture candidates with checksums/bounds, displays saved time/tick, shares depth clipping and disposes failed/retired resources. Captured scenes require explicit reader-format negotiation; old resource bundles and no-capture Java/Bedrock paths retain their behavior. Reports and the collector recognize captured_appearance separately from model resolution and owner visual success. Runtime capture leases last 30 minutes for export/upload; durable helper output reuse remains P18.24 work.

**Essential regressions:** Three controlled cases in `crates/msc-infrastructure/tests/map_capture.rs` cover forged host/snapshot/resource/state/source identities and duplicates; malformed geometry/images/archives/materials and cancellation preserving a prior capture; and per-position packed-palette replacement across a width change, including negative coordinates and waterlogged fluid. Existing coordinator tests already cover failed/stale publication, so no duplicate coordinator-only test was added. Fixtures use generated bytes, unique temporary directories and cleanup, without network, clocks, installed games or timing assertions. Expected runtime below two seconds after compilation. Tests are compiled only until Cameron specifically requests `cargo test -p msc-infrastructure --test map_capture`.

**Acceptance still pending:** No Minecraft was launched, source world altered or actual production map comparison performed. The previous isolated proof uses a different unbound format and is not automatically trusted or relabeled. P18.24 must produce a matching source-bound bundle and real owner-run command; P18.25 wires the successful repair action; P18.26 supplies the original platform/repair/preservation observations. No tests, release pushes/tags or publication actions in this step.

**Agent checks:** `cargo fmt --all -- --check`, production agent Clippy and targeted infrastructure regression Clippy passed with existing warnings only (unused uninstall request, Bedrock mutable variable and auth helper). `cargo check -p msc-infrastructure --test map_capture` compiled the three cases without running them. Svelte check passed with zero errors and eleven existing warnings; frontend production build passed with its existing large-chunk notice. API types were regenerated and checked against the updated contract; changed frontend formatting, contract JSON, collector compilation and `git diff --check` passed. No test suite, game/client, release workflow or remote operation ran.

**Owner verification received (2026-10-05):** Cameron ran the P18.23 `map_capture`
regression target and reported all three tests passing, zero failures, in 0.05s.
This verifies those controlled data/adoption cases; it does not establish visual
or native platform acceptance. Step closure remains Cameron's decision.

### P18.24 — Deliver matching-client exporters for all required loaders

**Status:** Implemented; awaiting Cameron’s verification. Real rendering/native platform acceptance remains pending.
**Files:** Domain capture request, infrastructure preparation/saved-context/cache modules and focused regressions; `tools/java-map-export/` common driver, manifest/pins, loader-specific helper sources and licenses; existing isolated proofs remain preserved; native desktop map helper module and registration; headless bundle import/export CLI; helper staging/install/update packaging scripts; design/usage/acceptance records and this plan.
**What:** Convert proven NeoForge and building Forge mechanisms into bounded production helpers and implement the matching Fabric adapter. Use real baked-model/block-entity entry points and captured texture output. Support the minimum/current loader/version families required by the original acceptance fixtures; pin exact ranges and refuse incompatible clients. Replace hardcoded proof fixture/mod/coordinates with explicit request receipts and a private dedicated working instance derived from matching selected inputs. Keep source client/server worlds read-only; never install into an ordinary instance or start game code during passive map opening. Capture only authorized saved area/neighbor context; refuse unloaded/generated/mismatched chunks and private context that cannot be established. Export reuseable model data separately from position-dependent content and label frame/time. Provide portable Java/Gradle selection and a build-only driver that explicitly selects compile/package tasks; no tests or implicit game tasks. Native launch must handle spaces, Unicode paths, Windows process trees, cancellation and helper failures. Stage versioned helpers with truthful checksums/licenses in all supported desktop/headless distributions without removing baseline helpers or adding workflow test gates. Headless import consumes data without running the game. Prefer shared driver/schema code to three separately maintained workflows. Record actual commands for builds and each owner-run fixture capture after implementing them.
**Verify:** `python3 tools/release/stage-map-capture.py --build-only && cargo fmt --all -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib && npm --prefix clients/desktop-web run check`
**Batch:** Completion A — P18.23–P18.24. Compile/package every declared supported helper target with its newly documented build-only command before handoff; update Verify to the actual driver command once it exists.
**Commit:** `P18.24: deliver matching client map exporters`
**Acceptance:** Named Fabric, Forge and NeoForge blocks including custom loaders and equal-state/different-content objects capture, transfer and display in production MSC. Native launch/transfer acceptance is required on Windows, Linux and both macOS architectures. No loader range is advertised solely from another loader's proof.

**Implementation record (2026-10-05):** Shared production protocol/driver generates
Fabric, Forge and NeoForge adapters at exact pins: Fabric Minecraft 1.20.1 /
loader 0.16.14 / API 0.92.5+1.20.1, Forge 1.20.1 / 47.4.10, and NeoForge
1.21.1 / 21.1.251. Compiled helpers verify actual running versions and the real
resource manager. Runtime capture uses baked models, actual entity renderers,
position-specific geometry and shared texture/material data. Reusable static
models/resources remain in the existing resource-bundle/baseline path; contextual
per-position content is a separate capture package, preventing equal states with
different contents from aliasing. It restores saved
states, entities, time/weather in the private working save, freezes simulation,
completes bounded lighting work, and explicitly sends restored context before
capture readiness. Atlas primitives are cropped losslessly with filtering margins
from sources up to 8192 pixels; unsupported/repeating/shader/renderer behavior and
exceeded budgets refuse with a bounded private diagnostic. Isolated proofs remain
unchanged.

The host context route exports authorized raw chunk records, original level data,
resource/approved-client manifests and bounded saved server configuration,
datapacks and world data. Delivered snapshot and auxiliary bytes are bound to the
request; unused global host configuration values are excluded. Approved client
closure is independently inspected again in the clone, so server-only mods do
not become client dependencies. Complete copied input trees, configuration,
component pins and metadata are checked against changes during preparation.
Directories are privately and atomically owned; cancellation cleans candidates.
Native process groups/Windows jobs own launch trees, uncertain termination retains
files with a durable refusal, and retained outputs can be reopened. Selected client memory settings are reused, with an explicit optional
`--memory-mib` override; launch commands/authentication settings are excluded.
Neither map opening nor preparation launches Minecraft. The Java version probe alone is
explicitly bounded/cancellable. CLI resource export/context export/preparation
and existing data-only import provide the headless handoff; visible guided repair
is still P18.25.

Durable capture reuse revalidates every hit and binds all saved/client/context
identities; only the base geometry generation is rebased. Cache errors cannot
reject valid adopted candidates or break baseline rendering. Two essential
controlled regressions cover cache reuse/corruption preserving leased artifacts
and region export excluding unrelated chunk records without source mutation;
the existing forged-context case also covers auxiliary context identity. Each
new case should run below one second; no timing, network or game assumptions.
Tests were compiled only. Cameron’s earlier three-case passing result remains
recorded above; it does not execute the new cases.

Packaging stages all three versioned/checksummed helpers and licenses across
existing desktop/headless distributions, and embeds the payload into release
binaries to cover older fixed-file archive updaters. Existing artifact/signature
requirements and baseline dependencies remain. Installer checks occur before
service stops or binary replacement; updates retain recursive validation/rollback.
The existing release workflow adds build dependencies and compile/package actions
only. No release workflow was launched.

**Evidence and limits:** Agent/native Clippy, focused regression compilation,
Svelte/API checks and Python/Node/Bash/YAML/whitespace checks pass with existing
unrelated warnings only. Actual Windows process-module cross-compilation passes;
that is not Windows execution. The three adapter builds and complete real-payload
staging pass; Rust agent and native checks with the final embedded three-helper payload also
pass. Final helper source receipt is
`93d713cccf285d9f7efedf04815e45d3120d6694a634c2ddef198009cba27f62`;
packaged Forge/NeoForge/Fabric JAR sizes are 45,619 / 43,386 / 46,777 bytes.
Cached Minecraft source/bytecode confirms client rendering drains light queues
outside the frozen client tick; this source check remains distinct from a launch.
No new test execution, Minecraft launch, source-world change, remote action,
push/tag or publication is claimed. Other versions, alternate Fabric renderers,
entity/global-service context beyond this saved scope and all physical platform,
appearance, preservation, successful-repair and resource-budget observations
remain the original P18.26 gate. No required fixture is substituted or closed.

### P18.25 — Connect one successful guided repair process

**Status:** Planned; depends on Completion A.
**Files:** Existing `MapAssetsRepairSheet.svelte`, `WorldMapViewer.svelte`, native commands, frontend map state and operation tracking; map-asset API/CLI handlers and repair evidence; design/usage/acceptance records and this plan.
**What:** Read `antiAIslop.md` before UI work. Replace the current dead-end runtime-model explanation with the working Use Minecraft rendering action. Identify the exact matching source/context needed, preview the bounded capture request, explicitly start the selected private client only after user action, transfer output through existing authenticated staging, prepare terrain, adopt it and recheck the same affected blocks. Expose equivalent request/bundle commands for headless operators and remote Fedora-to-Ubuntu use. Keep progress/cancellation and host/world identities throughout; ignore late events after switch/close. Make missing resources, wrong versions, pack order, stale tiles/context, custom loaders and code-driven blocks lead to their applicable working remedy. A successful upload alone never yields repaired; require validated geometry and resolved relevant diagnostics in the same saved scope, plus accurate visual-verification status. Preserve navigation, camera/depth/player polling, baseline maps, compatible rollback and offline retries. Publish only exact supported ranges and remaining material/animation limitations derived from implementation/evidence.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib && npm --prefix clients/desktop-web run check`
**Batch:** Completion B — P18.25–P18.26.
**Commit:** `P18.25: connect successful minecraft rendering repairs`
**Acceptance:** Cameron fixes one genuinely broken difficult block from the repair sheet inside MSC, compares it with Minecraft, and verifies unaffected blocks and scene continuity. The CLI remedies work on a headless remote agent. Any observed failure is fixed before moving past it.

### P18.26 — Test behavior and record real platform acceptance

**Status:** Planned; depends on production repairs.
**Files:** Essential existing/new regression targets only where risk is uncovered; acceptance collector/README if evidence format needs extension; `docs/msc2/world-map-assets-acceptance.md`, consolidated Phase 18 acceptance, usage/support records and this plan.
**What:** Inventory existing meaningful regressions for resources, import/adoption, supplemental validation and repair success; propose exact focused commands with risks covered and expected cost, then run only those specifically authorized by Cameron. Do not use source-text assertions or mirror implementation. Automate real fixture preparation and receipt capture, retaining owner-only appearance confirmation. First prove Fedora desktop to Ubuntu headless end to end with static and difficult models, then run the original native Windows, Intel/Apple Silicon macOS, Linux and all OS-pair transport scope using reusable pinned bundles/sessions. Cover all six Java flavors, saved custom dimension, representative minimum/current versions, missing/corrupt/wrong/order/stale repairs, cancel/offline/retry/rollback and binding isolation. Record vanilla, Paper/Tectonic, Purpur and Bedrock controls, material fidelity/navigation, first-use/cache behavior and actual resource costs against budgets. Package/installation evidence is distinct from builds. Fix concrete gate failures within this batch, rerun only affected checks, and record absent machines/credentials/fixtures as pending inputs. Do not manufacture observations or downgrade original gate requirements. Guide Cameron one real command/action at a time and retain logs/receipts across compaction.
**Verify:** `python3 tools/world-map-assets/collect-acceptance.py --help`
**Batch:** Completion B — P18.25–P18.26. This Verify confirms collector availability only; exact owner-authorized tests and actual sessions must be recorded in the step's evidence before acceptance.
**Commit:** `P18.26: record verified mod rendering and repair behavior`
**Acceptance:** Original required rendering, repair, preservation, transport and resource-budget rows have authoritative observations and owner visual results, with no required pending/failing row. Build receipts and test passes alone do not close it.

### P18.27 — Independently review the feature gate

**Status:** Planned; depends on P18.26 acceptance evidence.
**Files:** Independent review report and this plan only; review does not change implementation.
**What:** Have the other agent inspect the original promises, actual code, manifests, test outputs, production captures, repair evidence, supported loader/platform ranges and packaged-helper assumptions. Check the end behavior and data boundaries, not merely completed step statuses. Explicitly audit missing geometry, context correspondence, cancellation/old-scene preservation and every required unresolved row. Report findings without fixes. If defects remain, return to the affected implementation step through an explicitly numbered corrective step/commit, repair and reverify before review closes. Cameron closes step statuses; the implementer cannot substitute its own review.
**Verify:** `git show --check --stat --oneline --grep='P18.27' HEAD`
**Batch:** Completion C — P18.27–P18.28; independent review is a separate move/conversation under repository policy.
**Commit:** `P18.27: record independent mod assets gate review`
**Acceptance:** Independent review finds the actual feature gate holds; all findings affecting the goal are resolved with evidence. Existing unrelated Phase 16/17 deferred gates are not silently closed.

### P18.28 — Publish the next immutable release and verify delivery

**Status:** Planned; depends on feature acceptance and independent review.
**Files:** Owned version manifests/lockfiles, release notes and exact-artifact acceptance records, usage/support documentation, packaging files only if proven necessary, and this plan.
**What:** Inspect remote main/tags/releases and the existing build-only publication workflow; select the next unused version. Align owned versions without modifying dependency versions, preserve all nine release artifacts, baseline/new helper payloads, licenses, checksums and signed update metadata. Explain expected build cost and known uncertainty before starting the one publication attempt. Commit release preparation once, push authorized commits and one new immutable tag, monitor the same workflow handle to its terminal result, inspect all failures before any targeted fix/retry, and never move the tag. Verify release publication, complete downloadable artifact set/checksums/signatures and helper inclusion; perform owner-run install/update/rollback checks on the exact published artifacts required by the feature acceptance. Keep build success, publication and physical artifact acceptance distinct. Do not add CI/test/lint gates. Mark the conversation goal complete only after all implementation/testing/acceptance/review/publication requirements are actually proven.
**Verify:** `git show --check --stat --oneline --grep='P18.28' HEAD`
**Batch:** Completion C — P18.27–P18.28. Publication is already owner-authorized; no repeated permission request solely to push the completed release.
**Commit:** `P18.28: prepare verified mod assets release`
**Acceptance:** New release is publicly published with complete authentic artifacts; required exact-artifact checks pass. Record version, commit, immutable tag, workflow URL/status and acceptance evidence. A queued workflow or prepared tag is not a published release.

**Current work:** P18.23 production capture intake/adoption/display is implemented and awaiting Cameron’s verification; focused regression execution requires his specific command instruction. P18.24 helpers and durable exporter reuse are next, not started. Full appearance/platform/repair acceptance and publication remain open. Cameron visually confirmed the isolated P18.14a proof on 2026-10-04. P18.15–P18.19 are implemented and awaiting Cameron's verification. P18.20’s Forge access-rule correction now compiles/packages successfully; its isolated capture remains awaiting Cameron’s verification and broader production integration is incomplete. Cameron authorized proceeding to P18.21 evidence preparation; the read-only collector and acceptance documentation are implemented, awaiting Cameron’s verification. Broader production/platform/mod acceptance gates remain open.

### P18.13 — Plan automatic modded map assets and successful repairs

**Status:** Written; awaiting Cameron's review.
**Files:** `docs/msc2/rolling-plan.md` only.
**What:** Record the implementation analysis, eight detailed execution steps, four batch boundaries, preservation requirements, and the final all-platform rendering/repair gate requested by Cameron. No implementation, new test source, workflows, dependencies, version changes or releases in this planning conversation. This is a continuation of Phase 18, not an assertion that its deferred acceptance or independent review is complete.
**Verify:** `git show --check --oneline --grep='P18.13' HEAD`
**Batch:** P18.13 only — planning conversation; stop after the documentation commit.
**Commit:** `P18.13: plan automatic modded map assets and repairs`

**Checks:** Inspected the changed document and whitespace. Checked that the planning entry and all eight execution steps carry Status, Files, What, Verify and Batch; confirmed four paired batches and no test-running Verify commands. Documentation only; no tests, builds, source changes or release actions.

## Java mod assets — analysis and intended outcome

**Owner direction, 2026-10-04.** Mod assets should "just work" on all supported platforms, working maps must keep working, and troubleshooting must lead to actual rendering success. Cameron requested more detail **per step**, not a long sequence: eight execution steps in four two-step batches. A download, an informational warning, a fallback cube, or an exported diagnostic report alone does not satisfy the repair goal.

### What is established, and what is not

- P18.12/v0.1.23 fixes the headless updater omitting Vantage/Bedrock helpers and prepares official, version-matched vanilla client assets. All six Java flavors use the same terrain route. It does not collect mod namespaces, selected client resource packs or mod-generated models. Its successful checks are build/type/format evidence; owner live acceptance remains separate.
- `tools/world-map-proof/stage_java_mod_assets.py` extracted ATM10 Lite client assets for a private proof. It flattened namespaces, used alphabetically sorted JARs, retained the first conflicting asset, required manually named dependent namespaces, and supplied placeholders for particular blocks. These are proof shortcuts, not a production resource-selection policy. Do not promote the script unchanged.
- `audit_java_terrain_assets.py` calls a state `exact` after finding geometry and texture files. That proves resolution, not the game's shape, transparency, lighting, tint, culling, connections or animation. Keep historical evidence intact; new reporting uses `model_resolved` and separate visual acceptance.
- Mod namespaces are resource prefixes such as `create:`. Vantage 0.15.1's flat lookup means simply extracting `assets/<mod>/...` will not establish successful rendering. A namespace-preserving renderer or a collision-free, format-aware adapter is a prerequisite, not a final cleanup task. The source world must never be rewritten to implement it.
- A namespace is not a unique mod identity: one JAR can contribute several namespaces, and another JAR or pack can override them. Model parents, texture variables, metadata, nested JAR resources and cross-namespace references matter. An asset file's presence is not proof that it is the selected resource in the game.
- Existing modpack identity records name/provider/version, which is insufficient to reconstruct every exact client download. Preserve original provider release/file IDs, hashes and client manifest/overrides when available; otherwise use verified individual-file identification or an explicit client import. Never infer a modpack identity from installed JAR names (D-030's proposed downstream contract and current product behavior).
- MSC's server importer intentionally excludes client-only files. The map asset collector must be a separate read-only client-resource consumer, never a change to that classifier or an installation of client-only mods into the game server.
- Resource selection cannot be inferred completely from the dedicated server. Client pack selection/order, configuration-driven assets and resources generated during game initialization may require an import/export from a matching client. Forge/NeoForge custom model loaders and code-driven block-entity renderers are separate problems from missing PNGs.
- The current Java compatibility helper handles the modern standard dimensions but returns the original world for other dimensions. A saved mod dimension must be included in the new compatibility proof; a working Overworld is not sufficient evidence for modded-world support.
- Headless hosts must not acquire a graphical desktop, GPU runtime or Minecraft login requirement for ordinary mod assets. If running Minecraft is needed for a difficult rendering case, do it through the user's matching client and transfer only the validated map output to the agent.

**Primary sources inspected for this plan:** [Modrinth pack specification](https://support.modrinth.com/en/articles/8802351-modrinth-modpack-format-mrpack) defines client/server environment distinctions, exact hashes, client overrides and download inputs; [Fabric block models](https://docs.fabricmc.net/develop/blocks/block-models) defines model resources/references; [NeoForge custom model loaders](https://docs.neoforged.net/docs/resources/client/models/modelloaders/) and [block-entity renderers](https://docs.neoforged.net/docs/1.21.11/blockentities/ber/) establish that available JSON/textures do not cover every rendered appearance. Local evidence is in the proof scripts, `tools/world-map-proof/README.md` and `phase18-map-acceptance.md`.

### Completion promise and honest boundaries

The target is an **automatic normal path plus a successful guided recovery path**, on every shipped platform. On first map open, MSC reads available inputs, obtains exact missing client resources when resolvable, builds the resource stack and displays saved terrain. When automatic recovery needs something only the client can provide, the action must identify it, import/export it, rebuild the affected map and demonstrate success. Users should not have to use SSH to install assets, rename resource files or manipulate renderer cache folders.

“All platforms” means native Windows x86_64, Linux x86_64 (Fedora desktop and Ubuntu headless included), and Intel/Apple Silicon macOS, through supported desktop and headless interfaces. Unsupported distribution architectures are not silently added. All six Java flavors remain usable; real block-adding Fabric, Forge and NeoForge content must be covered, not just those loaders running vanilla terrain. Bedrock regression checks remain required because the viewer and helper delivery are shared. Exercise Bedrock on every desktop via a supported BDS host; this does not add local Bedrock support to Apple Silicon macOS.

No finite acceptance set can prove every present/future mod or every arbitrary rendering program. **This is a feasibility limit, not permission to replace Cameron's goal with a list of unsupported blocks.** Required representative custom-loader and code-driven examples must have real successful remedies. If P18.14 finds no viable route for them, record the exact failure and concrete alternatives, and stop before wiring a misleading repair UI. If the final matrix still contains a required unresolved case, the feature gate stays open. A broader universal claim needs evidence beyond this plan, not a wording change at the end.

Default advanced output is proposed as an accurate **saved appearance at capture time**, consistent with the map's saved-terrain semantics. A frozen animation, stale connected state or missing block-entity contents must not be represented as an exact/live result. Confirm this presentation during the plan review; exporter feasibility must explicitly state what can/cannot be captured without changing that promise.

### Preservation and architecture requirements

1. Keep the v0.1.23 vanilla path and known-good asset generations intact. No-mod/no-custom-pack worlds use the established path. Plain Paper/Tectonic remains a mandatory control case. Loader flavor alone does not activate a mod renderer; explicit packs or actual required resources do.
2. Store map preferences separately from game configuration/world profiles under MSC data, keyed by host, stable server/slot identity and input fingerprints. Use immutable resource generations and staged publication. Validate binding on slot switch/restore/import; never apply another host/world's resources because its display name happens to match. Shared content bytes may be deduplicated, but authorization/bindings cannot be shared accidentally.
3. Preparation reads game/mod/client files only. It may write private scratch terrain and MSC-owned caches. It must not modify the game save, JARs, mod enablement, server resource-pack settings, launcher settings, or MSC 1; no game-server/agent restart for an asset repair. Reuse existing consistent terrain capture when a new snapshot is actually needed, rather than promising zero pause for an operation that performs save/copy.
4. Candidate assets, geometry and diagnostics must be from one compatible generation. Prepare outside the global renderer lock, retain the visible map, then atomically switch once a candidate is ready. A failed/cancelled job preserves the current scene and reports the error. After a version change, explicitly label a retained old scene as stale; do not silently render new block states using incompatible old assets.
5. Use private, bounded artifacts: checked paths, no links, cross-platform filename/case handling, bounded decompression/reference recursion/image sizes, checksummed downloads, bounded redirects, disk budgets and cleanup. Local hashes identify imported bytes; they are not publisher verification. Never upload launcher accounts, access tokens, unrelated files or a whole private world by default.
6. Diagnostic counts distinguish unresolved textures/models, unsupported features, intentional empty geometry, missing saved chunks and material/culling errors. Name the inspected dimension/area/snapshot and count types; do not imply world-wide coverage or infer visual correctness from a texture lookup. Fallback blocks remain explicitly distinguishable and must not erase terrain or incorrectly hide neighboring faces.
7. Reuse the Worlds permission, existing operation journal/progress/cancellation, shared staging and host-bound native transport where appropriate. Background work must not block health, player polling, map navigation or the game. Resume/retry must not repeatedly download identical verified bytes or re-run the same failing action without a changed input.
8. Internal rendering adapters/helper tools are MSC-owned implementation details, not a third-party plugin API. The optional exporter is a repair tool, not a required client map mod for ordinary maps (engineering §21). The agent remains usable headlessly; the CLI can accept a portable bundle exported on another machine.
9. UI follows `antiAIslop.md`: integrate quiet rendering status and concrete actions into the existing map, with one bounded repair sheet and clear progress. No decorative dashboard, repeated warnings, status dots or generic troubleshooting wizard. Keep camera/follow/player/dimension/exit behavior intact.

### Proposed interface and automated work

P18.14 freezes these names or records a justified adjustment before implementation; they are proposed interfaces, not commands already available:

| Surface | Proposed operation | Result |
|---|---|---|
| Worlds API | Read map-asset status/report; start check/prepare/import/repair/restore operations; update selected map resource packs | Every request names/binds the stable server/slot and current generation; long work returns an existing operation ID. No arbitrary world/client paths. |
| Headless CLI | `msc world map-assets status`, `check`, `prepare`, `import <bundle>`, `repair`, `select`, `restore`, `report` | Uses the established active-server selection and explicit world binding; `--json` exposes the same typed results/progress as desktop. No separate full-screen terminal UI. |
| Desktop | Map status -> Check rendering -> one repair sheet, with native source chooser/exporter when needed | Matching backend action, progress/cancel, then recheck the same affected area and adopt the corrected scene. |
| Input bundle | Versioned manifest plus content objects, game/loader/source hashes, ordered selected packs and optional context-bound export | Each imported byte/object is validated and bound to the selected agent/world; credentials/launcher-private files are excluded. |

Automatic work includes source inventory, available exact downloads, dependency closure, candidate validation, compatible generation adoption, changed-input invalidation, bounded retry/resume and post-repair rechecks. User action is reserved for selecting an unavailable exact client source, resolving a genuinely unknown resource selection, explicitly using the matching Minecraft client for code-driven capture, and confirming visual acceptance. Do not ask separately for every asset file or interrupt an execution batch for routine implementation choices.

The implementation must collect identities, diagnostics, before/after counts, operation results, metrics and map capture references automatically into a redacted acceptance report. P18.21 adds a small read-only evidence collector so Cameron does not transcribe every platform result. It must label cells pending until the relevant platform actually supplies evidence, and visual rows pending until Cameron confirms the named appearances. A collector cannot manufacture a platform pass from one host's results. This is not an active CI workflow or a mandatory release gate.

### Batch execution and verification policy

| Batch | Steps | Reviewable result | Boundary |
|---|---|---|---|
| A | P18.14–P18.15 | Viable renderer/export route, exact acceptance contract and useful read-only diagnostics | Stop for a real feasibility blocker; otherwise no extra approval between steps. |
| B | P18.16–P18.17 | Automatic exact asset preparation and guarded rendering adoption | Existing maps survive preparation failures. |
| C | P18.18–P18.19 | Cross-machine client imports and working desktop/CLI repairs | Missing resources/version/order problems visibly repaired. |
| D | P18.20–P18.21 | Difficult-model recovery and all-platform acceptance evidence | Feature stays open until Cameron records actual rendering/repair success. |

Each execution step has one commit, including its rolling-plan update; leave its status **Implemented; awaiting Cameron's verification**, never Done. Batch execution runs each permitted non-test Verify command before the next step. Stop on failure and report the cause. The owner can request a whole named batch; do not invent a separate conversation for every file or helper. Work past a recorded feasibility blocker or an explicitly owner-run proof requires its actual resolution, not an elapsed wait.

No Verify command below runs a test suite. Essential regressions may be added only for the concrete risks listed; reuse existing coverage first, use controlled inputs and record runtime. Compile focused sources when useful; do not run tests/rendering smoke suites unless Cameron specifically requests the exact command. The known unrelated `cli_service.rs` all-test-target compilation failure must not become a reason to run/fix unrelated suites. Manual visual checks are consolidated at batch boundaries and final acceptance. Builds and source inspection must not be recorded as visual passes. These checks do not become release workflow gates; no release/tag/workflow changes belong to these eight steps.

## Detailed execution plan — eight steps

### P18.14 — Establish the renderer route and acceptance fixtures

**Status:** Feasibility analysis recorded; blocked on difficult-model capture/adapter proof; awaiting Cameron's verification of the recorded findings. Not complete.
**Files:** `docs/msc2/world-map-assets-design.md` (new), `docs/msc2/world-map-assets-acceptance.md` (new), existing Java proof scripts and `tools/world-map-proof/README.md`; isolated prototype files under `tools/world-map-proof/` only if needed; this plan. Production renderer paths stay unchanged in this step.
**What:** Resolve the high-risk engineering choices before acquiring thousands of resources or building UI:

- Inspect the pinned Vantage resolver, block-state dispatch, tile geometry/material format and viewer decoder. Choose a documented, reproducible namespace route: prefer a namespace-aware extension; a private-copy adapter is acceptable only if its mapping is bijective (each original identifier has exactly one reversible translated name), it follows recognized resource-reference fields and preserves arbitrary mod data. Demonstrate that duplicate local names, parent chains and cross-namespace references can be represented; do not reuse the proof's blanket string rewriting.
- Inspect custom model-loader and block-entity capture entry points for Fabric, Forge and NeoForge, using the actual supported loader/game versions. Establish whether the existing tile format carries the required geometry, UVs, face/material properties, tint, bounds and positional state. Propose the smallest private format extension if necessary; preserve backward decoding for baseline Java/Bedrock. Decide the reproducible build/pinning path for any changed/additional executable; the current official Vantage checksum must never be attached to different bytes.
- Establish a viable difficult-model remedy using a matching client or a specific internal renderer adapter. A catalogue of ordinary JSON models is insufficient. Address how a snapshot-dependent object obtains its contents/neighbors, how unvisited chunks are handled, and what an exported animation means. A logged-in active client is not assumed available on a headless agent. Do not automatically start arbitrary mod code during a passive map scan.
- Write the acceptance contract and fixture catalogue before implementation. Include Vanilla/Paper-Tectonic/Purpur controls, block-adding Fabric/Forge/NeoForge examples, ATM10 or a similarly pinned mixed pack, collision/dependency/order examples, a custom model loader, a code-rendered block entity and a saved custom dimension. Each entry records exact game/loader/mod/pack versions and hashes, named coordinates/block states, the expected appearance in Minecraft, the expected repair action and who supplies the private fixture. No private worlds or third-party game assets enter Git.
- Freeze the product/API design: per-server/slot resource binding, source evidence, immutable generations, state transitions, repair outcomes, cancellation, capability/version negotiation and the proposed `msc world map-assets` grammar. Agree on operation/progress data and the report scope. Sketch native client import and advanced export boundaries, including the minimal fields transferred. Record which context changes invalidate a resource or geometry generation.

**Failure/stop rule:** If namespaced/difficult geometry cannot be represented or captured with the chosen renderer, record the failing case and a concrete alternative with compatibility/cost implications. Stop before B/C, rather than promising that import fixes it. Use existing proof evidence/source/builds first; if a new live renderer/export proof is essential, give Cameron its specific command. Do not execute it under a generic Verify line or mark it passed from compilation.
**Verify:** `git show --check --stat --oneline --grep='P18.14' HEAD`
**Batch:** A — P18.14–P18.15; feasibility blockers must be resolved before progressing into B.
**Commit:** `P18.14: define modded map renderer and acceptance contract`
**Acceptance evidence:** The design maps every required rendering class to an implementable success path and lists any pending owner proof. It identifies a supported solution for the two same-name namespace blocks and at least one contextual/code-rendered object. Named private fixture/version availability is recorded; an unavailable fixture remains a blocker for final acceptance, not a waived requirement.

**Execution result (2026-10-04):** Added [renderer/resource contract](world-map-assets-design.md) and [acceptance catalogue](world-map-assets-acceptance.md). Inspected Vantage v0.15.1 at upstream commit `953f0ac78d62de751a935b8e8d5be2d711d6fd56`, including model dispatch, namespace stripping, state-keyed mesh cache, tile topology and viewer material/decoder contracts. Selected a namespace-aware private resolver extension; baseline production paths remain untouched. Recorded conditional API/CLI grammar, bindings, generation fingerprints, progress/outcomes, quota policy and exact fixture-receipt requirements. Exact private hashes/mod/loader pins/coordinates unavailable on this host remain pending, rather than invented.

**Feasibility blocker:** The current resolver/cache cannot render two identical palette states with different visible block-entity contents/context, cannot execute custom model loaders, and has no general client-output ingestion path. Existing ATM10 Lootr evidence uses a placeholder. The proposed client-capture/supplemental triangle/material route is an engineering alternative, not a demonstrated remedy. A matching-client or pinned internal-adapter prototype and specific owner-run visual proof must establish that path before P18.15; no nonexistent exporter command is presented as runnable. See the design's alternatives and acceptance's proof sequence. This does not waive custom-loader, code-rendered or saved-custom-dimension acceptance.

**Checks:** Documentation diff/whitespace inspection and the recorded non-test Verify command. No Rust or frontend changes, so Rust formatting/Clippy and frontend checks are not applicable. No tests, live rendering proof, Minecraft execution, source-world changes or release actions. Historical proof scripts/reports retain their original behavior and evidence.

### P18.14a — Prove matching-client supplemental mesh capture

**Status:** Owner-confirmed by Cameron on 2026-10-04 after the live proof and saved visual evidence; scoped to this pinned feasibility fixture.
**Files:** New isolated `tools/world-map-proof/client-capture/` client mod/build sources, fixture preparation/export validator/evidence tools and proof viewer; proof README; this plan. Existing production renderer, world files and the P18.14 findings commit remain untouched.
**What:** In one feasibility batch, inspect available private fixtures and choose one exactly pinned Minecraft/NeoForge client with a real custom-loader block and a contextual block pair whose identical states have different visible contents or surroundings. Build a client-side exporter using the actual baked-model/block-entity rendering entry points, emitting bounded supplemental triangle geometry, captured textures and explicit material properties. Prepare only a new isolated client/fixture, record source/version/content/snapshot/context hashes automatically, refuse unloaded chunks and mismatched context rather than producing substitute geometry, and label animation as a saved frame. Provide validation and an MSC-owned proof viewer for the exported result with automated receipts and owner visual evidence. Ask only for an essential unavailable input, naming the recommended fixture and its purpose. Do not launch Minecraft or run tests; compile/package and static checks are permitted. Record limitations honestly and keep P18.15 paused until Cameron runs the existing proof command and confirms the named appearances and refusal cases. No cross-platform/general-mod claim or release action.
**Verify:** `python3 tools/world-map-proof/client-capture/proof.py run --workspace /home/camerontemple/.cache/msc-map-client-proof`
**Batch:** Feasibility A — P18.14a only; one implementation/evidence-tooling batch and one commit, then stop for owner-run live proof. P18.15 stays paused.
**Commit:** `P18.14a: prototype matching-client mesh capture`

**Implementation evidence (2026-10-04):** Inspected the installed ATM10 inputs and selected Minecraft 1.21.1 / NeoForge 21.1.251 with exact-hashed Supplementaries 1.21.1-3.9.9 and Moonlight 1.21.1-3.6.8. Barnacles at 9,65,3 use the actual `supplementaries:random_rotation` loader; equal-state pedestals at 3,65,3 and 6,65,3 hold a diamond and emerald. The new client mod collects actual model/block-entity output into supplemental indexed triangles, GPU texture PNGs and explicit materials; the MSC proof viewer consumes these files. Preparation copies only the two pinned JARs into a marked private workspace, checks the prepared launch directory and produces source/binary/input receipts. Capture binds the flushed isolated snapshot, canonical state/block-entity context and resource selection; unloaded-chunk, wrong-snapshot and wrong-context requests must be refused before publishing an immutable capture. Animation is a saved frame. Validation bounds/checks artifacts and the viewer records explicit owner confirmations with Minecraft/viewer screenshots. See [prototype instructions and limits](../../tools/world-map-proof/client-capture/README.md). No input request was needed; no existing world or production file was changed.

**Checks:** Built/package-compiled the pinned Java client mod and prepared its launch configuration using `proof.py build` (`jar`, `prepareClientRun`, `writeProofLaunchReceipt` only). Python compilation, viewer JavaScript syntax and Git whitespace inspection passed. No tests or Minecraft launch; no live capture/export validation or visual pass is claimed. No Rust edits, so Rust formatting/Clippy are not applicable. Initial development asset preparation downloaded approximately 786 MiB into Gradle's cache; subsequent builds reuse it.

**Owner acceptance still required:** Run the exact Verify command above, create the new named Creative Superflat world, teleport to the fixture chunk, run `/mscproof setup` and `/mscproof capture`, then Load capture in the proof viewer. Compare barnacles shape/orientation/texture transparency and both pedestal/item appearances against Minecraft. Inspect the three refusal records and saved-frame label; save confirmation only if those results match. A clean build or valid export does not close this step. P18.15 stays paused pending Cameron's confirmed visual result; later mixed-pack/custom-dimension/platform/loader acceptance remains unchanged.

### P18.14b — Bind capture after integrated-server pause saves

**Status:** Owner-run correction included in Cameron's successful, visually confirmed proof on 2026-10-04.
**Files:** Isolated client capture source and README under `tools/world-map-proof/client-capture/`; this plan.
**What:** Fix Cameron's first live proof refusing `snapshot_mismatch`. The client log records integrated-server pause saves between fixture setup and capture, even with simulation frozen. On an explicit capture request, verify client/server fixture context against setup and verify unchanged resources/frozen simulation, flush the current save on the server thread, then bind its snapshot for the following render-frame capture. Preserve all unloaded-chunk, wrong-snapshot and wrong-context refusals and before/after mesh checks. No production changes or existing-world edits. Do not run Minecraft or tests.
**Verify:** `python3 tools/world-map-proof/client-capture/proof.py run --workspace /home/camerontemple/.cache/msc-map-client-proof`
**Batch:** P18.14b only — one corrective commit; stop for Cameron's live proof. P18.15 remains paused.
**Commit:** `P18.14b: bind mesh capture after flushing pause saves`
**Checks:** Java package build and prepared-launch directory check; Git whitespace inspection. No tests or Minecraft launch. The observed pause-save cause is supported by the private client log; the correction's live rendering result remains pending.
**Owner verification:** Close the running proof client, run Verify, reopen the isolated proof world and run setup then capture. Switching focus before capture must not invalidate an otherwise unchanged fixture. Compare the named exported appearances and inspect all three refusal records before confirming evidence. This correction does not close P18.14a's visual acceptance or waive later acceptance rows.

### P18.14c — Check actual cached chunks before capture

**Status:** Owner-run correction included in Cameron's successful, visually confirmed proof on 2026-10-04.
**Files:** Isolated client capture source and README under `tools/world-map-proof/client-capture/`; this plan.
**What:** Fix Cameron's live capture reporting `Required refusal did not happen: unloaded_chunk`. Inspection of pinned Minecraft 1.21.1 source establishes that `ClientLevel.hasChunk()` unconditionally returns true. Replace that unsuitable check with the actual chunk source lookup for `ChunkStatus.FULL`, loading disabled, requiring a non-null cached chunk. Apply the same guard to fixture setup, every context/halo position and requested capture coordinates; refuse missing chunks without accepting an empty fallback or generating terrain. Preserve all required runtime refusal checks and snapshot/context guards. No production changes, tests or Minecraft launch.
**Verify:** `python3 tools/world-map-proof/client-capture/proof.py run --workspace /home/camerontemple/.cache/msc-map-client-proof`
**Batch:** P18.14c only — one corrective commit; stop for Cameron's live proof. P18.15 remains paused.
**Commit:** `P18.14c: refuse chunks absent from the client cache`
**Checks:** Java package build/prepared-launch check and Git whitespace inspection. Inspected `ClientLevel`, `ClientChunkCache` and `ChunkSource` in the pinned local development source archive. No tests or Minecraft launch; live capture and visual acceptance remain pending.
**Owner verification:** Close the proof client, run Verify and reopen the isolated world. Run setup then capture. Successful export must include `unloaded_chunk`, `snapshot_mismatch` and `context_mismatch` refusals. Compare the named appearances in the viewer before confirming visual evidence; no broader acceptance is waived.

### P18.14d — Validate the pinned multipart custom-model wrapper

**Status:** Owner-confirmed by Cameron on 2026-10-04; the existing capture validates and visual evidence is saved.
**Files:** Isolated export validator and README under `tools/world-map-proof/client-capture/`; this plan.
**What:** Fix the viewer rejecting Cameron's successful capture because its outer baked-model class is Minecraft's `MultiPartBakedModel`. Inspect the real export, pinned barnacles blockstate and custom loader bytecode: the north-only state selects `supplementaries:block/barnacles`, whose resource uses `supplementaries:random_rotation`, within the vanilla multipart wrapper. Replace the incorrect package-name check with exact expected wrapper, north-only state and pinned loaded model-resource SHA-256 checks. Retain exact mod/game pins, loader identity, required nonempty custom geometry, contextual pair, materials/artifacts and all three refusal checks. Validate the owner's existing export; no recapture or Minecraft launch is required to inspect it. No production or world changes.
**Verify:** `python3 tools/world-map-proof/client-capture/proof.py view --workspace /home/camerontemple/.cache/msc-map-client-proof`
**Batch:** P18.14d only — one corrective commit; stop for Cameron's visual comparison. P18.15 remains paused.
**Commit:** `P18.14d: validate pinned multipart custom-model evidence`
**Checks:** Python syntax compilation, validation of Cameron's actual published capture and Git whitespace inspection. No tests or Minecraft launch. Existing live export records equal pedestal states with 84 diamond and 72 emerald block-entity vertices, exact custom model resource hash and all three required refusal codes. Export validation does not establish visual matching.
**Owner verification:** Use Verify to inspect the existing capture, click Load capture and compare its appearances with Minecraft/the saved frame. Confirm evidence only after the named geometry, textures/transparency, distinct items and saved-frame/refusal details match. No acceptance criterion or broader platform/mod claim is waived.

### P18.14e — Record Cameron's confirmed feasibility evidence

**Status:** Cameron explicitly reported “saved and confirmed” on 2026-10-04; confirmation recorded.
**Files:** This plan and proof READMEs only. Private captures, game assets and screenshots remain outside Git.
**What:** Read the saved owner-confirmation receipt and check its capture/screenshot hashes against the actual private artifacts. Record the successful pinned Minecraft 1.21.1 / NeoForge 21.1.251 / Supplementaries 1.21.1-3.9.9 / Moonlight 1.21.1-3.6.8 proof: real custom-loader barnacles, equal-state pedestals with distinct diamond/emerald contents, exported geometry/textures/materials displayed in the MSC proof viewer, all three required refusal codes and saved-frame identification. Clear P18.15's owner-proof prerequisite without starting its implementation or claiming production/general-mod/cross-platform acceptance.
**Verify:** `git show --check --stat --oneline --grep='P18.14e' HEAD`
**Batch:** P18.14e only — evidence-recording conversation and one documentation commit.
**Commit:** `P18.14e: record owner-confirmed mesh feasibility proof`
**Evidence:** Private capture `capture-d4360e02-e3c1-4d29-894e-77235b3e4c67`; capture SHA-256 `129aab95be5a83a6b3d6e0c396311fcd8574fc51fb354b94efb616453268e27f`; saved viewer screenshot SHA-256 `dc50d25aa32315ec8b45d1625f8b7e599bd7e088350ad117e2ccd261759d17ba`. Export contains 692 vertices / 346 triangles and saved frame tick 631, partial tick 0, captured at `2026-10-04T17:59:39.508362219Z`. Receipt has `visualAcceptance=owner_confirmed`, `confirmedBy=Cameron`, all three visual confirmations true, plus `unloaded_chunk`, `snapshot_mismatch` and `context_mismatch` refusal evidence. Verified receipt hashes against actual capture JSON and viewer PNG. This supersedes the preceding entries' historical pending-acceptance statements for this fixture only.
**Checks:** Read-only evidence/hash inspection and Git whitespace inspection. No tests, Minecraft launch, source-world changes, production implementation or release actions. Existing broader acceptance requirements remain unchanged.

### P18.15 — Add resource identity, safe inventory and useful diagnostics

**Status:** Implemented; awaiting Cameron’s verification. Proof prerequisite confirmed on 2026-10-04. P18.16–P18.17 are implemented pending owner verification.
**Files:** New `crates/msc-domain/src/map_assets.rs` and exports; new application/infrastructure map-asset modules and their exports; Java map bridge, compatibility helper, DTOs and API schema/generated types; `crates/msc-agent/src/cli/mod.rs` for read-only reporting; focused regression sources only when essential; design/acceptance documents and this plan.
**What:** Establish the shared foundation without changing which assets a working map renders:

- Model exact source evidence, game/loader/version, selected pack order, namespace contributions, original block/state identity, rendering capabilities and generation IDs. Do not identify a mod from a namespace or filename alone. Fingerprints include asset bytes, selected order/configuration, adapter/renderer/format version and relevant game version; snapshot/context keys are separate from reusable resource keys. Unknown source identity remains unknown.
- Inventory authorized server mods, retained import manifests/receipts, approved server resource packs and explicitly supplied client inputs read-only. Handle declared nested JARs with strict recursion/byte ceilings, enabled/disabled mods, malformed metadata, changing files and duplicate contributions. A consistent rescan discards/retries a changed input rather than trusting a half-read JAR. Never execute JARs or alter the client-only classifier.
- Establish the content store/generation manifest with immutable complete generations, atomic publication pointers, reader leases, cleanup/quota policy and separate candidate/current/previous state. Quotas and numeric limits must be written in the design before coding: define archive/decompressed/image/model-reference/nested-JAR/scan/concurrency limits, then record representative input sizes. If a real pack exceeds a limit, explain the specific limit instead of looping or quietly truncating it. Never delete bytes leased by a running renderer or the rollback generation.
- Add bounded diagnostic data from the actual chosen model resolver, not only a filename-existence audit. Classify missing models/textures, unsupported loader/type, absent block-entity/context data, missing saved chunks, intentional empty geometry and unresolved materials. Air/invisible technical blocks are not automatically errors. Include original IDs, owning source evidence, a few sample coordinates and counts with an explicit inspected area/snapshot.
- Serve read-only map-asset status/check/report through Worlds authorization and versioned capabilities. Reuse captured terrain and asynchronous operations for scans that exceed a short read; avoid one subprocess for every block in an entire world. Expose identical results in the CLI. No new frontend repair buttons until there is a working backend remedy.

**Essential regression candidates:** A small controlled fixture covering namespace collisions/cross-namespace parents, deliberate empty geometry versus genuinely missing geometry, and input/generation isolation. A store regression must prove cancelled/corrupt candidates leave the prior generation and active-reader bytes intact. Reuse focused archive/staging coverage; expected local runtimes below one second each, no network/sleeps; compile only unless specifically authorized.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc`
**Batch:** A — P18.14–P18.15.
**Commit:** `P18.15: inventory map resources and classify rendering failures`
**Acceptance evidence:** Opening existing Java/Bedrock maps retains their rendering path. A report correctly identifies the supplied broken-model example and the intentional empty model separately. Host/world switch does not reuse another binding. Report labels/counts describe the checked region, not the entire world. Batch A ends with an actionable report and recorded feasibility evidence, not a claim that modded rendering is already complete.

**Implementation evidence (2026-10-04):** Added typed resource/source/binding/report identities, bounded read-only archive inventory, namespace-preserving model inspection, saved Java chunk inspection, immutable diagnostic generations with atomic current/previous pointers, Worlds-authorized asynchronous status/check/report API and matching CLI. Explicitly reports unknown client selection and unsupported production renderer namespaces; no supplemental geometry is adopted. Existing Java/Bedrock rendering code is untouched. See [usage and limits](world-map-assets-usage.md). Active live-world inspection requires the server stopped; archived slots are inspected directly without extraction. Existing exact-version vanilla cache is read without downloads. Client import/acquisition and repair remain P18.16 work.

**Checks:** Rust formatting and agent Clippy passed; API generation/contract consistency passed; agent build passed and the exact `world map-assets check --help` command exists; focused regression sources compiled only. No tests, Minecraft, agent launch or release actions. Existing unrelated Rust warnings remain. Three essential regression sources protect namespace/empty/context distinctions, cancelled/corrupt generation isolation and negative-coordinate/missing saved chunks. Controlled temporary inputs, no network/timing assumptions; expected total runtime below one second, not measured because they were not run.

### P18.16 — Obtain exact client resources and compose the resource stack

**Status:** Implemented; awaiting Cameron's verification.
**Files:** Application/infrastructure map-asset acquisition and composition modules; existing modpack manifest/import modules and provider/download boundaries only where exact provenance must be retained; private map source receipts; resource-pack metadata readers; focused controlled regressions; design/acceptance documents and this plan. Server mod installation/classification behavior is preserved.
**What:** Make ordinary mod asset collection automatic when its inputs are available:

- Prefer already installed verified/hashed resources, then exact retained Modrinth/CurseForge file/release references, then an exact manifest/source import. Preserve client-only resources in the map cache while keeping them out of server mods. Retain client-overrides and required resource metadata during future pack imports without changing the source identity policy; existing servers without that evidence receive a concrete missing-source result, not a guessed newest pack.
- Resolve individual loose JARs by exact provider/hash evidence when possible. If the server and client artifacts differ, validate the publisher's matching game/loader/mod release relationship rather than assuming their filenames or hashes match. Record confidence/evidence and missing client counterparts. Reuse provider credentials/manual-download rules; never silently change installed mod versions or substitute a similar project/release.
- Download through bounded HTTPS/redirect/provider rules and validate published hashes/sizes. Reuse verified objects on retry, including after partial provider failure. If automation cannot obtain a file, return its exact required identity and an importable remedy. Authentication/provider restrictions and offline state must not discard the usable generation or become repeated automatic attempts on every tile request.
- Compose vanilla, mod resources, enabled resource packs and client overrides using a documented, evidenced priority order. Record winners and overridden sources. Include required parent models, texture variables, cross-namespace resources, applicable metadata/overlays/filters and animation descriptions; flag custom loaders/generated resources for their proper remedy. Do not flatten everything by namespace, alphabetize precedence or enable every optional client resource. If active client order is unknown and affects appearance, ask for that concrete input through the eventual importer.
- Resolve local/server resource-pack bytes from approved files; if a pack exists only at a configured URL, use the established approved-download boundary with a verified identity. Do not turn a map scan into an arbitrary host URL fetch. Compiled map resources never modify the server's resource-pack configuration.
- Produce a complete candidate with source manifest, diagnostic delta and prerequisites for rendering. No production map switches here. Third-party assets are downloaded/imported on the user's machines, not bundled in MSC releases or republished from a central service.

**Essential regression candidates:** One controlled end-to-end fake-provider case for exact client selection, client override/order, checksum refusal, manual-only/offline behavior and repeat reuse. Add only cases existing provider/staging tests do not cover. Expected local runtime below two seconds; no network or elapsed-time assertions.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc`
**Batch:** B — P18.16–P18.17.
**Commit:** `P18.16: acquire and compose exact modded map resources`

**Implementation:** Added exact manifest/provider acquisition, verified retry cache, bounded candidate resource storage and an explicit provenance stack. Future pack imports preserve exact client manifests/overrides separately from server mods. Approved server packs require matching local configured checksum; arbitrary configured URLs are never fetched. Unknown client selection, counterpart relationship or overlay format remains a concrete input requirement. Candidate rendering adoption belongs to P18.17.
**Checks:** Fixed the boxed missing-source constructor after the failed check. Required formatting and ordinary agent Clippy now pass, with three existing unrelated warnings. The focused infrastructure regression compiles; it was not run. No test suite, live rendering proof or release run. The new controlled acquisition regression protects checksum refusal, exact identity retention, offline/manual requirements and retry reuse without server-mod writes; expected under two seconds after compilation.

**Acceptance evidence:** The selected static-mod fixture prepares exact resources without SSH or a launcher on the agent host. A pack override visibly selects the intended texture in the candidate inspection. A blocked/missing exact client file yields its specific import requirement and retains reusable verified downloads. No client-only JAR is installed into server mods, and no existing generation is replaced by a partial candidate.

### P18.17 — Adopt prepared resources without disrupting working maps

**Status:** Implemented; awaiting Cameron’s verification. Owner visual/platform acceptance remains pending.
**Files:** Java terrain bridge and compatibility module, map-asset application coordinator, chosen namespace renderer/adapter and pinned build/staging inputs from P18.14, viewer artifact handling only if required by the chosen format, DTO/capability/operation data; packaging helper delivery as needed; design/acceptance documents and this plan.
**What:** Connect asset preparation to actual map rendering as one guarded operation:

- Add first-open orchestration: inventory, exact acquisition/composition, bounded candidate validation/render preparation and activation. Worlds without mod/custom-resource requirements retain the established vanilla path. Provider/transient failures preserve the visible scene; avoid failing every map artifact request because one optional custom source cannot be acquired.
- Implement the P18.14 namespace route, retaining original IDs in diagnostics. Adapt only private snapshot/palette/resources, never source saves. Handle block properties/variants, parent chains, supported geometry/materials, vanilla references, modern save layouts and a saved custom dimension. A generic unknown cube must not be counted as resolved geometry; avoid culling adjacent terrain based on an unknown model's invented opacity.
- Prepare outside the global renderer mutex with bounded workers, per-binding deduplication and cancellable operations. Expose truthful progress states and retryable versus input-required failures. Tile traffic must not restart preparation. Snapshot/asset generations are checked again before publication so host/world/dimension switches and server changes cannot publish stale work.
- Render enough of the requested region to validate the candidate before atomically adopting it. Invalidate only tiles whose resource/context fingerprints changed; include reverse dependencies so a parent texture change refreshes all affected models. A failed tile/model is attributable and does not blank the whole map. Keep supported baseline and difficult-model fallback distinct.
- Coordinate scene adoption with the viewer: keep camera, selected dimension/depth, follow state and player feed when valid; keep the old scene until the new scene is ready. A changed snapshot or resources cannot mix old geometry with new textures. Cancellation/closing/switching hosts retires unneeded jobs and releases caches/readers. A retained stale scene is labeled with its actual snapshot/resource age.
- Preserve first-use renderer recovery, complete headless helper updates and explicit renderer overrides. Any additional adapter/build input must be reproducibly staged for every shipped architecture; packaging helper byte checks remain build-only, not a new CI/smoke gate.

**Essential regression candidates:** Controlled coordinator cases for stale job publication/host isolation and candidate failure retaining the usable generation. One representative namespace/culling/affected-tile regression only if current coverage misses that observed risk. No broad timing matrix; use explicit fake completion/cancellation, expected under two seconds locally.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && npm --prefix clients/desktop-web run check`
**Batch:** B — P18.16–P18.17.
**Commit:** `P18.17: adopt modded resources with guarded map generations`

**Implementation:** Connected first-open and explicit preparation to exact resources, private namespace/save adaptation, cancellable per-binding tickets, saved tile/atlas validation and guarded scene/report adoption. Java artifacts carry pinned generations; the viewer retains its old scene, camera/depth/follow state and player polling while a replacement loads. Original-ID scoped diagnostics distinguish marked non-occluding fallbacks and missing saved chunks; snapshot/resource age and required sources remain explicit. Vanilla/Bedrock paths, helper recovery/overrides and build-only release packaging remain intact. See the P18.17 design/usage records for bounds and conservative atlas reuse.

**Checks:** Required non-test format/Clippy/viewer checks pass with only pre-existing warnings. Focused infrastructure regression source compiles; tests were not run. Live standard-model/custom-dimension rendering, cancellation/provider failure, texture-change reuse, Paper/Tectonic/Bedrock preservation and supported desktop platform acceptance remain Cameron’s verification. New controlled coordinator/adapter regressions protect stale visible-scene adoption and namespace/culling/dependency risks absent from existing store/inspection coverage; expected combined runtime under two seconds. No release actions or workflow changes.
**Required owner acceptance (not yet recorded):** A standard-model modded map now renders through the normal entry point with automatic assets. Paper/Tectonic and existing Bedrock rendering remain intact. Cancel/offline/provider failure preserves the scene, health/player polling continues, and retry reuses verified data. Changing a pack updates affected appearances without a server restart. Missing saved chunks remain distinguishable from asset failures.

### P18.18 — Import matching client resources locally and across hosts

**Status:** Implemented; awaiting Cameron’s verification. Live import/platform acceptance remains pending.
**Files:** New native client-asset inspection/bundle commands in `clients/desktop-web/src-tauri/src/`, capability registrations and native transport; shared bounded upload/staging DTOs and routes; map-asset import/application modules; CLI map-asset import command; existing archive/download primitives where reused; design/acceptance documents and this plan.
**What:** Provide the user-supplied input path for everything the server cannot resolve:

- Let users explicitly select a Minecraft client instance, modpack archive, asset bundle or required individual file. Support the official instance layout and a proven launcher layout (Prism included); allow selecting a custom instance directly instead of assuming every launcher root can be discovered. Show detected game/loader/mod/pack versions and conflicts before applying. Select only resource-related files and the minimum active-pack/order/config metadata; do not copy accounts, auth/session tokens, logs, screenshots, unrelated saves or arbitrary configuration.
- Inspect and extract JAR/pack bytes without executing them. Include referenced nested resources, supported format metadata and complete cross-namespace dependencies. Preserve enabled pack order and optional selections when actually known. Hash the bundle and source objects; explicitly distinguish a local imported identity from provider-verified provenance. Validate mismatch/conflict and give an actionable correction rather than importing a wrong pack optimistically.
- Extend existing staging with a map-only purpose tied to host/credential, server/slot, expected bytes, operation and source generation. Check Worlds permission and ownership at begin, chunk upload, complete, inspect and apply; do not rely on UI host selection for isolation. Preserve resumable transfer, cancellation/expiry cleanup, bounds and server-switch checks. No client filesystem path is supplied to a remote agent.
- Handle Windows drive/UNC paths, separators, reserved names/case collisions and Unicode alongside Linux/macOS symlink and permission handling. Asset identifiers have Minecraft's case semantics even on a case-insensitive filesystem. Conflicting inputs require a deterministic represented result; do not let Windows choose a different texture accidentally.
- Transfer only the validated selected resource bundle from Fedora/Windows/macOS desktop to the remote agent, using the existing authenticated native API channel. Reinspect the bundle at the host before publishing a candidate. CLI users on a headless host can import the same portable bundle exported elsewhere, without a local game installation or GPU.
- After import, invoke the same preparation/validation/adoption path as automatic acquisition. Return affected-block diagnostics and actual rendering readiness, not only upload success. Add an import-source receipt so subsequent repair can reuse the imported bytes without silently rereading an arbitrary local path or uploading data to the wrong host.

**Essential regression candidates:** Reuse staged upload and archive safety coverage; add one map-purpose authorization/binding case and a compact malicious/case-colliding asset import case if absent. Cancellation and retry use controlled chunk input, not timed network activity. Expected local runtime under two seconds each.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib && npm --prefix clients/desktop-web run check`
**Batch:** C — P18.18–P18.19.
**Commit:** `P18.18: import matching client map assets across hosts`
**Acceptance evidence:** Import the matching instance from Fedora into the Ubuntu agent and see previously missing standard-model blocks render. Repeat an import on native Windows and macOS paths; final full platform coverage is P18.21. Wrong release is rejected with a correction path. Cancelled uploads leave no published candidate. Imported resources never alter the source client or server mod tree.

**Implementation:** Added native resource-only instance/archive/file inspection and portable bundle export; official unambiguous game directories and explicitly selected Prism instances are supported. The agent reinspects checksummed objects, game/loader/mod compatibility and resource order. Map-purpose uploads bind the initiating credential and exact host/server/slot revision at admission, every chunk and apply; owner cancellation remains possible after switching servers. Imports share the cancellable renderer operation, reuse MSC-owned receipts and preserve the displayed scene on failure. Loose files carry explicit matching-map context and local-hash evidence, never fabricated publisher verification. Unknown mod versions require identical original JAR hashes. Existing server/client files are read only.

**Checks:** Fixed both reported compiler errors. Required Rust format, agent Clippy, native-shell Clippy and Svelte checks passed with existing unrelated warnings. Generated API consistency and focused bundle regression source were checked without running tests. Two essential controlled regressions protect resource-only privacy/selection and malicious/case-colliding/checksum inputs absent from ordinary archive coverage; expected combined runtime below one second, unmeasured. Native lockfile was refreshed for infrastructure dependencies already introduced in Batch B. No tests, game launches, releases, tags or workflows.

### P18.19 — Deliver repairs that verify the affected blocks

**Status:** Implemented; awaiting Cameron's verification.
**Files:** Map-asset repair coordinator, operation/status/report DTOs and routes, CLI map-asset commands, `WorldMapViewer.svelte` and a focused rendering repair sheet/native action bindings, relevant embedded help content, design/acceptance documents and this plan. Read `antiAIslop.md` before frontend work.
**What:** Turn the diagnosis/import backend into a complete remedy with matching desktop and CLI behavior:

- Add quiet map status and a **Check rendering** action that reports the inspected area, affected mods/blocks and evidence. A bounded repair sheet puts the recommended working action first and lets the user inspect details. Support **Repair map assets**, **Import client assets**, **Select resource packs**, **Rebuild affected terrain**, **Restore previous map resources** and **Export report** only when their backend exists and the action can change the diagnosed state. Do not present a generic Retry for unsupported code or missing user input.
- Bind each issue to its specific remedy: missing exact bytes -> acquisition/import; wrong game/modpack release -> matching source selection; wrong ordering -> pack selection/order; corrupt candidate -> targeted reacquisition; stale tiles -> dependent-tile rebuild; missing saved terrain -> explain generation/save/refresh; unsupported model/context -> exporter/adapter capability from P18.20. Until that capability is available, explain the precise limitation without a fake active Export button. Reports are an escalation aid, not the success outcome.
- Make repairs transactional and resumable with explicit source/target generation, progress/cancellation, one operation per compatible binding, changed-input retries and logs without secrets/host paths. Avoid reinstalling MSC, changing mod enablement, resetting worlds or deleting all caches. Restore chooses only a compatible previous generation; otherwise explain the mismatch and retain the current scene.
- Validate repaired resources and re-render/re-check the exact affected area. An operation becomes **repaired** only when those classified failures are resolved and candidate artifacts are ready/adopted. Distinguish `needs_input`, `partially_repaired`, `unsupported`, `cancelled` and `failed`; a successful download cannot masquerade as a completed repair. Correct visual appearance is still recorded by owner acceptance rather than inferred from a resolver counter.
- Expose equivalent CLI inspection, source import, selection, repair, rollback and redacted report export using the P18.14 grammar. Structured JSON carries real state, precise required input and operation/result IDs with meaningful exit status. No GUI-only dependence for a headless agent; it can consume a client bundle exported on another platform.
- Keep map exit/camera/dimension/depth/player behavior and the normal automatic path simple. Remember whether a user dismissed an unchanged issue; avoid warning floods while still revealing new severe failures. After a mod update, give the concrete changed-version/source issue and a route to reimport/export, instead of repeated invisible failures.

**Essential regression candidates:** A controlled diagnosis -> acquisition/import -> generation adoption -> affected-area recheck case must refuse `repaired` while the original missing/unsupported problem remains. Use existing UI coverage or owner manual acceptance for UI wiring; do not add snapshots/exact-prose/timer tests. Expected controlled runtime below two seconds.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build`
**Batch:** C — P18.18–P18.19.
**Commit:** `P18.19: repair map assets and verify affected terrain`
**Acceptance evidence:** Deliberately remove a disposable fixture's map resource, import a mismatched source, reverse two conflicting packs and invalidate affected tiles. For each case, desktop and CLI recommend a relevant action, it restores the named block's expected appearance, and unchanged good blocks remain good. Offline/cancelled/unsatisfied actions do not claim success. A diagnostic report alone does not close any repair acceptance row.

**Implementation:** Added the bounded Java map rendering sheet and matching headless repair/rebuild/selection/restore/report-export commands. Checks reuse adopted resources and a consistent saved snapshot; mutations re-render the last inspected area, compare classified before/after failures and validate/adopt coherent terrain artifacts. Only resolved failures in the same saved scope/binding can yield `repaired`; partial, unsupported, input-required and failed/cancelled outcomes remain explicit. Source selection keeps a compatible previous receipt, and revision guards reject stale requests. Desktop transfer captures the host, validates the native checksum and cancels on close/switch. Unsupported runtime exporter/context cases remain P18.20 limitations. No tests or release actions ran. Essential success-boundary regression source was added and compiled only; expected controlled runtime below two seconds.

**Agent checks:** The full permitted Verify command passed, with only existing agent/Svelte/build warnings. Native-library Clippy, native formatting, generated API consistency and `cargo check -p msc-application --test map_assets_repair` also passed. The last command compiles regression source without running tests.

**Owner acceptance pending:** Matching source appearance, wrong-release rejection, conflicting pack order, invalid affected terrain, rollback, offline/cancel/host-switch behavior, camera/player continuity, baseline maps and Windows/macOS/remote checks remain the owner's live verification. See `world-map-assets-acceptance.md`; a report alone closes no rendering gate.

### P18.20 — Complete recovery for custom loaders and code-rendered blocks

**Status:** Forge build correction implemented; awaiting Cameron’s verification. Broader P18.20 production integration remains incomplete. Cameron explicitly authorized compilation retries and proceeding to P18.21 evidence preparation on 2026-10-05; no rendering pass is claimed.
**Files:** Versioned private exporter/helper sources under `tools/java-map-export/` (new), a cross-platform build-only helper driver there, and loader-specific modules/build manifests selected in P18.14; internal renderer adapters only where evidenced; native exporter controls, map export import/application coordinator and validated format/DTO/capability updates; existing viewer decoder only if a compatible extension is needed; helper staging/packaging/license notices as required; design/acceptance documents and this plan.
**What:** Implement the difficult-model route as part of this feature, not an indefinitely optional future promise:

- Prefer specific internal adapters for common, well-defined custom model formats when feasible; use the proven matching-client exporter for loader-generated/baked geometry or block-entity rendering that static assets cannot reproduce. Support Fabric, Forge and NeoForge loader/version families actually exercised by the required fixtures. Record the precise supported ranges and negotiate the helper/format version; do not reuse a helper across incompatible loader APIs merely because the mod name matches.
- The exporter must use the same enabled mods, resource order and relevant configuration as the client that displays the block correctly. Prefer an explicitly selected running client or dedicated private working instance. Do not install helper JARs into a user's ordinary instance, launch mod code, change game files or request credentials as a side effect of passive map opening. The deliberate **Use Minecraft rendering** repair action explains the work and obtains any required user action; authentication remains inside Minecraft and is never transferred to MSC.
- Export reusable baked models/materials where sufficient and context-bound appearance for objects that require position, neighbors, block-entity contents or dynamic resources. Bound any transferred world/context to the requested area and required neighbor margin using authorized saved snapshots. Do not claim coverage of unvisited client chunks; support a private read-only-source snapshot route from P18.14 or name the required in-game area/capture action. The real server world is never opened by a helper with write access.
- Emit portable checked geometry/UV/material/texture objects, identity/fingerprints, context/snapshot times and capability counts into the versioned map bundle. Headless agents only validate/consume the bundle; they do not run a graphical game or acquire desktop libraries. Cache reusable output separately from context-dependent appearances and invalidate it when mod versions/order/configuration/context change.
- Import through P18.18's authenticated staging, validate all format/size/reference bounds, bind it to the right host/world/input generation and adopt it through P18.17. Wire the exporter remedy into the repair sheet and CLI bundle importer. Build/distribute required loader helpers reproducibly for every supported desktop OS/architecture, including native Windows path/process/cancellation handling. Preserve baseline Vantage/Bedrock payloads and build-only release policy.
- Verify the required custom-loader and code-rendered fixture against Minecraft at named positions/states: shapes, textures, contents/connections and captured appearance. Label capture time and any frozen animation clearly; no universal animation/live-fidelity claim. A placeholder, absent object or a capture that drops required neighboring state does not pass. If the proof cannot meet the defined fixture promise, record the precise blocker and revise the route before final completion.

**Verification scope:** The build-only helper driver selects compilation/JAR assembly tasks explicitly (not a Gradle build task that brings tests with it), records loader/helper output identity, and fails if a required target cannot build. Compile helper sources with their documented build tasks without invoking test tasks; native integration type/build checks remain non-test. List exact owner-run exporter/visual commands in the acceptance document at implementation time. No automatic launches/rendering smoke suites under this Verify command. Do not expand the project into an unrestricted third-party plugin API or a second full Minecraft installation manager.
**Verify:** `python3 tools/java-map-export/build.py --all-supported --build-only && cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib && npm --prefix clients/desktop-web run check`
**Batch:** D — P18.20–P18.21; an unresolved required difficult-model remedy keeps D and the feature gate open.
**Commit:** `P18.20: prepare pinned forge capture and correct access rules`
**Acceptance evidence:** A required object that fails the static path renders correctly after the offered remedy, through the MSC map, on each supported client platform; exported output also works on a headless Ubuntu/Windows/macOS agent. The helper's captures and limitations are observable, cancellation leaves the scene intact, and no game/agent credentials or source world changes result.

**Partial execution (2026-10-05):** Cameron authorized selecting a pinned public Forge fixture. Installed Prism metadata identifies `campack`/`campackBIG` as Minecraft 1.20.1 / Fabric 0.19.3; the confirmed NeoForge proof is Minecraft 1.21.1 / NeoForge 21.1.251. Selected public Minecraft 1.20.1 / Forge 47.4.10 with Supplementaries `1.20-3.1.43` (Modrinth release `S0TIJ1hU`) and Moonlight `1.20-2.16.35` (`W0ZWjZib`). Verified downloaded mod bytes against published SHA-512 hashes and recorded SHA-256/size, provider release/file IDs, exact game/loader bytes and tool pins. The older mod does not contain barnacles; selected its actual `supplementaries:goblet` loader plus intended diamond/emerald pedestal context pair. Added isolated draft Forge exporter, validator and viewer sources under `tools/java-map-export/forge-1.20.1/`; original proof artifacts and production paths remain untouched.

**Historical failed check / stop:** Explicit Java 17 / Gradle 8.8 `fixture.py build` requested only `jar` and `reobfJar`. `compileJava` failed with 13 protected/private member-access errors: render-type name, composite state, texture binding and light pixels. The copied readable-name access rules do not meet Forge 1.20.1's SRG-name requirement, and the name field rule is missing. Read-only inspection of the generated mapping identified the precise member names recorded in the [draft helper README](../../tools/java-map-export/forge-1.20.1/README.md). No fix/rebuild followed that failed source check. An earlier Java 25 setup attempt failed before compilation; explicitly choosing Java 17 resolved only that toolchain issue. The consolidated Verify above has not passed and its production build driver is not yet implemented.

**Remaining at the failed check (correction below supersedes the access-rule failure):** Correct Forge access rules and portable Java selection, then compile and establish its runtime capture behavior. Complete the production loader adapters, snapshot/context correspondence, portable capture validation/import/adoption, native/CLI remedy, helper distribution and owner appearance checks. This commit records partial work and the failure, not an implemented step. P18.21 remains unstarted under the batch stop rule. No tests, Minecraft launches, source-world changes or release actions.

**Authorized correction (2026-10-05):** Replaced the Forge material/lighting access rules with the exact generated SRG names and added `RenderStateShard.name`. Explicit Java 17 / Gradle 8.8 `fixture.py build` now passes compilation, `jar` and `reobfJar`, checks pinned game/loader bytes, and writes the private helper build receipt. Three deprecated `ResourceLocation(String)` warnings remain. No Rust/frontend source changed, so their format/Clippy/type checks are not relevant to this narrow correction. The original failed attempt above remains historical evidence. The consolidated production Verify has not passed because its production driver/integration are not implemented. Owner verification of the Forge capture is pending.

**Correction Verify:** `JAVA_HOME=/usr/lib/jvm/temurin-17-jdk python3 tools/java-map-export/forge-1.20.1/fixture.py build --gradle /home/camerontemple/.cache/msc-map-forge-capture-inputs/gradle-8.8/bin/gradle --workspace /home/camerontemple/.cache/msc-map-forge-capture`

**Owner clarification:** “If verification fails, STOP” means do not advance while the current step's verification is failing; routine errors can be corrected and permitted checks rerun without further approval. Cameron specifically directed proceeding to P18.21 after this Forge correction compiles. This authorizes evidence preparation while keeping the missing production remedy and all platform/visual gates open. No tests, Minecraft launch or release actions.

### P18.21 — Record all-platform normal rendering and repair success

**Status:** Evidence collector and acceptance preparation implemented; awaiting Cameron’s verification. Actual all-platform rendering/repair acceptance remains pending.
**Files:** `docs/msc2/world-map-assets-acceptance.md`, `docs/msc2/world-map-assets-design.md`, `tools/world-map-assets/collect-acceptance.py`, its `README.md`, existing map/client/headless help/support documentation and this plan. Targeted fixes to earlier step files are allowed only when an observed gate failure requires them and are recorded in this step's commit; no unrelated cleanup or release workflow changes.
**What:** Consolidate real evidence and close the feature only when the end behavior holds:

- Add the read-only acceptance collector described above: ingest actual agent/desktop report JSON and package/build identities, preserve per-host/architecture/input provenance, validate missing/mismatched evidence, and generate the matrix with pending visual confirmations. It must not install packages, launch Minecraft, change fixtures or execute tests. Record its exact owner invocation beside the matrix.
- Run the documented owner-controlled matrix using the same pinned fixture/bundle identities. For desktop-to-agent OS coverage record all nine macOS/Windows/Linux pairs; include both native macOS architectures, native Windows service/headless operation, Fedora desktop -> Ubuntu headless and local Fedora/Ubuntu Linux installations. A portable bundle produced on each desktop family must render on each agent family. Sharing a Java exporter JAR is not evidence that its launch/graphics/path handling works on every OS.
- Exercise all six Java flavors across the matrix without needlessly repeating every fixture in every cell: full representative content/repair coverage on each OS, plus transport/import/binding coverage for every OS pair and both macOS architectures. Record minimum/current game-version families required by P18.14, saved standard and custom dimensions, first-use online preparation, cache reuse offline, loose-mod and provider-manifest sources, manually supplied client counterparts, resource order and version updates.
- For every required repair row start with the actual broken named block/area, execute the UI/CLI's offered action and record the corrected appearance at the same coordinates alongside the recheck result. Include missing/corrupt bytes, wrong release, pack-order conflict, stale tile/context, custom loader and code-driven object. Account for a cancellation/restart/retry and an offline-to-online recovery. A warning cleared, an import completed or an `unsupported` report is not a successful repair.
- Repeat the working-map controls: Vanilla, Paper/Tectonic, Purpur and Bedrock, including water/glass/foliage/stairs, Nether cutaway, End, camera/Fly/depth, players/follow/dimension transitions, refresh and clean exit. Compare before/after evidence with the same snapshot/resource fingerprints where possible. Switching host/world during preparation must not show another binding's assets or scene.
- Record preparation/render resource cost with representative packs: first-visible/preparation time, peak agent and renderer/helper RSS, disk/cache usage, downloaded/reused bytes, scan scope and game/agent responsiveness. Apply the design's budgets; the existing 512 MiB proof guardrail is not a published whole-pack performance target. Retrying/repairing must not start endless downloads/exports or require interrupting the game server to manage assets.
- Inspect helper staging/install/update/rollback assumptions on all shipped artifacts without launching a release as a test. Necessary package build evidence may be obtained with existing build commands; owner artifact acceptance stays distinct. Preserve complete helpers/checksums/signed metadata and the unchanged build-only release workflow. Publication is a later explicit task.
- Summarize exact supported behaviors and remaining limits from evidence. Keep failures/pending owner checks visible. Prepare independent gate review for the other agent after owner verification; an implementer cannot provide that phase review. Do not mark steps Done or close the original Phase 18/Phase 16 deferred gates on Cameron's behalf.

**Verify:** `git show --check --stat --oneline --grep='P18.21' HEAD`
**Batch:** D — P18.20–P18.21. Batch work may prepare evidence/checklists, but owner-run acceptance remains pending until recorded; the final gate cannot be inferred from successful builds.
**Commit:** `P18.21: record cross-platform modded map and repair acceptance`
**Acceptance evidence:** The following final gate has actual observations, artifact/source identity and no pending required row. If any row fails, record the concrete correction and repeat only the affected checks; do not restart a broad release or pretend troubleshooting succeeded by exporting a report.

**Implementation (2026-10-05):** Added the read-only standard-library collector and private evidence format. It validates actual report/binding/input/package/capture identities, retains owner visual confirmation separately, and reports 18 architecture-specific transport cells plus 222 fixture/repair/control/scenario cells. Missing observations stay pending; build receipts fill no rendering cells. Resource measurements remain pending unless actually supplied, and require budget review even when complete. The original isolated NeoForge owner confirmation remains feasibility evidence only. No production/platform acceptance was inferred.

**Checks:** Python compilation and collection from the actual corrected Forge build receipt/JAR passed: one build, zero observations, no rejected inputs, all 18 transport and 222 coverage cells pending. Exact owner invocation and session instructions are in `tools/world-map-assets/README.md`. No tests were added or run; no Minecraft or release was launched. Read-only staging inspection found the existing agent/Bedrock helpers and signed publication metadata preserved; the draft Forge exporter is not staged in shipped packages. Missing P18.20 production integration, actual platform observations, visual repairs, performance budgets and independent gate review remain open.

## Final feature gate — owner-visible success

| Required outcome | Passing evidence | Does not count as a pass |
|---|---|---|
| Ordinary mod assets just work | First map open acquires/composes exact available resources and named modded blocks match Minecraft on every supported OS/architecture. | A loader running only vanilla blocks; downloaded assets without rendered geometry. |
| Exact client inputs can remedy missing assets | Import from a matching client/pack on another machine fixes the same named blocks on the remote/headless agent. | SSH cache edits, full app reinstall, upload completion alone. |
| Resource conflicts are repairable | Correcting version/order rebuilds the affected map and restores expected textures/models without damaging good blocks. | Choosing newest files; suppressing the warning. |
| Difficult rendering has a real success path | Required custom-loader/context/code-rendered examples match the defined saved appearance after the offered adapter/export remedy on every supported platform. | Checker cubes, blank geometry, `unsupported`, a report-only escape route, an untested optional exporter. |
| Working maps remain working | Baseline Java/Paper-Tectonic/Purpur/Bedrock scenes and navigation/player/refresh controls retain their known behavior through preparation, repair failure and rollback. | Build success, silently switching to incompatible old resources. |
| Headless/remote operation works | All nine desktop/agent OS pairings and both macOS architectures have recorded bundle/import/binding outcomes; ordinary collection needs no desktop/game/GPU on the host. | One local macOS proof or one Fedora -> Ubuntu transfer used as a universal platform claim. |
| Repairs are truthful and durable | UI and CLI recheck the affected area, distinguish partial/unresolved outcomes, persist correct bindings and survive retry/update without repeating work indefinitely. | `repaired` after a download while the original blocks still fail. |
| Data and operating cost stay controlled | Source worlds/mods/launchers remain intact; inspected bounds/cancellation/isolation hold; measured preparation/cache costs meet the recorded design budgets. | Unlimited scans/uploads/caches or mod/server changes as an asset workaround. |

**Closure:** Cameron records verification; the other agent reviews this gate. Remaining required rendering/repair/platform failures leave the feature open, regardless of how many implementation steps are committed. No claim of universal compatibility with arbitrary future mod code follows from this finite matrix.

---

### P16.42 — Publish v0.1.23 with Java map dependency recovery

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.23.md`, this plan.
**What:** Increment the latest remote/published tag v0.1.22 to v0.1.23 and synchronize owned release identities without changing registry dependency versions. Include P18.12's complete headless helper updates, missing Vantage recovery, verified version-matched Java assets and diagnostic errors. Push main and the new immutable tag through the unchanged four-platform build-only workflow, preserving nine artifacts, checksums and signed update metadata. Cameron explicitly requested implementation, commit pushes and the next release tag.
**Verify:** `gh release view v0.1.23 --json tagName,isPrerelease,assets,url`
**Batch:** P16.42 only.
**Commit:** `P16.42: prepare v0.1.23 release`

**Preparation:** Inspected the complete active release workflow and successful v0.1.22 publication. No release gates changed or tests run. Implementation formatting, Clippy and frontend checks recorded under P18.12; synchronized source identity and locked metadata checked before tagging. README retains the actually published v0.1.22 while introducing the new candidate. Recent successful release runs took 38–47 minutes. Ubuntu map acceptance remains Cameron's check.

### P18.12 — Prepare Java map dependencies on headless hosts

**Status:** Implemented; awaiting Cameron's verification.
**Files:** Agent Java terrain bridge and new dependency module; headless CLI updater; agent manifest/lockfile; shared Vantage release pin and stager; map loading copy; headless installation guide; this plan.
**What:** Correct the headless updater's agent-only replacement list: validate and install Vantage, the Bedrock map helper and Vantage license together with the agent, with rollback for every replaced/new component. Validate the complete extracted payload before stopping the agent. Recover Vantage missing from older installations into the agent's own cache using the same platform-specific SHA-256 pin as release packaging; retain an explicit MSC2_VANTAGE_BIN override. Determine Minecraft's version from the saved world's level.dat, falling back to the configured Minecraft version. Download official version metadata/client bytes, verify their published SHA-1 digests and sizes, and safely extract only Minecraft assets/game data into a versioned cache. Reuse complete caches offline; reject incomplete caches and unsafe or oversized archives. Pass the matching asset directory explicitly to Vantage. Distinguish missing overrides, renderer download, client asset preparation, terrain preparation, startup exits and startup timeout; retain bounded stderr diagnostics in agent logs and redact the private renderer token. Explain terrain/texture preparation in the existing loading text. Do not modify game saves, install a launcher or restart Minecraft during dependency setup.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && npm --prefix clients/desktop-web run check`
**Batch:** P18.12 only.
**Commit:** `P18.12: prepare java map dependencies on headless hosts`

**Checks:** Formatting, ordinary agent Clippy, Python stager syntax/shared-pin inspection, Svelte check and diff whitespace passed. Existing warnings: unused infrastructure request, unnecessary mut in Bedrock runtime, unused auth helper, eleven Svelte warnings. New unit regressions compiled during cargo check; compiling all agent test targets is blocked by existing cli_service.rs references to four removed CommonArgs fields. No tests, map smoke suites or release workflows run for implementation verification.
**Essential regressions:** Three controlled unit regressions protect helper installation/complete rollback/refusal before mutation, version-matched asset selection/cache reuse/incomplete-cache repair, and checksum/path-traversal rejection without publishing partial assets. Existing coverage missed the helper omission and this new first-use asset path. Tiny local ZIPs, a fake HTTP transport and unique temporary directories with cleanup; no live network, sleeps or environment assumptions. Expected runtime under one second each after compilation; not run.
**Manual acceptance:** Update the Ubuntu agent, then reopen the saved Paper/Tectonic Overworld from Fedora when convenient after Chunky finishes. With the older updater's missing Vantage, confirm first use recovers the renderer and prepares Minecraft textures under MSC2_DATA_DIR/map-dependencies. Reopen and confirm cache reuse without another download; saved terrain should render without joining Minecraft. Inspect agent logs for preparation/failure details. A future headless update should install both helpers beside msc; fresh archive installers already do so. Offline first use reports the relevant dependency failure and can be retried after connectivity returns. Vanilla assets do not add third-party mod resource packs.

### P16.41 — Prepare README publication wording for v0.1.22

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `README.md`, this plan.
**What:** At Cameron's explicit request, identify v0.1.22 as the latest published build and update download/install examples accordingly. Remove candidate/building wording and retain the fixes and prerelease acceptance limitations. This wording is prepared ahead of publication: GitHub returned release not found at editing time; Cameron was informed. Do not change the active release tag or restart the build.
**Verify:** `git show --check --oneline HEAD`
**Batch:** P16.41 only.
**Commit:** `P16.41: update readme release references to v0.1.22`

**Checks:** README diff and whitespace inspected. Documentation only; Rust checks are not applicable. No tests run.

### P16.40 — Update README for the corrected v0.1.22 candidate

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `README.md`, this plan.
**What:** Describe the modern Java datapack fix and terrain-generation limitation, link the corrected release build, retain v0.1.21 as the newest published release, and update installation/update examples from stale v0.1.18 to v0.1.21. Preserve the active release tag and workflow; push documentation to main only.
**Verify:** `git show --check --stat --oneline --grep='P16.40' HEAD`
**Batch:** P16.40 only.
**Commit:** `P16.40: document corrected v0.1.22 candidate`

**Checks:** Inspected README diff and whitespace; checked replacement workflow is in progress. Documentation only; Rust formatting/Clippy are not applicable. No tests run and no release rebuild started.

### P16.39 — Replace the cancelled v0.1.22 candidate

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** `clients/desktop-web/src-tauri/Cargo.lock`, `docs/msc2/release/v0.1.22.md`, this plan.
**What:** Restore serde_repr 0.1.21 with its original checksum; the P16.38 version bump accidentally changed this unrelated registry package to nonexistent 0.1.22. Include P12.249 modern datapack support and retain the headless Linux fix. Cameron explicitly requested cancellation/removal and confirmed replacement of the unpublished v0.1.22 tag. Remove cancelled run 37177543797 and its old tag, then publish the corrected source using the unchanged build-only release workflow. No published v0.1.22 release or uploaded workflow artifacts existed when inspected.
**Verify:** `gh release view v0.1.22 --json tagName,isPrerelease,assets,url`
**Batch:** P16.39 only.
**Commit:** `P16.39: repair and replace v0.1.22 candidate`

**Checks:** Inspected all three completed failed platform logs: identical unavailable serde_repr dependency, before packaging. Fourth build cancelled. Restored the original registry version/checksum from the prior tag. Locked desktop metadata, synchronized application versions, formatting and diff whitespace checks passed. Application Clippy and addon regression compilation recorded in P12.249. No tests run or workflow gates changed. Nine artifacts, SHA256SUMS and signed update metadata remain required. Expected build time about 40 minutes based on recent successful runs; physical acceptance remains Cameron's verification. Tag replacement is a specifically owner-authorized exception, not a routine retry.

### P12.249 — Accept modern Java datapack metadata

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, this plan.
**What:** Accept modern min_format/max_format bounds as integers or one/two-element major/minor arrays, including inclusive upper major versions. Retain legacy positive integer pack_format support and require a non-null description. Reject malformed, missing or reversed modern bounds before changing the world archive. Preserve provider Minecraft-version checks, archive safety, overlays, backups and original pack bytes. Confirmed official Tectonic 3.0.29 datapack targets 26.3 and declares min_format/max_format 121 without pack_format.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-application --lib && cargo check -p msc-application --test addons`
**Batch:** P12.249 only.
**Commit:** `P12.249: accept modern datapack format metadata`

**Checks:** Formatting and ordinary application Clippy passed; focused addon regression source compiled. Existing infrastructure unused-request warning remains. No tests run. Extended the existing controlled archive regression to cover modern integer/array bounds, preserved metadata bytes, and rejection without world modification. Essential because valid modern packs were blocked and malformed metadata must not modify saved worlds; no network, runtime or timing assumptions; expected additional runtime under one second.
**Manual acceptance:** With the updated agent, add Tectonic 3.0.29 to a new Paper 26.3 world during creation and to a stopped world's datapacks afterward. Confirm metadata rejection is gone and Paper lists/enables the pack. Already generated terrain is not regenerated by installing it afterward.

### P16.38 — Publish v0.1.22 with the headless Linux status fix

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.22.md`, this plan.
**What:** Increment the latest remote tag v0.1.21 to v0.1.22 and synchronize release identity. Include P12.248's headless Linux status correction. Integrate the two existing remote commits before publication and retain their changes. Push main and the new immutable tag through the existing four-platform build-only workflow, preserving nine artifacts, checksums and signed update metadata. Cameron explicitly requested the push and next tag.
**Verify:** `gh release view v0.1.22 --json tagName,isPrerelease,assets,url`
**Batch:** P16.38 only.
**Commit:** `P16.38: prepare v0.1.22 release`

**Preparation:** Inspected the complete release workflow and successful v0.1.21 run. No workflow changes or tests run. Fix formatting, Clippy and regression compilation passed before the version bump; release identity and locked metadata checked during preparation. Ubuntu live acceptance remains Cameron's verification.

### P12.248 — Read headless Linux service status without private metadata

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-platform-linux/src/service.rs`, `crates/msc-platform-linux/tests/systemd_unit.rs`, this plan.
**What:** Recognize the shipped Linux headless archive service template when the installed agent unit has no private metadata. Reconstruct its installing account, custom data directory, binary and loopback arguments for status and routine service control. Keep metadata-based desktop units supported. Require the complete shipped template to match before reporting its fixed settings; reject unknown definitions instead of guessing. Read the existing service without rewriting it, so updating the binary repairs existing archive installations. The shared report's required log path uses the conventional data-directory path; the archive service continues logging to journald and status creates no log file.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && cargo check -p msc-platform-linux --test systemd_unit`
**Batch:** P12.248 only.
**Commit:** `P12.248: read headless linux service status`

**Evidence:** Cameron's Ubuntu `msc status` failed with missing metadata ServiceName. The headless installer renders a standard systemd template without the private comments required by the previous status parser; the CLI queried that parser before reading the live service state.
**Checks:** Rust formatting, shipping-agent Clippy and focused regression compilation passed. Existing warnings remain in infrastructure uninstall and agent runtime/auth code. No tests ran; no installed binaries or live services changed.
**Essential coverage:** One controlled regression renders the real package template with a custom data path containing spaces and a distinct primary group, checks running status, configured data-directory recovery, start/stop and preservation of the service file, and refuses an altered bind address rather than reporting a false default. Uses a temporary directory with cleanup and fake systemctl, with no network, real service changes or timing assumptions; expected runtime under 10 ms. Compiled only.
**Manual acceptance:** Update the Ubuntu headless agent binary with a build containing this commit. Run `msc status` and compare with `systemctl status com.ctemple.msc2.agent.service --no-pager`. Confirm the CLI reports the actual state instead of a metadata error. Routine start/stop may be checked when managed Minecraft servers can safely be stopped. Existing unit files, data and journald logging should remain intact.

### P16.37 — Publish v0.1.21 through the build-only release workflow

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.21.md`, this plan.
**What:** Increment v0.1.20 to v0.1.21 and synchronize locked release identity. Publish current main through the existing four-platform build-only workflow, preserving all nine desktop/headless artifacts, checksums, and signed update metadata. Monitor completion and inspect all failures before targeted fixes or retries; keep the tag immutable. Cameron explicitly authorized publication and release-error fixes. Preserve the uncommitted agent heading edit locally.
**Verify:** `gh release view v0.1.21 --json tagName,isPrerelease,assets,url`
**Batch:** P16.37 only.
**Commit:** `P16.37: prepare v0.1.21 release`

**Preparation:** Inspected the complete active release workflow, previous successful release, changes since v0.1.20, and configured signing-secret/public-key names. No workflow gates or tests added; no tests run. Physical acceptance remains Cameron's responsibility.

### P12.247 — Hide initiation and main console scrollbars

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`, this plan.
**What:** Hide the initiation and main dock console scrollbars using the existing app pattern for standard and WebKit scrollbar styling, keeping overflow scrolling available. Owner-requested narrow visual change; no tests added or run.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.247 only.
**Commit:** `P12.247: hide initiation and main console scrollbars`

**Checks:** Svelte check passed with zero errors and eleven existing warnings outside the changed components. No Rust files changed; no tests or release workflows run.

**Manual acceptance:** Open Initiate Server and the main console, wait for enough console output to overflow, and confirm both scrollbars are hidden while the mouse wheel or trackpad still scrolls the output.


### P19.5 — Authorize Fedora complete uninstall once

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-platform-linux/src/uninstall.rs`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/uninstall.rs`, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Keep one protected installed MSC helper running after a single Fedora authorization prompt. Send it only fixed service-stop, system-data, verified package, and marked-archive actions while the existing copied worker checks the reviewed inventory and removes user-owned data. Preserve package ownership checks, ordered service shutdown before data deletion, the rule that a partial removal retains the installed command, and the current macOS/Windows uninstall paths.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P19.5 only.
**Commit:** `P19.5: authorize linux uninstall once`

**Evidence:** `LinuxUninstall` previously launched `pkexec` separately for every `systemctl`, `rm`, and `rpm` command. Fedora therefore prompted Cameron repeatedly during one confirmed removal. The installed agent is a root-owned executable that can accept a narrow privileged cleanup command after one OS authorization. The copied unprivileged worker still owns inventory comparison and user data cleanup; the privileged helper accepts no caller-supplied filesystem paths.
**Checks:** Rust formatting, shipping-agent Clippy, and Fedora RPM build pass with existing unrelated warnings. No tests or destructive uninstall ran; the installed RPM and live services were not changed.
**Manual acceptance:** Install an RPM containing this commit, open Settings → Uninstall MSC 2, review the inventory and confirm. Fedora should request authorization once for the complete local removal, then show a completed result. Verify `rpm -q msc-2` reports it absent and the local MSC services are gone. Test a partial-failure report by observation only if it occurs; do not interrupt or deliberately damage a live removal. Other platforms retain their previous uninstall code.

### P12.246 — Install live Bedrock map players for every active world

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/assets/bedrock-map-feed/`, `crates/msc-application/src/bedrock_map_feed.rs`, `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/bedrock_map_feed.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, this plan.
**What:** Bundle the existing Bedrock player-position behavior pack with the agent and register it for the active world before each stopped-server start, including newly created and later selected worlds. Preserve other behavior packs and update an installed proof pack in place. Remove Bedrock feed lines, Java feed lines, and Java position-query replies and command echoes before they enter MSC's console, regardless of Auto-hide. Keep Java's existing fallback query path and all platform runtime/service implementations unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.246 only.
**Commit:** `P12.246: install bedrock map feed automatically`

**Evidence:** The live Fedora Bedrock server reported its online player while `/v1/worlds/map/players` remained `awaiting-feed`; its new world had no feed pack or world pack registration. The prior pack existed only under the manual map-proof tool. Java already polls position fields through built-in console commands when no server feed exists, so it needs console filtering but no pack installer. The agent currently pushed both feed messages and Java query replies into the console buffer, where disabling Auto-hide exposed frequent coordinate lines.
**Checks:** Rust formatting, focused regression compilation, shipping-agent Clippy, and desktop agent build/staging pass. No tests were run, and the running agent and worlds were not changed.
**Essential coverage:** An isolated filesystem regression installs the bundled feed in an active world, checks preservation of another pack and idempotence, then switches the active world and checks registration there. A small console-filter regression checks that Bedrock roster lines and Java coordinate replies/command echoes are hidden while ordinary server events remain visible. These catch the observed new-world `awaiting-feed` failure and the reported coordinate spam without Minecraft, network, timing, global state, or credentials. Expected runtime under 10 ms total; compiled only, not executed.
**Manual acceptance:** Rebuild the desktop, Repair the agent, and create or select a new Bedrock world. Start it, join, open View Map, and confirm live player coordinates appear and update. Turn Auto-hide off and confirm map feed positions never appear in MSC's console. Start and join a Java server, confirm live players appear using the existing query path, and confirm its position and rotation replies do not appear in the console. Repeat on Windows and macOS when available; the shared start and console logic applies there, while platform runtime/service code is untouched.

### P12.245 — Keep Fedora Bedrock shutdown from cancelling Broadcast

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/bedrock_linux.rs`, `crates/msc-application/tests/bedrock_linux.rs`, this plan.
**What:** Apply the P12.239 native Windows shutdown correction to the Linux Bedrock runtime. Consume queued process exits before escalating a graceful stop, clear stop timing when the process exits, and leave later stopped-state polls inert. Keep true 20-second forced shutdown for a live process. Windows and macOS runtime source is unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.245 only.
**Commit:** `P12.245: preserve broadcast through linux bedrock shutdown`

**Evidence:** The live Fedora journal shows successful first-run and Playit account operations followed by two Xbox Broadcast operations cancelled without error; the desktop therefore displayed its generic setup failure. The native Linux runtime still ran its 20-second graceful-stop check before draining a queued Bedrock exit and retained the stop deadline after setting its state to Stopped. The lifecycle error handler stops helpers and aborts first-start on that invalid force-stop. P12.239 corrected the same sequence on Windows only and explicitly identified the Linux path as still affected. This explains the observed cancellation pattern; a live Broadcast sign-in has not yet been repeated after the correction.
**Checks:** Rust formatting, shipping-agent Clippy, focused Linux regression compilation, and `npm --prefix clients/desktop-web run prepare:agent` passed with existing unrelated warnings. No tests ran and the live service was not restarted.
**Essential coverage:** One fake-process/fake-clock regression now covers a clean Linux exit observed before or after the stop deadline, then repeated stopped-state polls beyond it. It catches the observed helper-cancelling runtime error without Minecraft, provider calls, sleeps, or real credentials. It uses the existing ephemeral UDP-port fixture and should run in under 10 ms; compiled only, not executed.
**Manual acceptance:** Restart `npx tauri dev`, Repair the Fedora agent with this staged build, then retry Xbox Broadcast from the Bedrock initiation sheet. Confirm the Microsoft sign-in code appears, authentication completes, and pass two finishes with the server stopped and Playit configured. Waiting beyond the first pass's 20-second shutdown deadline must not cancel Broadcast. Recheck the Windows and macOS initiation flows; their runtime sources were not changed.

### P12.244 — Recognize Linux Playit connections during Bedrock setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/playit.rs`, `crates/msc-application/tests/playit.rs`, `crates/msc-agent/src/routes/networking.rs`, this plan.
**What:** Remove ANSI control sequences from Playit's connection line before matching its agent ID. If setup starts a Playit helper but the connection wait fails or is cancelled, reset that helper before returning the error. Leave existing helpers, saved credentials, cloud agents, and tunnels intact. Plain Windows and macOS output remains accepted.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.244 only.
**Commit:** `P12.244: recognize linux playit connections`

**Evidence:** The installed Playit v1.0.10 Linux binary emits ANSI sequences between log field names and `=`. MSC searched for literal `agent_id=`, so it could not recognize a successful connection and reported a 75-second timeout. The setup route marked a helper as newly started only after that wait succeeded, leaving its own helper running after this failure. Upstream Playit enables ANSI output on Linux but not Windows/macOS.
**Checks:** Rust formatting, shipping-agent Clippy, and compilation of the focused application regression target passed with pre-existing unrelated warnings. `npm --prefix clients/desktop-web run prepare:agent` built and staged the corrected agent and its existing helper bundle. No tests ran, and the live service was not restarted.
**Essential coverage:** The existing connection-recognition regression now supplies Playit's actual Linux ANSI field formatting. It catches the observed 75-second setup failure; a separate existing case retains plain output coverage. The fake process supplies controlled bytes with no network, files, timing, or live credentials. Expected runtime under one millisecond; compiled, not executed.
**Manual acceptance:** Repair the desktop agent from this staged build, then retry the saved Playit setup for Bedrock. Confirm it reaches tunnel provisioning and shows an address without waiting 75 seconds or creating another cloud agent. Also confirm a cancelled or failed setup does not leave a newly started Playit helper running. Check Playit setup on Windows and macOS with their plain logs.

### P12.243 — Align Fedora development Repair with staged agent bundles

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-platform-linux/src/service.rs`, this plan.
**What:** Make the elevated Linux helper verify the same agent, Vantage renderer, and Bedrock map exporter digest that the desktop uses for a staged development build. Copy all three verified executables into the root-owned system build directory so the repaired agent can find its map tools beside itself. Packaged agent installs retain their existing system-package path; Windows and macOS service paths are unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.243 only.
**Commit:** `P12.243: align linux dev repair with staged agent bundles`

**Evidence:** The live Fedora `tauri dev` Repair command returned `staged MSC executable does not match its content-addressed build name` from the installed elevated helper. The staged directory name matches SHA-256 over `msc`, `vantage`, and `bedrock-map` in that order, while the old helper compared it with SHA-256 of `msc` alone. It rejected the valid bundle before changing the running service. The agent resolves both map tools beside its own executable, so the helper must copy the complete bundle when installing the system-owned dev build.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated warnings. The live staged bundle's three-file digest matched its directory name. No tests ran. A corrected RPM is needed to replace the installed elevated helper before development Repair can use this fix.
**Manual acceptance:** Install a Fedora RPM containing this commit, restart `npx tauri dev`, then click Repair agent and approve the OS prompt. Confirm the dev desktop connects and the service runs from `/usr/lib/MSC 2/agent/dev-builds/<digest>/msc`; confirm its sibling `vantage` and `bedrock-map` executables are present. Confirm a packaged desktop still connects using `/usr/lib/MSC 2/agent/msc`.

### P12.242 — Decode desktop API frames on Fedora

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/auth/desktop.ts`, this plan.
**What:** Normalize Tauri's binary API response to bytes before decoding its length, headers, and body. Fedora WebKit returns the response as a JavaScript number array; other desktop runtimes may return an ArrayBuffer. Keep the Rust agent, native authorization, and Windows/macOS response content unchanged.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.242 only.
**Commit:** `P12.242: decode desktop api frames on fedora`

**Evidence:** On the installed v0.1.20 Fedora desktop, the native `desktop_bootstrap_local` call succeeds. The same desktop's `desktop_authorized_request_binary` returns an Array of 704 numbers for `/v1/me`, while the frontend passed it directly to `DataView`, which requires an ArrayBuffer. That exception prevents the app from reading successful API responses and leaves the setup screen at its generic unavailable state. The running agent, credential helper, saved desktop credential, and all four startup API endpoints were healthy. The fix converts either IPC shape to a byte view before reading the existing frame format.
**Checks:** Svelte check passed with zero errors and eleven existing warnings; production frontend build passed. No test suite ran. The installed RPM still contains the old frontend until a corrected package is built and installed.
**Manual acceptance:** Install a desktop package containing this commit on Fedora, open MSC, and confirm the local agent connects without another repair. Confirm Repair reconnects. Check the same packaged desktop connection on Windows and macOS; their native authorization and service installation paths were not changed.

### P16.36 — Publish v0.1.20 through the build-only release workflow

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.20.md`, this plan.
**What:** Increment v0.1.19 to v0.1.20, synchronize locked release identity, and include the required Windows Tokio dependency correction in the Tauri lockfile. Publish current main through the existing release workflow only; preserve nine artifacts, checksums, and signed update metadata. Monitor until completion, inspect failures before any targeted retry, and keep the tag immutable. Cameron explicitly authorized publication and fixes for release failures. No CI/test workflows or test execution.
**Verify:** `gh release view v0.1.20 --json tagName,isPrerelease,assets,url`
**Batch:** P16.36 only.
**Commit:** `P16.36: prepare v0.1.20 release`

**Preparation:** Inspected the full release workflow and prior successful v0.1.19 run. Only release.yml is active. Signing-secret and public-key variable names are present. Locked Cargo metadata resolves for the agent and desktop manifests; version fields are synchronized. Publishing will run the existing four-platform build matrix; physical acceptance remains separate.

### P18.10y — Retry maps after an initially empty Bedrock world

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`, this plan.
**What:** Reject and remove empty dimension catalogs instead of retaining them indefinitely. When no rendered dimension remains, release the old saved-world snapshot so the next map request captures fresh data. Keep a shared snapshot while a populated dimension still uses it. Preserve readiness gating, save-resume cleanup, exporters, Java maps, and platform runtimes. This shared Bedrock cache correction applies on Windows, macOS, and Linux.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P18.10y only.
**Commit:** `P18.10y: release empty bedrock map snapshots for retry`

**Evidence:** Bedrock 2's first map snapshot at 17:43:27 preceded Cameron's player connection at 17:45:09. The snapshot database had two tiny .ldb files and a 64-byte log; its exported Overworld manifest had zero tiles and no spawn. The live database subsequently had a roughly 2.9 MB log. The cache retained both the empty catalog and original snapshot, so reopening could never see the later generated world. Console output confirmed the previous readiness fix resumed saves promptly after its successful repeated query.
**Checks:** Rust formatting, shipping-agent Clippy, and agent build passed with existing unrelated Windows warnings. `cargo rustc -p msc-agent --bin msc --profile test -- --emit=metadata` compiled the binary unit-test target without executing tests or including the unrelated broken CLI integration-test target. Built agent staged in desktop development/package resource directories with matching SHA256 hashes; the running service was not restarted or repaired. Live rendering and macOS acceptance remain Cameron's verification.
**Essential coverage:** One controlled regression rejects an empty Overworld catalog and checks that its temporary catalog/snapshot files are released, then checks an empty Nether catalog leaves a populated Overworld and its shared snapshot available. This protects the observed persistent empty-map failure and prevents invalidating an already usable dimension. Unique automatically cleaned temporary directories; no Minecraft, exporter, network, sleeps, or timing assumptions. Expected runtime under 10 ms; compiled only, not executed.
**Manual acceptance:** Stop Minecraft, Repair the agent using the staged development binary, restart Bedrock 2, join it, and open View Map. Confirm Overworld terrain renders. On a fresh server, open the map before joining, then join and reopen; confirm an initially empty result no longer prevents later terrain from loading. Confirm an unvisited Nether still reports empty without breaking the populated Overworld. Repeat the map retry on macOS Bedrock. No live worlds or existing user snapshots were changed by the investigation.

### P12.241 — Preserve Java's initial slot through fresh-server registration

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/provisioning.rs`, `crates/msc-application/src/worlds.rs`, `crates/msc-application/tests/provisioning.rs`, this plan.
**What:** Once fresh Java creation has saved its profile and active slot ID, record the existing world-reconciliation marker for that owned initial slot. The shared creation finalizer covers download-and-go and installer-based Java server families. A custom generation data pack can create a world folder before level.dat exists; import recovery must not turn that preparation into a second slot. Keep this slot archive-less so Minecraft still generates the world on first start. Imported Java servers retain existing reconciliation/archive comparison; Bedrock creation and platform runtimes are unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.241 only.
**Commit:** `P12.241: preserve fresh java slots during registration`

**Evidence:** Java generation_properties creates a world folder for custom biome/generator settings. Registration counts an existing folder as live data; an archive-less initial slot then triggers a second slot. Java's preparation lacks level.dat, so creating a world archive here would violate archive/activation expectations. Recording the owned fresh slot as reconciled prevents import recovery from adopting the generation pack as another world. Normal fresh creation previously reached the same marker through the archive-less/no-live-folder reconciliation branch.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. `cargo check -p msc-application --test provisioning` compiled the focused regression with existing warnings. The updated agent is rebuilt/staged for desktop Repair. No tests executed or live world/slot data changed.
**Essential coverage:** One regression covers fresh Java creation and registration with and without custom generation. Assert one original active slot, identical saved profile, preserved generation-preset contents, no pre-generated level.dat, and no world archive. Existing fake download transport and unique automatically cleaned temporary directories avoid real Java/Minecraft, live networks, ports, sleeps, and clock assumptions. Expected runtime under 100 ms; execution remains with Cameron.
**Manual acceptance:** Repair with the staged agent. Create a Java server with a custom Biome Source or Generator Options, confirm one slot before/after initiation, and confirm the chosen generation settings apply. Also confirm normal default-world generation. Existing duplicate entries are retained; this prevents duplicates in new fresh Java servers across platforms.


### P18.10x — Retry Bedrock map readiness while save preparation finishes

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/backup_operations.rs`, this plan.
**What:** Bound each live Bedrock `save query` confirmation wait to 500 ms within the existing ten-second overall budget. This lets the existing readiness loop query again after BDS initially reports that save preparation is incomplete. Preserve immediate readiness, same-run console boundaries, required readiness before map copying, and save-resume cleanup. Keep Java flush confirmation's full wait and leave platform runtimes, terrain exporters, and frontend rendering unchanged. The shared correction also applies to macOS/Linux Bedrock and live Bedrock backup readiness.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P18.10x only.
**Commit:** `P18.10x: retry live bedrock map save readiness`

**Evidence:** The running Windows BDS console recorded Saving followed immediately by A previous save has not been completed at 17:23:48 and 17:26:44, followed by Changes to the world are resumed approximately ten seconds later. The production waiter previously spent the entire overall budget on the first query, preventing the existing retry loop from sending a second. The bundled BDS how-to explicitly requires repeated queries until preparation finishes. Java instead receives a completion response to `save-all flush`; it does not use Bedrock's query protocol. MSC 1's checkout is unavailable on this Windows machine; no source there was modified.
**Checks:** Rust formatting, shipping-agent Clippy, and agent build passed with existing unrelated Windows warnings. `cargo check -p msc-agent --bin msc --tests` compiled the binary's unit-test target, including the new regression, but the overall command failed on existing `cli_service` integration-test references to removed CommonArgs fields base_url/host/port/token. Those unrelated tests remain unchanged. No tests were executed. Built agent copied to the desktop development/package resource directories with matching SHA256 hashes; the running service was not restarted or repaired.
**Essential coverage:** One regression exercises the production wait loop with a simulated clock: first query not ready, a later query ready inside the overall budget, immediate readiness, a Java flush completing after the Bedrock query interval, and readiness never arriving before the overall timeout. Existing application fakes returned immediately and missed the production waiter exhausting the deadline. No real sleeps, server processes, files, ports, or network assumptions; expected runtime under one millisecond. Compiled only; execution remains Cameron's decision.
**Manual acceptance:** Stop the running Minecraft server, apply desktop Settings → Repair to load the staged agent, then start Bedrock and open View Map while it is running. Confirm terrain appears and world saving resumes; refresh/reopen and check another saved dimension. Repeat the live map on macOS Bedrock and on a Java server. Actual Windows terrain rendering and macOS compatibility remain owner verification; build/compilation alone do not establish those results.

### P12.240 — Keep the initial Bedrock world in its original slot

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/provisioning.rs`, `crates/msc-application/tests/bedrock_imports.rs`, this plan.
**What:** After applying a fresh Bedrock world's creation profile, archive its initial live folder into the existing creation slot using the existing slot-update operation. Preserve slot identity, name, timestamp, and profile; return the updated slot. Registration can then verify the live world matches that archive instead of creating a second slot. This applies to fresh Bedrock creation on all platforms; Java creation and imported-world recovery branches are unchanged. No live user slots or worlds are removed.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.240 only.
**Commit:** `P12.240: archive fresh bedrock worlds into their initial slot`

**Evidence:** Cameron's Bedrock server has one live world folder and two same-name/same-timestamp slots. The original slot contains only its creation profile; the second, active slot has world.zip and detected world metadata. Fresh Bedrock creation applies its profile by writing level.dat, while registration's import reconciliation treats a live folder alongside an archive-less slot as a separate world and creates a new slot. Archiving into the original slot before registration closes that mismatch without weakening recovery for actual imported or mismatched world data.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. `cargo check -p msc-application --test bedrock_imports` compiled the focused regression target. The agent is rebuilt/staged for desktop Repair. No tests executed or live world/slot data changed.
**Essential coverage:** Extend the existing fresh Bedrock creation regression through the same import-reconciliation call used by server registration. Assert registration recognizes the original slot, only one slot remains, and the complete profile is unchanged. Uses the existing unique temporary directory with automatic cleanup and a tiny local world archive; no Bedrock process, network, port, or sleep assumptions. Expected runtime under 100 ms; execution deferred to Cameron.
**Manual acceptance:** Repair with the staged agent, create a new fresh Bedrock server, and confirm Worlds shows exactly one slot before initiation and after both initiation runs. Confirm its selected seed/gameplay settings and active identity remain intact. Existing duplicate entries are retained; this correction prevents their creation in new servers.

### P12.239 — Clear Windows Bedrock shutdown tracking after exit

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/bedrock_windows.rs`, `crates/msc-application/tests/bedrock_windows.rs`, this plan.
**What:** Clear graceful-stop timing and forced-stop tracking when Windows Bedrock exits. Restrict deadline escalation to the Stopping state and consume queued process exits before attempting forced termination. Continue observing the termination event when a force-stop is needed. This prevents status polling after pass one from raising a nonexistent-process error, cancelling Playit/Broadcast, and aborting initiation. Linux/macOS runtime sources are unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.239 only.
**Commit:** `P12.239: clear windows bedrock shutdown tracking after exit`

**Evidence:** The current Windows pass one stopped at 17:00:03, followed by Playit helper cancellation/stop at 17:00:24 and two Broadcast operations cancelled as Xbox Broadcast stopped. Windows poll_event checked its stale 20-second graceful-stop deadline before checking whether the process still existed. The error handler stopped both helpers and aborted the first-start coordinator on each later poll. P12.238 corrected setup display and click handling but did not address this runtime error. Linux source contains a similar timer pattern; investigation/fixes there are outside this Windows change and no Linux behavior has been modified.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. The focused Windows runtime regression target compiled with `cargo check -p msc-application --test bedrock_windows`; no tests executed. The agent is rebuilt/staged for desktop Repair. Live acceptance remains with Cameron.
**Essential coverage:** One fake-process/fake-clock regression exercises clean exit observed both before and after the shutdown deadline, then repeats stopped-state polling past the deadline. It asserts one clean termination, no subsequent error/events, and no forced termination. Existing coverage checks real deadline escalation but missed polling after clean exit. It reuses the existing ephemeral UDP-port setup; no real Bedrock, provider requests, or sleeps are used. Expected runtime under 10 ms; execution deferred to Cameron.
**Manual acceptance:** Repair the Windows service using the newly staged agent. Initiate Bedrock with Playit and Broadcast; leave the stopped connection stage open beyond 20 seconds, complete Playit, and click Xbox Set up. Confirm Microsoft sign-in appears and the helper remains running while authenticating. Complete sign-in, confirm pass two runs, then confirm the final Minecraft server is stopped. Existing server/configuration/credentials can be retained; no fresh-install reset is needed for this correction.

### P12.238 — Separate initiation setup from connection readiness

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/BroadcastAuthSheet.svelte`, `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-application/tests/xbox_broadcast.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/routes/servers.rs`, this plan.
**What:** Call Broadcast setup from the actual click handler. Represent successfully saved Playit credentials/tunnels as Configured during the stopped setup stage, remove its credentials action, and explicitly start/check Playit again in pass two. Monitor its operation failures instead of leaving arbitrary failures dependent on prose matching. Prevent duplicate Broadcast starts, bound startup before sign-in, preserve a device code when closing the initiation prompt, and clear prompts on helper stop/failure/cancellation/timeout. Keep the initiation coordinator as prompt owner while its sheet is hidden. Offer Continue without Broadcast after a failure; the explicit action disables Broadcast in the saved server settings, stops its helper, and excludes it from pass two. Keep the existing stopped-server completion guard and Playit-only public-address summary.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.238 only.
**Commit:** `P12.238: repair initiation connection setup and recovery`

**Evidence:** Live Windows operations showed successful Playit account/tunnel provisioning alongside a cancelled helper start and successful helper stop. Status contained the saved key/address with isRunning false and no error note. The previous UI always reset successful setup to Waiting and required a running helper to leave it; Set up then reopened credentials. P12.232 also introduced a Broadcast click handler that referenced its function without calling it; P12.237 changed availability but retained that error. The exact caller of the historical helper stop remains unproven; configuration-stage success no longer depends on that helper remaining alive, and pass two explicitly starts/checks it again. No live sign-ins or account/provider changes were performed during implementation.
**Checks:** Svelte check passed with zero errors and eleven existing warnings; production frontend build passed. Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. The focused Broadcast test target compiled without execution. The agent is rebuilt/staged for desktop Repair. Checks establish compilation/build, not live Windows, macOS, or Linux acceptance.
**Essential coverage:** Extend the existing cancellation/watchdog test to assert discarded device codes, and add one fake-process regression covering stop and crash after a prompt. This protects the observed late sign-in sheet without networks, real Java, sleeps, or wall-clock assumptions; expected runtime under 10 ms. Test execution remains with Cameron.
**Manual acceptance:** Rebuild the desktop and Repair its agent. Initiate a fresh Bedrock server with Playit and Broadcast: after tunnel creation, confirm Configured and no repeated credentials action; click Xbox Set up and confirm Starting then the Microsoft code; close/reopen the code without another helper launch; authenticate, observe pass two, and confirm the final server is stopped with the Playit public address and friend name. Check a failed Broadcast launch offers Retry and Continue without Broadcast, and that choosing the latter disables it in settings, completes pass two, and leaves no late prompt. Existing saved Playit credentials must avoid another login. Check the same flow on macOS/Linux; these shared changes retain the platform runtime implementations.

### P12.237 — Make Xbox setup actionable after Playit readiness

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, this plan.
**What:** Preserve Xbox setup's existing Playit-attempted unlock condition and additionally allow the displayed Playit Ready state to unlock it. If the agent already has a Microsoft sign-in prompt, use the button to reopen it instead of disabling the button. Label that action Sign in, and explain when Playit setup still blocks the action. Preserve the existing Broadcast launch/sign-in-before-pass-two sequence. This shared frontend change applies to all platforms; it does not remove any previously allowed setup action on macOS/Linux.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.237 only.
**Commit:** `P12.237: unlock broadcast setup from playit readiness`

**Evidence:** Cameron reports Set up stays completely unchanged on click. Inspection found two independent disabling conditions: a separate Playit attempted flag and any existing Broadcast prompt. These could leave setup disabled while the sheet showed Playit Ready or while sign-in was available. The live agent currently reports no running Broadcast/prompt and the active server has no Broadcast working directory; this does not establish which condition applied to the earlier click. Both disabled-action paths are corrected without claiming a reproduced Microsoft sign-in.
**Checks:** Svelte check passed with zero errors and eleven existing warnings. No Rust changes or tests run.
**Manual acceptance:** Rebuild the desktop frontend and initiate a Windows server with both helpers. After Playit becomes Ready, confirm Xbox Set up is enabled and changes to Starting on click. Confirm the Microsoft sign-in sheet appears, closing it offers Sign in/Show code again, and reopening does not start a second helper. Authentication should continue the existing pass-two/shutdown sequence. Physical acceptance remains pending.

### P12.236 — Recognize executable files on the real Windows filesystem

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-infrastructure/src/fs.rs`, `crates/msc-infrastructure/tests/java_runtime_detection.rs`, this plan.
**What:** Replace the Windows filesystem's unconditional `executable: false` stub with a file-and-extension check for exe/com/bat/cmd. Preserve the exact Unix execute-permission calculation. This allows the discovery and normalization fixes from P12.233–P12.235 to accept the installed Java 25 executable.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.236 only.
**Commit:** `P12.236: recognize windows executable files during java discovery`

**Diagnosis:** The newly repaired running agent still returned only configured Java 21. Source inspection found that `StdFileSystem.stat` used a non-Unix placeholder always returning false for executable files. Runtime discovery requires both `is_file` and `executable`, so no real Windows JDK could pass even after correcting filenames/search roots. Earlier fake-filesystem coverage marked java.exe executable and missed this production boundary; environment-only fixes were incomplete.
**Checks:** Rust formatting, shipping-agent Clippy and compilation of the discovery regression target passed with existing unrelated warnings. No tests run. The agent was rebuilt/staged for desktop Repair; the running service is not replaced by building alone.
**Essential coverage:** Add one Windows-only real-filesystem regression with a temporary metadata-only java.exe, a text file, and a directory ending in .exe. Assert discovery and home normalization find the JDK, and reject non-executable files/directories. The fake executable is never launched; no Java install/network/timing wait is needed. A unique temporary folder is cleaned by an RAII guard even after an assertion failure. Expected runtime below 100 ms. Execution remains deferred to Cameron.
**Manual acceptance:** Load the freshly staged agent with Repair, reopen the Java picker, and confirm Local Temurin Java 25 is listed. The live CLI Java list should include its `bin/java.exe` path. No reinstall is required. macOS/Linux retain the same permission-bit behavior; physical acceptance remains with Cameron.

### P12.235 — Find Windows Java without an interactive Local AppData variable

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/versions.rs`, this plan.
**What:** On Windows, include `HOME/AppData/Local/MSC2/runtimes` alongside the agent-managed root and optional LOCALAPPDATA root. The desktop service explicitly records the installing user's HOME; discovery must not rely solely on an interactive LOCALAPPDATA variable. macOS/Linux paths are unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.235 only.
**Commit:** `P12.235: locate windows java from the service user home`

**Evidence:** Java 25.0.4.1 executed successfully again from Cameron's Local runtime folder. The repaired service binary and rebuilt `target/debug/msc.exe` have matching SHA-256 hashes, but the live Java list still returns only Java 21. Service metadata from the preceding diagnosis records `HOME=C:/Users/Cameron`. P12.234 added the environment-dependent Local root but did not cover missing/different service LOCALAPPDATA. This correction derives the known installed root directly from HOME; the service process's live environment has not been inspected, so that cause remains an inference until owner acceptance.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. `npm --prefix clients/desktop-web run prepare:agent` rebuilt and staged the debug/package agent successfully. No tests, service restarts, or Java downloads run.
**Manual acceptance:** Repair using the freshly staged agent, reopen the Java picker, and confirm the already installed Local Java 25 appears. No Java reinstall is needed. Live acceptance remains pending until the service runs this revision.

### P12.234 — Discover Windows Java installed under the default data folder

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/versions.rs`, this plan.
**What:** Include `%LOCALAPPDATA%/MSC2/runtimes` in Windows Java discovery as well as the current agent's managed runtime root. Preserve macOS/Linux search roots and leave existing installations in place.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.234 only.
**Commit:** `P12.234: discover windows java across agent data folders`

**Diagnosis:** The running service binary matched the rebuilt packaged agent by SHA-256. Its live `msc --json java list` returned only configured Java 21. Service metadata sets `MSC2_DATA_DIR` to `C:/Users/Cameron/AppData/Roaming/MSC2`, but the verified Temurin 25 executable resides under `C:/Users/Cameron/AppData/Local/MSC2/runtimes`. P12.233 fixed executable recognition but missed this separate root mismatch.
**Checks:** Rust formatting and `cargo clippy -p msc-agent --bin msc` passed with existing Windows warnings outside the change. Inspection confirms the additional root is guarded by `HostOs::Windows`; macOS/Linux discovery behavior is unchanged. No tests run.
**Manual acceptance:** Rebuild the packaged agent and Repair the Windows service, reopen the runtime picker, and confirm the existing Java 25 appears without another install. The CLI `msc --json java list` should include its Local `bin/java.exe` path. No service restart, credential changes, Java download, or tests performed by the agent in this step.

### P12.233 — Refresh newly installed Java in the Windows server picker

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `crates/msc-infrastructure/src/java_runtime_detection.rs`, `crates/msc-agent/src/routes/versions.rs`, `crates/msc-infrastructure/tests/java_runtime_detection.rs`, this plan.
**What:** Refresh the builder's runtime list after installation. Discover Windows `bin/java.exe`, recognize backslash-separated executable/home paths, and return the installed executable rather than its directory on Windows. Preserve the existing macOS/Linux installer return behavior, macOS bundle inspection, and preferred `bin/java` discovery/normalization.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.233 only.
**Commit:** `P12.233: refresh installed java in the windows server picker`

**Checks:** Svelte check passed with zero errors and eleven existing warnings. Rust formatting and shipping-agent Clippy passed with existing Windows warnings outside this change. The focused discovery test target compiled with `cargo check -p msc-infrastructure --test java_runtime_detection`; no tests run.
**Essential coverage:** One in-memory filesystem regression covers discovery and normalization of a Java 25 `java.exe`, then confirms `bin/java` remains preferred when present. It protects the reported missing Windows runtime and the owner's Unix compatibility requirement; no real JDK, network, timing or host paths are required. Expected runtime below one millisecond; execution deferred to Cameron. Existing Unix/macOS discovery fixtures remain unchanged.
**Manual acceptance:** Rebuild the desktop and agent. On Windows, create a Paper server requiring Java 25, install Java from the runtime picker, return with Okay, and confirm Java 25 appears selected with a `bin/java.exe` path and can be used to continue. Check Detect also finds already installed managed Windows runtimes. On macOS/Linux, confirm the existing Java picker/install flow still works; physical platform acceptance remains pending.

### P12.232 — Coordinate Bedrock initiation connection setup and shutdown

**Status:** Implemented; awaiting Cameron's Windows verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `crates/msc-agent/src/routes/networking.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, this plan.
**What:** Refresh Playit/Broadcast during the stopped connection-choice stage. Start Xbox Broadcast from its Set up action and finish its real setup operation/device sign-in before starting pass two; give failed setup a retry instead of treating every failure as a timeout. Hide the coordinator behind credential sheets and prevent late Broadcast prompts after completion. Feed Broadcast readiness to the agent's first-start coordinator outside the helper lock. Apply the existing safety limit to the Bedrock process pump. Confirm the server is stopped before displaying completion, and suppress port-forwarding addresses when Playit was selected. Require a true firstStartComplete result rather than the mere presence of that field.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.232 only.
**Commit:** `P12.232: coordinate initiation helper setup and shutdown`

**Checks:** Svelte check passed with zero errors and eleven existing warnings; Rust formatting passed. `cargo clippy -p msc-agent --bin msc` compiled successfully with existing Windows warnings outside the changed code. Strict all-target Clippy was blocked by the existing unused `BEDROCK_HELPER_SOCKET_MODE` constant; the broader non-strict run also found existing outdated `CommonArgs` fields in `tests/cli_service.rs`. Those unrelated files were left unchanged. No test suites or account/provider operations run, and no source-text assertion tests added.
**Manual acceptance:** On Windows, initiate a new Bedrock server with Playit and Xbox Broadcast enabled. Complete Playit and confirm its row updates without clicking Xbox setup. Click Xbox Set up and authenticate in the device-code sheet while the Minecraft server remains stopped. Confirm pass two begins only after authentication, completion waits for Minecraft shutdown, and the summary has the Playit endpoint and authenticated friend name without a port-forwarding endpoint. Close the result and confirm the server remains stopped and no late sign-in sheet appears. Also check a saved Playit key and an Xbox sign-in failure/retry. This account and process acceptance remains pending; local checks do not establish it.

### P12.231 — Remove repeated onboarding instructions

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `content/guides/onboarding.json`, `content/guides/onboarding-source-map.json`, `clients/desktop-web/src/lib/help/TourOverlay.svelte`, `clients/desktop-web/src/lib/help/SetupIntro.svelte`, this plan.
**What:** Remove repeated body/footer instructions and duplicate setup completion text. Fold the first-world introduction into Essentials. Suppress the world-review popup while retaining its Continue action listener. Give section-opening cards distinct titles, preserve their expansion actions, shorten add-on/create cards, and remove the repeated running-state claim from the later tour completion copy. Keep source mapping and order aligned with the remaining content.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.231 only.
**Commit:** `P12.231: remove repeated onboarding instructions`

**Checks:** Svelte check passed with zero errors and eleven existing warnings. No Rust changes or tests added/run.
**Manual acceptance:** Restart setup/tour. Confirm setup completion has one heading; each card gives its action once; Essentials follows connectivity directly; expansion and Okay buttons still advance through world options; the wizard's Continue advances from world settings without a review popup; add-on and creation cards dismiss correctly and creation finishes the tour.

### P12.230 — Hide scrollbars throughout first-time setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, this plan.
**What:** Hide scrollbars on every setup page and the nested Java runtime list, matching the outer first-launch window. Preserve scrolling so overflow content remains accessible.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.230 only.
**Commit:** `P12.230: hide scrollbars throughout first-time setup`

**Checks:** Svelte check passed with zero errors and eleven existing warnings. No Rust changes or tests run.
**Manual acceptance:** Open first-time setup and move through every page, including step 2 with Java and Bedrock selected. Confirm no scrollbars appear; confirm overflow content and long Java runtime lists remain reachable by scrolling.

> ## STATUS: Phase 16 step records P16.1–P16.34 and Phase 17 step records P17.1–P17.27 and all Phase 18 steps/substeps are Done at Cameron's direction and archived. [v0.1.18](https://github.com/ctemple9/msc2/releases/tag/v0.1.18) published all nine artifacts from `ededaf33632bbbdcc518ae8928a54bb3ba073cc6`. Outstanding physical acceptance, deferred checks and independent phase reviews remain separate from Done step status.
> **Next move:** Cameron records Phase 17 physical acceptance, then the other agent reviews its gate. Phase 16 still needs Cameron's exact-artifact results in `docs/msc2/release/phase16-acceptance.md` and an independent gate review. Phase 18 awaits independent review against its consolidated map acceptance record; named checks remain deferred. Done step statuses do not assert that pending gate evidence exists.

## How this document works

The vision documents describe where MSC 2 is going; the port plan defines the phase sequence and exit gates; this file records the active priorities and current phase state. Completed phase plans and historical step records live in `rolling-plan-archive.md`.

Each implementation step is planned, read, executed, verified by Cameron, reviewed against its phase gate, and then archived. A step's status records the completion state Cameron directs; phase acceptance evidence and independent review are tracked separately. Marking steps Done does not by itself close a phase gate.

## Current phase

| Phase | Name | State |
|---|---|---|
| Setup | Repo, docs, agent instructions, CI, editor config | complete |
| 0 | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| 6 | Worlds and backups | complete |
| 7 | Server families and provisioning | complete |
| 8 | Mods, plugins, modpacks | complete |
| 9 | Networking and helpers | complete |
| 10 | Bedrock runtimes | complete |
| 11 | Desktop and web clients | complete |
| 12 | Client redesign and post-phase corrections | complete |
| 13 | Full-screen terminal client | retired by D-034 |
| 14 | Operational refinements | complete |
| 15 | Maintenance follow-ups | complete |
| 16 | Release safety and codebase readiness | complete |
| 17 | Local CLI refinement | step records done; physical acceptance and independent review pending |
| 18 | Integrated 3D world map | all step records done by owner direction; named checks deferred, independent review pending |

## Active Phase 16 acceptance

The [v0.1.18 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.18)
completed its build and publish workflow with all nine platform artifacts.
Cameron's exact-artifact physical observations and the independent Phase 16
review remain outstanding. See `docs/msc2/release/phase16-acceptance.md` for
the release identity, published asset metadata, and pending acceptance rows.

P16.1–P16.34 and the September 17 and September 28 audits are preserved in
`docs/msc2/rolling-plan-archive.md`. Phase 16 remains in progress until its
exit gate holds.

## Active Phase 17 acceptance

P17.1–P17.27 are Done at Cameron's direction. Their complete records are in
[the archive](rolling-plan-archive.md#phase-17--local-cli-refinement), including
[completion steps](rolling-plan-archive.md#phase-17-completion-steps).
Physical acceptance and independent gate review remain outstanding. The
local CLI contract and acceptance guidance remain in
[Phase 17 CLI](clients/phase17-cli.md) and the [port plan](msc2-port-plan.md).

## Active Phase 18 acceptance

All Phase 18 steps and corrective substeps, including P18.11, are Done at
Cameron's direction. Their complete records are in
[the archive](rolling-plan-archive.md#phase-18--3d-world-viewer).
The [map acceptance record](phase18-map-acceptance.md) preserves owner
observations, measured costs and limits. Nether absent-source troubleshooting,
ATM10 custom-dimension visual proof and further runtime/platform checks remain
deferred. Independent gate review remains outstanding. The map work is merged
into main; no release publication or exact-artifact acceptance is implied.

## Owner-requested Phase 12 follow-up

### P12.194 — Redesign agent home and remove signal/status dots

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/App.svelte`, shared status components and their callers, player-presence/chat/setup-progress surfaces, `clients/desktop-web/tests/agent-install/agent-install.test.ts`, `clients/desktop-web/tests/screens/first-launch-reset.test.ts`, `clients/desktop-web/tests/components/base.test.ts`, `clients/desktop-web/tests/archive/agent-home/`, `docs/msc2/antiAIslop.md`, this plan.
**What:** Implement Cameron's reviewed HTML home design inside the existing desktop shell. Show the current agent, its actual server list and a permanently visible explanation of the app/agent/Minecraft relationship. Missing local service offers Install; installed local service offers one Start/Stop agent button plus Repair. Those native actions await the existing connection refresh; remove separate reconnect/disconnect buttons. Service controls remain scoped to the selected local host; a remote host's service remains managed on that computer. The remote dropdown offers Connect new host / View saved hosts and populates the chosen content below, retaining SSH/tunnel review, route selection, secure credentials, editing, pairing replacement and removal confirmation. The root route opens agent home; server Overview and explicit deep links remain available. Apply Cameron's 2026-10-02 app-wide prohibition on signal/status dots and record it in the design law. Preserve state information as text. Archive obsolete exact-source assertions that required the replaced page's wording/layout; retain the controlled behavioral checks for automatic installation/startup. No tests added or run; Rust formatting/lint are not applicable because no Rust is changed.
**Verify:** `npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build`
**Batch:** P12.194 only.
**Commit:** `P12.194: redesign agent home and remove status dots`
**Owner visual verification:** Open agent home with a missing, stopped and running local service; confirm Install or the single Start/Stop button plus Repair, and automatic connection refresh after each action. Open both remote-dropdown choices, edit/switch a saved host, and open a server's Overview. Confirm signal/status dots are absent in setup, service status, player lists and chat. These interactions remain Cameron's verification; builds do not assert physical service or remote-host acceptance.

**Agent checks:** Frontend type-check and production build passed after restoring the checkout’s dependencies with `npm ci --ignore-scripts`; 10 existing type-check warnings remain outside this change. Targeted frontend formatting and `git diff --check` passed. Tests were not run. Physical app/service/remote verification remains pending.


## Owner-requested release

### P16.35 — Prepare and publish v0.1.19

**Status:** Prepared; publication requested by Cameron, exact-artifact verification pending.
**Files:** `Cargo.lock`, `crates/msc-agent/Cargo.toml`, client package manifests/lockfile, Tauri package manifests/lockfile/configuration, bundle identity (source and static asset) and its existing assertion, `tools/release/stage-windows-agent.ps1`, `README.md`, `docs/msc2/release/v0.1.19.md`, this plan.
**What:** Increment the latest release tag from v0.1.18 to v0.1.19, preserving the current main-branch agent-home redesign, Cameron's subsequent adjustment and merged world-map work. Keep agent, desktop, frontend and locked package versions consistent. Include the two terrain helper executables and their existing license in Windows desktop staging, matching the other desktop/headless packaging paths. Push main and the new immutable tag once to trigger the existing build-only beta workflow, preserving all nine artifacts, checksums and signed update metadata. No workflow gates or tests added. The pre-existing Tauri Cargo.lock dependency edit remains uncommitted; stage only the release-version change from that file.
**Verify:** `gh release view v0.1.19 --json tagName,isPrerelease,assets,url`
**Batch:** P16.35 only.
**Commit:** `P16.35: prepare v0.1.19 release`

**Release preparation checks:** Locked Cargo metadata resolved for the agent and desktop packages; all release version fields agree at 0.1.19. Frontend production build and Windows staging PowerShell syntax inspection passed. Existing workflow signing-key variable/secret names are configured. No tests were run.


### P12.195 — Remove local service details disclosure

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Remove the local “Service details & pairing another desktop” disclosure and its contents from the Agents screen at Cameron's request. Retain the local Install/Start/Stop/Repair controls and remote host/pairing workflows. Preserve Cameron's uncommitted heading edit without including it in this step's commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.195 only.
**Commit:** `P12.195: remove local service details disclosure`

**Agent checks:** Frontend type-check passed with zero errors and 10 existing warnings; `git diff --check` passed. Tests were not run.


### P12.196 — Explain closing MSC and stopping services

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Replace the connection/pairing explanation with the approved “What happens when you close MSC?” guidance: running servers continue after the app closes; use the server’s Stop button for Minecraft and Stop agent for the agent. Remove the duplicated closing-window note from the left panel and its unused styling. Preserve Cameron's uncommitted heading edit without including it in this step's commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.196 only.
**Commit:** `P12.196: clarify closing the app and stopping the agent`

### P12.197 — Collapse the agent server list and open rows directly

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/sections/performance/PerformanceSection.svelte`, `clients/desktop-web/src/lib/sections/shared/server-uptime.ts`, `clients/desktop-web/src/App.svelte`, this plan.
**What:** Collapse the agent server list by default with a labeled disclosure and server count. Open Overview by clicking a keyboard-accessible server row with a quiet arrow. Place the selected server first and distinguish its running/stopped state using existing status colors without dots. Show server type/port and selected-server live players/RAM, refreshing only while the list is expanded and the screen is active. Share the existing Performance uptime semantics across tabs and the shell: count from an observed stopped-to-running transition; show Running when the start time is unknown. Forget host observations when the connection is reset, reject late list responses after host/server changes, and never present the selected server's stats on other rows. Preserve Cameron's uncommitted heading and Tauri lockfile edits outside this commit. No Rust changed; no tests added or run.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.197 only.
**Commit:** `P12.197: make agent server rows collapsible and show live stats`

**Manual verification:** Expand On this agent; confirm the selected server is first, click another row to open its Overview, and compare Players/RAM with that server's existing screens. Start a previously stopped server while connected and compare uptime across Agents and Performance. Reconnecting to an already-running server should show Running rather than inventing elapsed time.

### P12.198 — Match the selected server row to the rest of the list

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Remove the selected row's permanent gray background. Place its extra stats beside the existing type/port details rather than adding a third line, so rows share the same spacing and height when room permits. Allow details to wrap on narrow screens. Retain selected-first ordering, colored status, and direct Overview navigation. Preserve Cameron's uncommitted heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.198 only.
**Commit:** `P12.198: match selected server row styling to the list`

### P12.199 — Update managed Geyser helpers and surface load failures

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-infrastructure/src/geyser.rs`, `crates/msc-application/src/geyser.rs`, `crates/msc-application/src/addon_updates.rs`, `crates/msc-application/tests/addon_updates.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-domain/src/crash_analysis.rs`, `crates/msc-domain/tests/paper_plugin_crash_analysis.rs`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/sections/components/model.ts`, `clients/desktop-web/src/lib/sections/server-editor/StartupFailureSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/StartupFailurePanel.svelte`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, this plan.
**What:** In the installed-plugin action menu, replace View with Update for Geyser and Floodgate. Keep both helpers out of Modrinth update checks, including when a stale project link exists. On request, resolve GeyserMC's latest Spigot build, report when the installed version/build is current, or checksum-verify and atomically replace it while preserving the existing JAR on failure. Report the resulting version/build and that a restart is needed. Detect the CraftItemStack reflection error from Geyser's Paper startup output, record a plain-language Paper API incompatibility diagnosis with the relevant log evidence, and open the existing startup issue sheet when that helper fails even if Paper reaches ready. Add one controlled regression test for the reported Geyser failure signature; strengthen existing update-resolution coverage for stale Modrinth links. Do not run tests.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-domain -p msc-application -- -D warnings && cargo clippy -p msc-agent -- -D warnings -A dead_code && cargo check -p msc-application --test addon_updates && cargo check -p msc-domain --test paper_plugin_crash_analysis && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build && npm --prefix clients/desktop-web run api:check`
**Batch:** P12.199 only.
**Commit:** `P12.199: add managed geyser updates and startup diagnosis`

**Essential test rationale:** The new single regression case protects the user-reported Paper/Geyser failure from being reduced to a generic plugin load error and verifies that the diagnosis retains the CraftItemStack cause. Existing tests cover other Paper plugin failures and Geyser's separate minimum-Minecraft-version message, not this Paper API signature. The case uses two fixed console lines and one in-memory plugin entry, with no network or timing assumptions. Expected runtime: under one second. It is added but not run.

**Agent checks:** Rust formatting passed. Strict Clippy passed for `msc-domain` and `msc-application`; the `msc-agent` Clippy check passed with its pre-existing `dead_code` lint allowed because `auth::forbidden` is unused elsewhere. Both affected integration-test targets compiled with `cargo check --test`; no tests were run. Svelte check passed with zero errors and 11 existing warnings; production frontend build and API generation passed. Manual desktop verification remains pending.

## Phase 19 — Complete local uninstall (owner-requested 2026-10-02)

**Planning state:** Cameron authorized implementation on 2026-10-02. P19.1 is implemented pending owner verification; later steps remain planned.

**Owner-approved intent:** Add **Uninstall MSC 2…** beside Reset in app settings and a local `msc uninstall --danger` command. Permanently remove this computer's MSC 2 agent services, managed servers/worlds/backups, MSC-owned helpers and runtimes, host/client data and credentials, caches/logs, installed command, and desktop app. Never contact or uninstall a saved remote agent. Running this flow on a remote computer means running its local MSC desktop or CLI there. Preserve MSC 1, source checkouts, separately installed Java/Tailscale/Docker, unrelated files, and OS-owned package caches.

**Confirmation contract:** Both interfaces must show the actual computer, server root(s), installation(s), and exact deletion list before execution. Desktop: review sheet, acknowledgement of permanent world/backup loss, exact typed `UNINSTALL MSC 2`, then a final destructive confirmation dialog. CLI: `--danger` enables the destructive flow but does not bypass review; print the same inventory and require the exact phrase interactively. `--confirm "UNINSTALL MSC 2"` is the explicit non-interactive equivalent, used only after a separate `--dry-run` inventory review; refuse redirected input without it. `--dry-run` performs no writes, elevation, service changes, or cleanup. A native command must enforce the confirmation independently of UI state. Local cleanup must still work while the selected desktop host is remote and while the local agent is missing/offline; no HTTP uninstall route.

**Downloaded installers:** MSC cannot prove ownership of every renamed/moved installer or its original download location. Include verified MSC release installers in known download/update locations and let the operator explicitly select additional installers. Show every selected file before confirmation, validate its package/bundle identity or signed release checksum, and remove only those files. No filename-only recursive disk search. Never promise that an unknown original DMG has been found. A mounted disk image needs explicit unmount handling; if another application holds it open, report the remaining file instead of claiming full removal. Do not delete MSI/OS package-manager caches directly.

**Completion contract:** Gracefully stop Minecraft and MSC-managed helpers before removing services/data. If shutdown or service removal fails, stop and report what remains. Remove the app through its OS installation mechanism: verified macOS bundle removal, Windows registered MSI uninstall, Linux owning package removal (or a verified standalone AppImage). Use a narrowly scoped detached continuation where the running app/command cannot remove itself. The continuation must validate its inventory again, propagate failures, and leave a readable result outside the deleted MSC trees; let the owner choose whether to retain that report. “Scheduled” is not “Uninstalled.” Reject unsupported/dev installations rather than deleting a source checkout. Clean up continuation files when finished.

### P19.1 — Inventory local installations and define the deletion boundary

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-infrastructure/src/uninstall.rs` (new), `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/uninstall.rs` (new only for essential boundary cases), `docs/msc2/clients/local-uninstall.md` (new), this plan.
**What:** Build one serializable local inventory for CLI and desktop, with canonical paths, ownership evidence, missing/unavailable states, and exclusions. Discover service definitions and their actual MSC2_DATA_DIR/MSC2_APP_CONFIG_PATH/MSC2_AGENT_SERVERS_ROOT overrides rather than guessing from the current shell. Cover desktop and headless data layouts (macOS MSC 2 vs MSC2; Windows roaming/local data; Linux desktop and system headless roots), registered server roots and external managed server/backups with explicit ownership boundaries, client WebView state, credential stores, helper installs, old marked headless versions, command links/PATH entries, application/package identity, and verified installer files. Inventory is read-only; corrupt configuration or ambiguous ownership blocks affected deletion and is visible. Reject root/home/shared-parent targets, MSC 1 paths, symlink escapes, traversal, and unsupported developer checkout removal. Record essential controlled tests for path escapes, ambiguous ownership, and remote exclusion; do not run them without a specific owner instruction. Avoid brittle timing or real-machine installation fixtures.
**Verify:** `cargo check -p msc-infrastructure`
**Batch:** P19.1 only.
**Commit:** `P19.1: inventory local msc installations for uninstall`

**Implementation boundary:** The shared layer consumes local OS service/package inspection from platform adapters; P19.2 wires those native inspections and execution. Missing service/package inspection produces a blocking entry, never an assumption that an installation is absent. No CLI command or Settings uninstall action is available in this step. The preview function itself has no network, elevation, process control, or filesystem mutation.

**Essential tests:** Added six controlled in-memory deletion-boundary cases for protected/symlink/source targets, corrupt-config parent removal, ambiguous custom data roots, service override/secret separation, unverified/changed installers, and preview mutation. These protect permanent data loss or secret disclosure rather than structure/prose. Expected runtime is under one second after compilation. No tests were run.

**Agent checks:** Package type-check, compilation of the focused test target without execution, formatting, and package-library Clippy with warnings denied passed. No Rust test executables were run. Owner verification remains open.

### P19.2 — Remove services, data, credentials, and OS installation locally

**Status:** Implemented; awaiting Cameron's verification. Cameron authorized completion of P19.2–P19.4 together on 2026-10-02.
**Files:** `crates/msc-infrastructure/src/uninstall.rs`, platform-specific uninstall modules under `crates/msc-platform-macos/src/`, `crates/msc-platform-windows/src/`, `crates/msc-platform-linux/src/`, existing service/secret-store adapters where necessary, essential controlled boundary tests if current coverage misses a concrete risk, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Execute the inventory through existing platform privilege boundaries, with no remote service API. Authenticate to the local agent if available, request graceful server/helper shutdown and verify it; handle an offline/stopped installation through its inspected service definition without guessing process ownership. Stop/unregister all verified MSC-owned service/helper definitions, remove approved data and credential records, remove verified links/PATH entries, uninstall the desktop/package and marked headless artifacts, then delete the approved installer files. Distinguish Windows MSI uninstall from raw file deletion; use Linux package ownership and macOS bundle identifier checks. Secure detached continuation state against tampering; revalidate filesystem boundaries/ownership immediately before deletion. Never execute a user-writable elevated cleanup script blindly. Provide partial-failure/result reporting and retry inventory for leftovers, preserving failures rather than suppressing them. Do not launch the real uninstaller while implementing or verifying this step.
**Verify:** `cargo check -p msc-platform-macos -p msc-platform-windows -p msc-platform-linux`
**Batch:** P19.2 only.
**Commit:** `P19.2: implement complete local uninstall execution`

**Agent checks:** Host-platform checks and package-library Clippy passed without running tests or uninstall. Linux cleanup code also compiles on Unix for inspection. A Windows-target check was attempted but blocked by missing Windows C headers in ring (assert.h); Windows native compilation/acceptance remains required on Windows. The copied worker and CLI handoff are wired in P19.3; native execution is not invoked here.

### P19.3 — Expose the confirmed uninstall command

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/uninstall.rs` (new), CLI documentation/help, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Add `msc uninstall --danger`, `--dry-run`, exact `--confirm` for explicit automation, and validated additional-installer selection. Share inventory/execution with desktop rather than duplicating deletion logic. Ignore/refuse remote target overrides and obtain all destructive targets locally. Print warnings and inventory before the typed interactive confirmation. Return nonzero on blocked/partial cleanup and distinguish a detached scheduled action from actual completion. Keep ordinary `msc service uninstall` and host-reset behavior unchanged. Review CLI parsing/help through non-destructive checks; do not run a destructive invocation on the developer machine.
**Verify:** `cargo check -p msc-agent`
**Batch:** P19.3 only.
**Commit:** `P19.3: add confirmed local uninstall command`

**Agent checks:** Agent compilation and Clippy passed (one pre-existing auth.rs dead-code warning); no tests or uninstall commands ran. A private copied worker waits for the parent/desktop to exit, rediscovers and compares the inventory, authenticates only to the local agent to stop Minecraft, and records partial/failure outcomes. Explicit optional reports survive outside the deleted directories; otherwise successful worker files are removed. Native Windows acceptance remains pending as noted in P19.2.

### P19.4 — Add Uninstall MSC 2 to app settings

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`, `clients/desktop-web/src/lib/sections/app-settings/UninstallSheet.svelte` (new), `clients/desktop-web/src/App.svelte`, platform adapter types/implementation, `clients/desktop-web/src-tauri/src/lib.rs` and a native uninstall module if needed, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Read antiAIslop.md before frontend work. Add a separate destructive Uninstall action beside Reset. Show native local inventory independent of selected remote host; support verified additional-installer selection. Require the loss acknowledgement, typed phrase, and final dialog before invoking native execution. Enforce typed confirmation and inventory identity natively, disable duplicate submissions, and show OS elevation and failures clearly. Clear all local saved remote credentials/connections without contacting those agents. Exit only after native handoff is established; communicate scheduled continuation and its result location accurately. Existing Reset remains unchanged. Preserve unrelated owner edits and commit only this step's work.
**Verify:** `npm --prefix clients/desktop-web run check && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Batch:** P19.4 only.
**Commit:** `P19.4: add confirmed complete uninstall to settings`

**Agent checks:** Svelte check passed with only ten pre-existing warnings; desktop native compilation passed. No tests or destructive uninstall ran. Settings uses only the packaged local command, previews verified targets, enforces the loss acknowledgement/typed phrase/final dialog, and closes after private worker handoff. Reports default to the home folder. Installer discovery is bounded by retained signed metadata, and Windows native acceptance remains pending. JSON CLI worker output is detached so the scheduled response stays parseable; Linux elevated tools use fixed absolute paths; marked Windows archives remove their exact User/Machine PATH entry. Agent and desktop Clippy completed with only existing warnings.

**Phase 19 acceptance gate:** Cameron verifies both entry points on disposable installed MSC 2 environments for macOS, Windows MSI, and Linux desktop/headless packaging. Observe server shutdown, service/helper removal, data/credential/cache cleanup, OS package deregistration, self-removal, verified installer deletion, and readable partial-failure results. Check cancellation at each confirmation, no-write dry run, stale/tampered inventory rejection, missing/offline agent behavior, protected symlink/root paths, and that saved remote agents plus MSC 1 remain unchanged. Developer source trees are never used for destructive acceptance. The other agent independently reviews the deletion boundary and phase gate. No release workflow gates or release runs are added by this work.

### P12.199 — Show remote host actions directly

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Replace Connect remote agent and its dropdown with two visible buttons, Connect new host and View saved hosts (including the saved count). Retain the existing form/list underneath. Remove unused dropdown state, focus/Escape handling, and styles. Preserve owner heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.199 only.
**Commit:** `P12.199: show remote host actions as separate buttons`

### P12.200 — Update server-computer instructions for the Phase 17 CLI

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Rename Remote service commands to Commands on your server computer. Replace Linux service-name discovery and lower-level service commands with msc status/start/stop/enable/disable agent. Explain local terminal or SSH use as the installing account, support across all three platforms while the agent is stopped, separate Minecraft server controls, and the distinction between stopping and disabling boot startup. Remove the unsupported repair wording; preserve desktop pairing. Keep owner heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.200 only.
**Commit:** `P12.200: update agent instructions for the local cli`


## Phase 6 import correction — owner-requested 2026-10-02

### P6.52 — Accept external world archive folder layouts

**Status:** Implemented; awaiting Cameron's verification. Cameron explicitly authorized implementation in this conversation.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, `crates/msc-application/src/worlds.rs`, `clients/desktop-web/src/lib/sections/worlds/ImportWorldZipSheet.svelte`, this plan.
**What:** Recognize Bedrock world files at the archive root, in a named folder, or inside enclosing folders; retain existing MSC layouts. Accept `.mcworld` in the Worlds import picker. Normalize only the new slot archive, removing known macOS packaging metadata while preserving file contents, compression and permissions. Store the detected single Bedrock world folder name so activation opens that imported world. Also accept loose Java worlds and enclosing folders around Java worlds with their sibling dimensions. Keep generic source safety/CRC/size checks and strict final world-layout checks; reject unrelated files and ambiguous external multi-world bundles with an explanatory error. Existing MSC-format multiple Bedrock worlds remain supported. Original archives are untouched; failed normalization removes the partial archive/slot. MSC 1's `WorldSlotManager.createSlotFromZIP` copies ZIPs unchanged without structural checks; this correction preserves P16.4's stronger safety boundary while broadening input packaging. No API contract or release workflow changed.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_import_`
**Batch:** P6.52 only.
**Commit:** `P6.52: normalize external world archive layouts on import`

**Essential test rationale:** Three focused cases protect previously rejected external layouts, correct Bedrock folder identity, original archive preservation, retention of Java dimensions, and refusal of traversal, links, executable permissions, server configuration, unrelated entries and ambiguous bundles. Existing strict-layout tests do not exercise normalization. All inputs are tiny local ZIPs with controlled contents and independent temporary directories; no network, Minecraft runtime or timing assumptions. Expected combined runtime: under one second. Tests were added and compiled with Clippy but not run.

**Agent checks:** Rust formatting and Clippy for the affected libraries and archive test target; Svelte static check (zero errors, ten existing warnings). No test commands run.

**Manual acceptance:** Rebuild/restart the local app and agent, then import `/Users/camerontemple/msc2-servers/XqKXS4++O7k=.zip` unchanged through the Bedrock Worlds tab. With the server stopped, activate the imported slot and confirm the existing terrain loads. Also import a `.mcworld` file without renaming it. External bundles containing several worlds require individual imports; a selection screen remains a separate UI/API change.


### P6.53 — Let API imports reach external archive normalization

**Status:** Implemented; awaiting Cameron's verification. Follow-up to Cameron's report that the unchanged ZIP still receives the original error after P6.52.
**Files:** `crates/msc-agent/src/routes/worlds/import_activation.rs`, `crates/msc-agent/src/routes/worlds.rs`, this plan.
**What:** Remove the import route's premature activation-layout check. All desktop and CLI imports now reach P6.52's source safety checks, normalization, and strict final layout validation. Preserve invalid-archive HTTP 400 responses and consumed staging cleanup; failures are recorded on the exclusive import operation. Diagnosis confirmed that the running local agent contains the new normalization code, so the repeated rejection was a missed route-level check, not an outdated binary. Preserve activation/restore validation unchanged.
**Verify:** `cargo test -p msc-agent --bin msc world_backup_routes_staged_upload_import_round_trip`
**Batch:** P6.53 only.
**Commit:** `P6.53: normalize world imports before checking stored layout`

**Essential coverage:** Strengthen the existing staged-upload/import round-trip with a loose `level.dat` ZIP and assert that the stored archive contains `world/level.dat`. This directly catches a route precheck preventing normalization, the gap missed by P6.52's library cases. Reuses the existing controlled local ZIP and fake process/journal setup; no additional test count, network, live Minecraft or timing assumptions. Expected runtime remains under one second. Test execution is deferred.

**Agent checks:** Rust formatting and production agent Clippy passed (one existing unused `auth::forbidden` warning). Broader test-target compilation was blocked by pre-existing `crates/msc-agent/tests/cli_service.rs` references to removed `CommonArgs` fields (`base_url`, `host`, `port`, `token`); no tests ran and that unrelated file was not changed. Rebuild/restart the development app/agent and retry the original Bedrock ZIP unchanged for manual acceptance.


### P6.54 — Preserve directory types when normalizing ZIPs

**Status:** Implemented; awaiting Cameron's verification. Follow-up to Cameron's `unsafe archive entry: worlds/XqKXS4++O7k=/db/` import failure.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, this plan.
**What:** Write normalized directory markers with ZIP `add_directory`, retaining their permission bits. Raw copying remains only for file contents. The ZIP library's `raw_copy_file_rename` reconstructs options via `unix_permissions`, which strips entry type bits; directory markers therefore became regular-file entries and failed strict validation. Inspect original entry modes before copying to reject executables and non-regular types rather than let option reconstruction disguise them. Source archives and strict activation validation remain unchanged.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_import_`
**Batch:** P6.54 only.
**Commit:** `P6.54: preserve directory entry types during world import`

**Essential coverage:** Extend the existing Bedrock packaging case with an explicit `db/` directory marker (the actual failed entry) for every supported layout, and assert the stored directory type and original 0700 permissions. No new test count; existing tiny local fixtures, expected combined runtime under one second. No tests run.

**Agent checks:** Rust formatting and Clippy for the affected libraries and archive test target. Manual acceptance requires rebuilding/restarting the agent and importing the unchanged original ZIP.


### P6.55 — Show real world activation progress and status age

**Status:** Implemented; awaiting Cameron's verification. Cameron explicitly requested progress reporting after successfully importing the original Bedrock ZIP.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/worlds/activation.rs`, `crates/msc-application/tests/world_activation.rs`, `crates/msc-agent/src/routes/worlds/import_activation.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Report activation stages through the existing operation DTO: checking imported world, creating/checking safety backup, saving the outgoing slot, checking archive before extraction, extracting imported world, installing world, and applying settings. Count actual verified/copied bytes; measure compression source totals only for progress-aware archive creation. Show a flat inline Worlds progress panel with the stage, per-stage bar/percentage and byte counts when a total is known, elapsed time, and time since the stage/count last changed. Connection/read failures explicitly say status is unavailable and retry reads without reissuing activation or declaring it failed. Stop monitoring on destruction or host/server changes and ignore late responses. Existing ordinary backup/archive entry points remain available. Progress callbacks preserve source safety/CRC checks, cancellation boundaries, world-swap ordering and recovery behavior. Throttle agent publication to four updates per second, plus stage starts/completions, to bound journal writes. No API contract or release workflow changed. Read the anti-slop design law; the panel uses existing neutral tokens, one flat group, a functional bar, and text for state.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_activation_archive_progress_counts_real_bytes`
**Batch:** P6.55 only.
**Commit:** `P6.55: report world activation stages and byte progress`

**Essential coverage:** One new small local archive round-trip checks measured compression/verification/extraction totals, intermediate byte counts and monotonic counts, plus preserved extracted contents. Existing archive safety and cancellation coverage exercises the shared primitives. Extend the existing activation/profile case to ensure stage reporting accompanies the unchanged resulting settings. Both use controlled local files, with no network, live Minecraft, sleeps, or timing assertions; expected combined runtime under one second. Tests were compiled by Clippy but not run.

**Agent checks:** Rust formatting; Clippy for the affected libraries, archive/activation test targets, and production agent (one existing unused `auth::forbidden` warning); Svelte static check (zero errors, ten existing warnings). No tests or live world mutations run.

**Manual acceptance:** Rebuild/restart the development app and agent. Activate a sizable saved world while the server is stopped; verify the checking/backup/save/extraction stages and real byte bar, followed by success. During a long stage, observe elapsed time and last-progress age. A status read/connection failure must show retrying and last-agent-contact age, then resume tracking the same operation after recovery. Switching tabs retains tracking; switching host/server or destroying the view must stop its timer/polling and prevent late old-operation notices. Percentages describe the current named stage and can reset at the next stage; periods with no countable byte work show the stage/time rather than a simulated percentage. Progress age reveals inactivity but does not by itself prove a stall.


### P12.201 — Open Modrinth project links in the default browser

**Status:** Implemented; awaiting Cameron's verification. Cameron reported that View on Modrinth does nothing in the Paper datapack project sheet.
**Files:** `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, this plan.
**What:** Route the shared Modrinth project sheet's external anchors through the existing platform `openExternal` function, which invokes Tauri's validated OS browser opener. Ordinary `target="_blank"` anchors did not invoke that desktop command. Apply the same handling to About-description links and Source/Issues/Wiki/Discord anchors in this sheet. Preserve link destinations and styling; show browser-opening errors inline instead of silently failing. The shared sheet covers datapacks, mods and plugins. No native code, URL policy, API contract or release workflow changes; no new tests needed for this small wiring correction and no tests run.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.201 only.
**Commit:** `P12.201: open project detail links in the default browser`

**Manual acceptance:** In the Paper world's datapack browser, open Terratonic and click View on Modrinth; confirm the default browser opens the project page. Check the About wiki link and a project's Source/Issues/Wiki links through the same shared sheet. The existing desktop opener reports failed/unsupported URL launches inline. Similar plain anchors in the separate CurseForge Bedrock pack sheet are outside this step.


### P12.202 — Install datapacks with overlays and remember installed versions

**Status:** Implemented; awaiting Cameron's verification. Cameron clarified that the Terratonic install actually failed, and also requested recognition of previously installed datapacks.
**Files:** `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, this plan.
**What:** Select the outermost `pack.mcmeta` rather than the first member in ZIP order. Inspection of the actual Modrinth Terratonic 3.0.27 download (version cT2AsHrJ) showed seven metadata members: six overlay copies before the root copy; the old installer selected an overlay and rejected legitimate sibling files. Preserve all overlay contents and keep the common-root, path, symlink, size, compatibility and checksum checks. Read the selected world's saved Modrinth datapack records when opening the browser, pass their version IDs into project details, and label successful/pre-existing recorded installations Installed (staging remains Added). Record successful new versions immediately. Shared project details no longer mark a failed/cancelled background install as installed. The browser-link correction remains its own P12.201 commit. Manual/imported packs without saved provider/version identity cannot be matched to a Modrinth release by these labels; no identity is guessed. No live install performed and no release workflow changes.
**Verify:** `cargo test -p msc-application --test addons java_datapack_install_uses_outer_metadata_and_preserves_overlays`
**Batch:** P12.202 only.
**Commit:** `P12.202: install overlay datapacks and restore installed labels`

**Essential test rationale:** One focused regression case reproduces overlay metadata preceding main metadata, at archive root and inside an enclosing folder; checks preserved overlays/main metadata, prior-world backup, and refusal of ambiguous separate packs without changing the world. Existing addon tests did not cover datapack metadata selection. Uses tiny local ZIPs and a unique directory with automatic cleanup, no network, live Minecraft, sleeps or timing assertions. Expected runtime under one second. Test compiled but not run.

**Manual acceptance:** Rebuild/restart the app and agent; on a stopped Paper server, install Terratonic 3.0.27 from the datapack sheet and confirm success/Installed. Close and reopen the browser and project details; the same saved Modrinth version should remain Installed. Installation errors must remain errors rather than create Installed labels. World-generation effects and external datapack dependencies remain Minecraft/pack behavior, not proved by installation alone.


### P12.203 — Limit world datapack choices to datapack releases

**Status:** Implemented; awaiting Cameron's verification. Cameron reported Fabric/NeoForge mod releases labeled Compatible in the Paper datapack browser and requested a correction.
**Files:** `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, this plan.
**What:** Require the provider's datapack loader/type marker in the world datapack detail sheet before collapsing/filtering versions or computing the compatible-version summary. Exclude mod builds even if they match the server Minecraft version; do not use the general add-on browser's fallback to show them when no datapack remains. Base Stable-only fallback on datapack releases. Automatic search-result installation selects a matching datapack release, not the first same-Minecraft mod release. Reject non-datapack releases before download in the agent and before mutation in the application installer. Datapacks for other Minecraft versions remain visible with Other version; the existing installer still refuses versions that do not list the server version. Shared mod/plugin browsing retains its existing behavior. No API contract or release workflow changes.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.203 only.
**Commit:** `P12.203: exclude mod builds from world datapack installs`

**Essential coverage:** Extend the existing overlay-install regression with Fabric and NeoForge releases advertising the same Minecraft version and containing otherwise valid datapack metadata. Assert refusal and unchanged world bytes. Reuse controlled tiny local ZIPs, no additional test count or runtime assumptions; expected under one second. Tests compiled but not run.

**Manual acceptance:** Rebuild/restart app and agent. Browse the mixed Tectonic project from the Paper world's datapack browser: Fabric/NeoForge builds must be absent; only datapack builds can receive the Compatible/Other version badge and install controls. A mod-only release list must remain empty rather than fall back to incompatible builds. Install from the search result must select a matching datapack release. Compatible here means the provider lists the server Minecraft version, not proof of runtime or dependency behavior.


**Agent checks for P12.203:** Rust formatting, application regression test compilation with Clippy, and production agent Clippy passed (one existing unused `auth::forbidden` warning). Svelte static check passed with zero errors and ten existing warnings. No tests or live installs run.

### P12.204 — Manage installed world datapacks

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, this plan.
**What:** Clicking an installed Java datapack opens its status and Delete action, with confirmation. View datapack opens its saved Modrinth project inside MSC, including Installed labels; packs without a recorded catalog identity explain why no page is available. Extend existing pack management to delete Java datapack files, retain safety backup and profile rollback, and refresh/reapply stopped active worlds using their actual edition and configured level name. Java enable/disable is not offered because it requires a separate Minecraft activation-list change. Keep existing Bedrock controls.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.204 only.
**Commit:** `P12.204: add installed datapack deletion and project navigation`

**Essential coverage:** Extend the existing small local overlay fixture to delete all installed files while preserving level.dat, verify backup bytes, and refuse unsupported Java disable without mutation. No network or timing assumptions; expected under one second. Compiled with Clippy, not run.
**Manual acceptance:** Rebuild/restart app and agent. Click an installed datapack on a stopped Paper world. View datapack must open its in-app project page if a saved Modrinth source exists. Delete requires confirmation, removes the row and its files from the selected world, and stays removed after reopening and activating the world. Cancel leaves it installed. Active running servers refuse deletion.

### P12.205 — Match datapack actions to Components menus

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Replace the datapack action sheet with the same shared popup Menu used by Components. Show View and destructive Uninstall only, with matching selected-row styling and a chevron. View opens the saved in-app Modrinth project; disable View when no supported catalog identity exists. Uninstall switches the row to inline Uninstall?/Cancel/Uninstall confirmation, matching Components. Keep stopped-server protection and existing removal/backup behavior. Anchor keyboard-triggered menus to the row.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.205 only.
**Commit:** `P12.205: match datapack actions to components menus`

**Manual acceptance:** Click a datapack row: the popup must look like Components with View and Uninstall. View opens its in-app catalog page where available. Uninstall shows inline confirmation; Cancel preserves the pack, confirmed Uninstall removes it. Clicking away or Escape closes the popup. Svelte check passed with zero errors and ten existing warnings; no tests added or run.

### P12.206 — Check Chunker updates and quiet conversion guidance

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldConversionWizard.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-infrastructure/src/chunker.rs`, `docs/msc2/worlds/phase6-api.md`, this plan.
**What:** Remove the orange side rail and inset from conversion guidance. Add an explicit Check for Chunker updates action on preflight when installed. Report the official latest release, up-to-date status, or lookup failure; offer Update Chunker when the release differs or installed provenance is unknown. Share official release/JAR selection validation between metadata lookup and acquisition. New permission-checked/audited GET reads metadata only on a blocking worker. Updating remains user-selected via existing operation/progress/download validation; reload supported formats and installed version on success. Preserve ready conversion controls if an update attempt fails. No automatic update, tests, or release workflow changes.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.206 only.
**Commit:** `P12.206: add optional chunker update checks`

**Manual acceptance:** Rebuild/restart app and agent. Convert World preflight has plain conversion guidance without an orange rail. Check for updates displays latest/current status; lookup failure does not prevent conversion with the installed version. Choose Update Chunker if offered, observe acquisition progress, then confirm installed version/formats refresh. No converter update occurs from checking alone. Svelte check passed with zero errors and ten existing warnings; Rust formatting and production Clippy passed with the existing unused auth helper warning. No tests or live downloads run.

### P12.207 — Remove decorative Manage Servers icons

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/ManageSheet.svelte`, this plan.
**What:** Remove the decorative server glyph and its boxed surface from each Manage Servers row, including unused styling. Server names and paths now begin at the existing row inset; preserve badges, activation and menu actions.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.207 only.
**Commit:** `P12.207: remove decorative server row icons`

**Manual acceptance:** Open Manage Servers: every row starts with the server name, without a left icon or reserved icon gap. No tests added or run.

### P12.208 — Open server actions from the whole row

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/ManageSheet.svelte`, this plan.
**What:** Replace per-server Set Active and three-dot buttons with a full-row button and trailing chevron. Use the existing shared Menu, Components selection treatment, and Set Active/Edit/Remove labels. Keep removal confirmation and editor behavior; disable Set Active for the already active server or missing control permission. Keyboard activation anchors the menu to the row.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.208 only.
**Commit:** `P12.208: open server actions from row clicks`

**Manual acceptance:** Manage Servers has a chevron instead of action buttons on each row. Clicking or keyboard-activating the row opens the Components-style three-action menu; Set Active updates the selected server, Edit opens its editor, Remove requires existing confirmation. Escape/click-away dismisses the menu. No tests added or run.

### P12.209 — Expand activation progress and simplify elapsed text

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Remove the Last progress age from activation display and replace its 520px cap with full available width. Preserve elapsed time, percentage, byte counts and lost-contact reporting. Display-only change; no activation logic, backend changes, restarts or live operations performed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.209 only.
**Commit:** `P12.209: expand world activation progress display`

**Manual acceptance:** Activation progress spans the world section width and shows elapsed time without Last progress text. Lost-agent-contact reporting remains available. No tests added or run.

### P12.210 — Fill stretched world slot cards with selection outline

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldSlotCard.svelte`, this plan.
**What:** Make the inner slot wrapper fill the grid-stretched Card. The selected border and background now cover the entire card height, which follows the tallest card in each grid row even when a slot has fewer metadata lines.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.210 only.
**Commit:** `P12.210: fill world slot selection cards`

**Manual acceptance:** Select slots with and without seed/profile details in the same row. Their selected border should run around the full, equal-height card including the blank space below shorter metadata. Svelte check passed; no tests added or run.

### P12.211 — Keep long active world names inside Overview cards

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/home/HomeSection.svelte`, `clients/desktop-web/src/lib/sections/home/ActiveWorldCard.svelte`, this plan.
**What:** Allow the Activity grid tracks and Active World column to shrink below their contents' intrinsic width. Let the world title metadata column take only available space and clip its existing single-line ellipsis inside the card. Long slot names no longer push into or overlap Chat.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.211 only.
**Commit:** `P12.211: contain long active world names`

**Manual acceptance:** Open Overview with an unusually long active-world slot name. The title should truncate within Active World and Chat should keep its own column with no overlap. Svelte check passed; no tests added or run.

### P12.212 — Freeze activation elapsed time at completion

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Stop the one-second elapsed-time clock as soon as the agent reports succeeded, failed, or cancelled. Keep the terminal progress and elapsed value steady while worlds/backups refresh; existing cleanup then closes the progress display.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.212 only.
**Commit:** `P12.212: freeze world activation elapsed timer`

**Manual acceptance:** Activate a world. On terminal status, elapsed time must stop changing immediately, including while the refreshed world list is loading. No tests added or run.

### P12.213 — Keep sidebar gameplay defaults in the active world profile

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** Read sidebar Difficulty and Gamemode from the active slot's `WorldProfile` and save changes back through its profile endpoint. Refresh the sidebar after profile saves, world activation, and world creation. Running Bedrock servers apply these fields with runtime commands and report pending restart if a command cannot be sent. Java uses the shared profile application path for Paper and all other Java flavors.
**Verify:** `npm --prefix clients/desktop-web run check` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.213 only.
**Commit:** `P12.213: sync sidebar world gameplay settings`

**Manual acceptance:** On Bedrock and Java servers, confirm the sidebar and active slot's World Settings show the same difficulty and default game mode. Change each value from either surface and confirm the other reflects it. Check Creative confirmation behavior on Bedrock and Java; confirm Paper, Fabric, Forge, and NeoForge share the Java behavior. No tests added or run.

### P12.214 — Keep latest Java selection stable through server creation

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, this plan.
**What:** Pin the resolved latest Java version when the wizard first asks which runtime to use. This keeps the selected Java runtime associated with the same release through the World step and Create action, so the wizard doesn't ask twice. Changing the selected version or Java flavor still invalidates the selection.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.214 only.
**Commit:** `P12.214: keep latest java selection through creation`

**Manual acceptance:** Create a fresh Java server with Download latest selected. Choose Java at the first prompt; the final Create Server action should not prompt again. If you go back and change Minecraft version or Java flavor, the runtime picker should appear again before proceeding. No tests added or run.

### P12.215 — Apply Bedrock default game mode to online players

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/worlds.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** When the active, running Bedrock world's default game mode changes, keep applying `defaultgamemode` for the saved world default and also run `gamemode <mode> @a` so currently connected players switch immediately. Difficulty continues to apply through Bedrock's live difficulty command. Keep Creative's existing achievements confirmation.
**Verify:** `cargo fmt --all` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.215 only.
**Commit:** `P12.215: apply bedrock gamemode to online players`

**Manual acceptance:** With multiple players online, change the active Bedrock world's default game mode from the sidebar or World Settings. Every connected player should switch immediately; new players should also receive the saved default. Confirm Bedrock Creative still requires the existing achievement warning. No tests added or run.

### P12.216 — Replace sidebar whitelist with Enforce Gamemode

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/sections/settings/SettingsSection.svelte`, `crates/msc-domain/src/settings_schema.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** Replace the sidebar Whitelist control with Enforce Gamemode. Read and save the server-wide `force-gamemode` property through `/v1/settings`, preserve its existing confirmation when enabled, and refresh the sidebar when Server Settings changes it. Mark this property restart-required for Java and Bedrock because the running server reads it at startup; explain that it enforces the server default when players join.
**Verify:** `npm --prefix clients/desktop-web run check` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.216 only.
**Commit:** `P12.216: replace sidebar whitelist with enforce gamemode`

**Manual acceptance:** Confirm the sidebar shows Force Gamemode's saved value on Java and Bedrock, including values changed in Server Settings. Enable it and accept the server-wide confirmation; the setting should persist and indicate restart when the server is running. After restart, players joining should receive the server default. Disabling should persist without confirmation. No tests added or run.

### P17.28 — Resolve the CLI socket from the installed agent service

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/cli/transport.rs`, `crates/msc-agent/src/cli/service.rs`, `docs/msc2/clients/phase17-cli.md`, this plan.
**What:** On macOS and Linux, locate the local CLI socket using an explicit process `MSC2_DATA_DIR` when set, otherwise read `MSC2_DATA_DIR` from the installed agent service definition, and use the existing platform default when the service has no override. This fixes the macOS headless installer path mismatch and also supports Linux custom data roots. Windows uses a fixed named pipe and is unaffected. Add focused path-precedence tests without running them.
**Verify:** `cargo check -p msc-agent`
**Batch:** P17.28 only.
**Commit:** `P17.28: resolve cli socket from installed service`

**Essential tests:** Two pure path-selection cases protect the concrete CLI authorization failure caused by a service/CLI data-directory mismatch and retain the documented explicit environment override. They use fixed paths, do not touch the host, and should complete in under one second after compilation. Tests were not run.

**Manual acceptance:** On macOS, run `msc capabilities` without setting `MSC2_DATA_DIR`; confirm it uses the installed service path. On Linux, repeat with the default installer path and with an explicitly configured service data root. Confirm an explicit shell `MSC2_DATA_DIR` still takes precedence. Windows needs no path-specific change because its CLI connects to the fixed local named pipe.

### P12.217 — Remove Enforce Gamemode helper text

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, this plan.
**What:** Remove the explanatory sentence beneath the sidebar Enforce Gamemode toggle.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.217 only.
**Commit:** `P12.217: remove enforce gamemode helper text`

### P12.218 — Allow older Java runtimes in first-run setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src-tauri/tauri.conf.json`, this plan.
**What:** Let first-run setup select Java 8 or later instead of incorrectly requiring Java 21 before a Minecraft version is known. Keep version-specific compatibility checks at server creation, and set the default Tauri window to 1740 × 1080 logical pixels to match Cameron's current window.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.218 only.
**Commit:** `P12.218: allow older java runtimes during setup`

### P12.219 — Match the default window to the resized app

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src-tauri/tauri.conf.json`, this plan.
**What:** Set the initial Tauri content area to 1600 × 900 logical pixels to match Cameron's manually resized app window.
**Verify:** Rebuild and launch the desktop app; confirm the initial window opens at the resized dimensions instead of filling the display.
**Batch:** P12.219 only.
**Commit:** `P12.219: match default window to resized app`

### P12.220 — Remove Tailscale from first-run setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, this plan.
**What:** Remove the Tailscale page from first-run setup and remove its mention from the intro page. Keep the remaining setup steps, optional skips, and completion navigation in order.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.220 only.
**Commit:** `P12.220: remove tailscale from first-run setup`

### P12.221 — Correct managed helper updates and startup recovery

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/geyser.rs`, `crates/msc-application/tests/geyser.rs`, desktop Components, App, FirstStartSheet, StartupFailurePanel, StartupFailureSheet, this plan.
**What:** Compare installed helper checksums against official releases; preserve disabled paths on update; restrict managed helper menus to plugins. Restart running servers during startup recovery, label plugin failures accurately, retain helper diagnosis, use official updates for helper repair, and surface soft helper failures after first-start completion. Add one essential controlled regression for disabled Floodgate replacement followed by a current suffixed snapshot: protects repeated-download and accidental-enable failures absent from existing coverage. Uses a fake provider and unique temporary directory, no network/timing assumptions; expected runtime under one second. Tests added but not run.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-application -- -D warnings && cargo check -p msc-application --test geyser && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build`
**Batch:** P12.221 only.
**Commit:** `P12.221: correct managed helper updates and startup recovery`


### P12.222 — Select first-world packs during server creation

**Status:** Implemented; awaiting Cameron's verification.
**Files:** Desktop wizard WorldStep, ConfirmStep and model; shared WorldPackBrowserSheet; Java datapack route, DTO and archive installer; world pack archive preparation/activation; world activation regression; API contract/generated types; this plan.
**What:** Add a Packs disclosure after Gameplay Rules, with edition-specific browse/import choices and selected pack rows matching the Add-ons step. Reuse the Worlds pack browser in staging mode, resolving Java loader version IDs to Minecraft versions for datapack filtering. Retain selection across wizard navigation, support removal and show the staged count on Confirm. Install packs into the new active world after creation and any backup import/activation; report individual failures and skip pack installation if the selected backup fails. Add local Java ZIP import through the existing bounded staging protocol and the shared archive validation/backup/profile rollback path. Record imported Java compatibility as unknown, with no invented catalog identity. Preserve Bedrock linked pack behavior. Refresh generated active Java worlds before archive changes and reapply the modified world, so packs selected for an imported backup also reach its live folder. Keep pre-generation packs in a separate `packs.zip`, install only validated pack paths during fresh-world activation, and preserve seed/first-generation settings without creating a fake saved world. Add one essential controlled regression covering local Java import, Java/Bedrock pre-generation activation twice, retained seed and refusal of non-pack files. Uses tiny local ZIPs and unique temporary directories with cleanup, no network or timing assumptions; expected under one second after compilation. Test compiled but not run; Cameron verifies wizard integration manually.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && cargo check -p msc-application --test world_activation && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build`
**Batch:** P12.222 only.
**Commit:** `P12.222: select first-world packs during server creation`

**Checks:** Formatting, ordinary Clippy, regression compilation, Svelte check and production frontend build passed. Svelte reports eleven existing warnings. Clippy reports the existing unused `auth::forbidden` function; strict `-D warnings` fails on that unrelated warning. No release workflow or test suite was run.

**Manual acceptance:** Rebuild/restart the app and agent. On a Java server, expand Packs below Gameplay Rules, browse datapacks filtered to the configured Minecraft version, add a catalog release and import a local ZIP. Confirm their rows show titles/descriptions/icons where available and Remove works; navigate Back/Continue and confirm the selection persists. Create the server and verify the selected packs in its Worlds tab and after first start. Repeat on Bedrock with behavior/resource filters and a linked `.mcaddon`; inspect the resulting behavior/resource records. Try a backup world and confirm packs target the imported active slot. Cancel creation and confirm no existing world received the staged packs. Local Java imports are validated as datapack archives but their Minecraft compatibility remains unverified.


### P12.223 — Include first-world packs in the onboarding tour

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `content/guides/onboarding.json`, `content/guides/onboarding-source-map.json`, desktop tour anchors/overlay, wizard WorldStep, this plan.
**What:** After Gameplay Rules, spotlight Packs and ask the user to expand it. Advance to its overview on the section click, then wait for Okay before the world review. Explain Java datapacks, Bedrock behavior/resource packs, browsing/importing, deferred installation and the option to add packs later in Worlds. Reuse the existing disclosure action and overview presentation; retain the Packs layout as Cameron directed. Renumber downstream guide steps and map the new first-world cards to their source section. No tests added or run for this small tour wiring change.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.223 only.
**Commit:** `P12.223: include world packs in onboarding tour`

**Manual acceptance:** Restart the tour on Java and Bedrock. After Gameplay Rules → Okay, confirm Packs is highlighted with an expansion prompt; expand it and confirm its overview appears with Okay. Confirm Okay advances to world review, without requiring a pack selection.


### P12.224 — Configure CurseForge without leaving pack browsing

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, this plan.
**What:** Turn the missing-key API error into a clickable prompt that opens a small API key sheet. Provide a masked key field, the owner-requested CurseForge console link through the native external opener, and Save/Cancel. Save through the existing agent settings endpoint and retry the current search without closing the browser or creation wizard. Cancel and Escape retain the browser, query, filters and wizard draft; clear key input on dismissal/success. Missing credentials no longer show the misleading no-results/search-term advice. No tests added or run for this existing API/UI integration.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.224 only.
**Commit:** `P12.224: configure curseforge key from pack browser`

**Manual acceptance:** With no CurseForge key, open Bedrock Browse Packs during creation and click the missing-key message. Confirm the console link opens externally, Cancel/Escape return to browsing without losing the draft, and Save stores the key and retries the current search. Saving failures stay in the key sheet. Repeat from the Worlds tab. The API key remains saved for the agent as in Settings; it is not read back into the field.


### P12.225 — Remove duplicate world review tour instruction

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/TourOverlay.svelte`, this plan.
**What:** Remove the hardcoded world-review hint that repeats the guide's body. Show the instruction once, retaining Okay and Continue behavior. No tests added or run for this copy removal.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.225 only.
**Commit:** `P12.225: remove duplicate world review tour instruction`

**Manual acceptance:** Restart the tour and reach Nice, Your World Is Configured. Confirm the review instruction appears once and Okay still reveals the world page for review and Continue.

### P12.226 — Show startup failure explanations once

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/StartupFailurePanel.svelte`, this plan.
**What:** Show each diagnosed failure explanation in its finding only, omitting the duplicate heading summary. For load failures with a supplied explanation, use that explanation directly and normalize its final period. Retain fallback summaries when no diagnosis exists and all repair/restart actions. No tests added or run for this copy cleanup.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.226 only.
**Commit:** `P12.226: remove repeated startup failure explanations`

**Manual acceptance:** Start Paper with the incompatible Geyser build. Confirm its explanation appears once, ends with one period, and the existing actions remain available. The same rendering applies to Floodgate findings.

### P12.227 — Exclude Tauri build output from Vite watching

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/vite.config.ts`, this plan.
**What:** Exclude `src-tauri/target` from Vite's recursive file watcher. Tauri compiles Rust dependencies into this directory while Vite watches the frontend root; on Windows, watching a locked proc-macro DLL fails with `EBUSY`. This generated directory is not frontend source on any platform. No tests added or run.
**Verify:** From `clients/desktop-web`, run `npx tauri dev` and confirm Vite starts without an `EBUSY` watcher error and the app window opens.
**Batch:** P12.227 only.
**Commit:** `P12.227: exclude tauri target from vite watcher`

### P12.228 — Keep Windows agent installation responsive and elevate registration

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `windows_service.rs`, `windows_service.ps1`, `windows_service_prompt.cs`, `crates/msc-platform-windows/src/service.rs`, this plan.
**What:** Run service actions and status checks on blocking workers so OS prompts and subprocess waits cannot freeze the desktop event loop. Replace the piped PowerShell credential prompt with an explicitly displayed native Windows credential dialog owned by the app. Ask for UAC approval for registration/removal; retain the original installing account even when a different administrator approves UAC. Collect its password inside the elevated helper, preserve the existing install request/environment through the agent CLI, grant service-logon permission and query/start/stop access to this service, and wait for startup. Routine start/stop stay unelevated under D-025. Return cancellation and diagnostic errors through a temporary result file containing no credentials, then remove it. Hide helper consoles and redact credentials from service-controller failures, including echoed diagnostics. macOS/Linux keep their existing platform actions with the new background dispatch.
**Verify:** From `clients/desktop-web`, run `npx tauri dev`; follow the manual acceptance below.
**Batch:** P12.228 only.
**Commit:** `P12.228: keep windows agent installation responsive`

**Checks:** Rust formatting, ordinary Clippy for the desktop and Windows platform crate, PowerShell parsing, and C# native-helper compilation passed. Strict Clippy is blocked by existing warnings: the unused infrastructure Bedrock socket-mode constant, unused desktop update helper and Windows installing-user return. The password-redaction regression was compiled with the platform tests; no tests were run. No release workflow changed or run.
**Essential regression:** A service-registration failure previously included the password-bearing command arguments in its error. The new controlled regression supplies command arguments and fake stdout/stderr containing a password, then checks that the password is absent and the useful error remains. It performs no OS calls, uses no timing/environment assumptions and should run in under one millisecond. It protects secret disclosure rather than incidental error wording.
**Manual acceptance:** Restart the development session. Choose Install, cancel UAC, and confirm the app reports cancellation and responds normally. Retry, approve UAC, cancel the native password dialog, and confirm the same. Retry with the installing account's Windows password (not its Hello PIN); confirm the agent reaches Running and connects. Stop/start the installed agent and confirm no further UAC or password prompt. Repair should use UAC and the credential dialog again. Check a failed credential/start attempt reports its error without disclosing the password. Live Windows acceptance and macOS/Linux physical checks remain Cameron's verification.

### P12.229 — Authenticate the Windows desktop with its running local agent

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, this plan.
**What:** Enable the existing host-local desktop pairing flow on Windows as well as Linux. The local bootstrap command previously returned unavailable on Windows even after successful service installation. Reuse the installing-account pairing CLI, redeem its one-use code through the loopback API, and store the resulting desktop credential in the native credential store. Preserve cached-credential probing and the agent identity check. Hide the Windows pairing subprocess console; codes and bearer tokens remain inside the native backend. No authentication bypass, agent-service change or tests added.
**Verify:** From `clients/desktop-web`, run `npx tauri dev`; confirm the already running Windows agent connects without another installation or repair.
**Batch:** P12.229 only.
**Commit:** `P12.229: authenticate local windows desktop agent`

**Diagnosis:** The installed Windows service was Running under Cameron's account and `/v1/healthz` returned HTTP 204. The desktop bootstrap had only macOS/Linux branches and returned unavailable on Windows; the setup screen replaced that failure with its generic reach/authenticate message.
**Checks:** Desktop Rust formatting and ordinary Clippy passed, with the same three existing Windows/infrastructure warnings recorded in P12.228. No tests or pairing commands were run by the agent.
**Manual acceptance:** Restart the desktop development session and confirm the existing running local agent connects. Restart again to confirm the stored credential is reused. If Repair is subsequently used, confirm its automatic connection retry also authenticates. Live pairing/credential verification remains Cameron's check; no tests were run.

### P18.10w — Expose depth slicing in every 2D dimension

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, this plan.
**What:** Extend the existing Depth Y control to every dimension in 2D. Use the viewer's manifest-backed height range (including its supported lower slice limit), retaining the Nether's Y126 roof limit and Y83 default when entering from above the roof. Other dimensions enter at full height; lowering Y cuts away terrain above that level. Hold camera focus at the selected height so surface tracking cannot lift it out of the cave; changing depth stops Follow. 3D, Fly, player focus and Home restore unsliced terrain and normal height tracking. Preserve the existing control styling under the required `antiAIslop.md` design law. No tests added or run for this narrow use of the existing viewer slice API.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P18.10w only.
**Commit:** `P18.10w: enable depth slicing in all 2d dimensions`

**Checks:** Svelte check passed with zero errors and eleven existing warnings outside the map component. No Rust files changed, so Rust formatting/Clippy are not relevant to this step. No test suites or release workflows run.

**Manual acceptance:** Reopen the map, select 2D in the Overworld and lower Depth Y below the surface, including negative Y. Confirm saved caves appear, panning/zooming retain the selected focus height, and 3D/Fly restore full terrain. Repeat in the End and a saved custom dimension; confirm each range follows its terrain manifest. Check Nether 2D still opens at Y83 from above the roof and stops at Y126. Follow a player, then change depth and confirm Follow stops. Home should restore the unsliced spawn view. Cave visibility depends on the saved geometry present in the map; this frontend change does not generate missing chunks.
