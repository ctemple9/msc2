# Phase 24 acceptance record

Implementation authorization: Cameron, 2026-11-05. No feature acceptance yet.
Contract: [integration specification](phase24-contract.md).

| Scenario | Owner evidence | Result |
|---|---|---|
| Modrinth/CurseForge search, details, pinned files, install and updates | pending | unverified |
| Both creation mismatch choices; staged-content conflicts | pending | unverified |
| Full client retention and separate audience editing | pending | unverified |
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
