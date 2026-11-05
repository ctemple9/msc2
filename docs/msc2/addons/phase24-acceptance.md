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
