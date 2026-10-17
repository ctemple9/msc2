# Storage acceptance — Phase 21

October 17, 2026. P21.1–P21.7 implemented; owner verification and the independent
feature-gate review remain open. Cameron authorized sequential implementation
with verification at the end. No step or physical case below is closed by a
successful build. This record accompanies the [contract](contract.md),
[screen sketches](screens.md) and [phase gate](../msc2-port-plan.md#phase-21--storage-inventory-and-management).

## Candidate

Implementation source: `c6025609` (`P21.7: add settings storage inventory and management`).
P21.8 adds documentation/capability accounting and a scoped UI completion guard:
a late cleanup response keeps its original result without clearing navigation
or a newer review opened while the request was in flight. Application version stays
0.1.30; this is an unpublished local Intel Mac candidate, not a release.

The normal packaging attempt first stopped at the pinned Vantage download
because the sandbox could not resolve its host. With network permission, pinned
renderer staging and Intel helper compilation succeeded; no release workflow was
run. The final agent includes the inspection-lease correction and is staged into
the newly built matching app before ad-hoc signing. The desktop executable is
also rebuilt with the final shared-library source and the default embedded
`custom-protocol` feature; original bundle metadata and signed helper
entitlements are retained. Candidate hashes follow below.
No installed application, running agent, service configuration or valued data
was changed to prepare it.

### Exact local artifact

ZIP: `target/storage-candidate/P21.8/msc2-0.1.30-p21.8-macos-x86_64.zip` (31,825,410 bytes).
SHA-256: `1369902ce2386ce93476caf7c21fac17e626f3594da83da27f02aee86d206d9c`.
Adjacent records: `P21.8-SHA256SUMS` (archive, signed executables and complete
helper/appliance resources) and `P21.8-artifact.json` (source-input and binary
hashes). The staged app is `target/storage-candidate/P21.8/MSC 2.app`.

Normal Tauri app packaging succeeded after the pinned-download permission retry.
Final locked optimized agent and desktop builds succeeded; the final native
executable and agent were assembled into that bundle. Ad-hoc signing, deep strict
signature verification, ZIP extraction, extracted-app signature verification and
all packaged-file hash comparisons passed. This is not Developer ID signing,
notarization or publication. The required locked debug agent build, agent/shared
library/native Clippy, formatting and Svelte type checks passed (ten existing
unused-style warnings). No tests were run.

## Install and first verification

1. Stop Minecraft through MSC and wait for **Stopped**. Quit MSC.
2. Extract the candidate ZIP to a temporary folder. Replace **MSC 2.app** in
   Applications using Finder. This candidate is ad-hoc signed, not notarized.
3. Open the candidate and use **Agent… → Repair** for This computer so the
   installed agent matches the new desktop API. Remote computers need a matching
   agent separately; this candidate does not update them automatically.
4. Open **MSC Settings → Manage Storage…**. The first view should show This
   computer plus saved remote computers, with dated summaries or Unknown.
   Opening it must not start scans or contact every saved host.
5. Select This computer, choose **Refresh**, inspect progress and cancellation,
   then inspect the category rows and server detail. Compare known small files
   and actual locations. Capture the overview, computer, server detail and a
   removal review as the visual evidence requested below.
6. Use only owner-selected disposable server/world/backup/transfer data for
   removal checks. Review names, computer, selected items and consequences;
   type the exact confirmation supplied by that review. Report per-item results
   and whether the filesystem agrees. Earlier removals stay removed if later
   work is cancelled or fails.

## Acceptance observations

All rows are **Pending Cameron**. No live destructive check or test suite was
executed during implementation.

| Case | Expected observation | Owner observation |
|---|---|---|
| Saved-computer overview | Cached date/partial status; one local row; no scan-all or network fan-out | Pending |
| Local and remote identity | A remote host keeps its own files, totals and capacities; changing the selected Minecraft server does not redirect a review/result | Pending |
| Complete known roots | Worlds, slots, backups, components, logs, agent, app, tools and cache have explained disjoint totals; parents/children are not counted twice | Pending |
| Custom server/data locations | Actual registered/custom locations are measured; paths outside verified ownership are excluded or protected | Pending |
| Multiple volumes | One capacity observation per physical volume; logical byte basis stays consistent; inaccessible capacity reads Unknown | Pending |
| Hard links, sparse files and clones | Hard links deduplicate known identities; allocation is separate/nullable; clone/sparse limits never promise space freed | Pending |
| Denied/unreadable roots | Total is a known lower bound with partial issues; unreadable item/allocation/capacity is not presented as zero | Pending |
| Large server | Progress remains visible and management stays usable; traversal respects entry/depth/identity/time bounds; cancellation ends at safe boundaries | Pending |
| Local agent offline/absent | Desktop measurement remains available independently; agent data is dated or unknown, not silently complete | Pending |
| Remote offline/reconnect | Historical results remain readable; explicit Refresh/status check contacts only that computer; failed status polling pauses; reconnect does not resubmit removal | Pending |
| Older agent | Updated inventory reports unsupported clearly; earlier map cleanup remains separately available on its labelled Settings host | Pending |
| Current app/agent/tools | Protected inventory; existing update/Agent/server-editor management locations shown; no generic deletion button | Pending |
| Active map/render/conversion | Live leases/process references protect work; abandoned copies can be reviewed; renderer source originals remain intact | Pending |
| Active upload/inspection/download | Existing transfer owner keeps active consumers protected; only expired/inactive owned entries can be reviewed | Pending |
| Obsolete agent builds | Hash/layout/current/service/process proof required; current and prior fallback retained; unknown legacy layouts/relative helper references protected; staging/repair admission prevents races | Pending |
| Backup batch | Same computer/category only; each selected ZIP/paired metadata identified; last verified backup for each server retained, including when earlier batch entries were removed | Pending |
| Saved world removal | Separate single-item review and typed identity confirmation; active slot refused; source archive/consequences visible | Pending |
| Whole-server removal | Separate single-item review; actual saved worlds/backups and directory consequences listed; running, unready or overlapping registered roots refused | Pending |
| Stale item/path/root | Changed fingerprint, registered path, root identity or item ownership causes retention/refusal; no raw-path fallback | Pending |
| Host/credential/permission change | Changed computer/credential cannot use another review; existing Admin/Worlds/Fleet rules remain authoritative; revocation stops later items | Pending |
| Sheet closure/navigation | Pending scan/cleanup stays on original computer; reopen shows its operation/result; no deletion is resubmitted | Pending |
| Cancellation/partial failure | Already removed items remain recorded; later items are not attempted; retained/failed/unconfirmed outcomes remain visible | Pending |
| Agent interruption/restart | Persisted in-progress item is unconfirmed; later items not attempted; operation reconciles without automatic deletion/resume | Pending |
| Space after removal | Logical bytes removed labelled separately; each affected volume sampled independently; absent/changed capacity stays Unknown and unrelated free-space changes are not attributed to MSC | Pending |
| Other supported platforms | Windows/Linux packaged discovery, allocation, leases, paths and operation outcomes require physical verification on those systems | Pending |

Use small controlled files first. For the large-world responsiveness observation,
use an owner-selected expendable copy. A snapshot/fingerprint is a changed-source
check, not an undo mechanism. Preview never stops a server or changes selection;
any required stop is explicit before another review.

## Visual evidence

| Screen/state | Evidence |
|---|---|
| Saved computers and dated/offline rows | Pending installed-candidate capture |
| Computer total, proportional category bar and volumes | Pending installed-candidate capture |
| Largest-first servers and grouped server detail | Pending installed-candidate capture |
| Cache/backup selection and separate world/server reviews | Pending installed-candidate capture |
| Partial, permission-denied, changed-source and result states | Pending installed-candidate capture |

The Browser skill runtime reported **No browser is available**, and its discovery
list was empty. No browser screenshots were produced or substituted with claimed
rendered evidence. Source inspection checked existing S0 tokens, neutral rows,
right-aligned sizes, a single proportional bar, restrained typography, no nested
cards, gradients or status dots. This does not establish visual acceptance.

## Reproducible non-test checks

From the repository root (these do not run a test suite):

```sh
cargo clippy -p msc-application -p msc-infrastructure --lib -- -D warnings
cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code
cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib -- -D warnings -A clippy::needless_borrow -A clippy::needless_borrows_for_generic_args -A clippy::needless_return
npm --prefix clients/desktop-web run check
cargo build --locked -p msc-agent --bin msc
```

Step-specific Verify lines remain in [the rolling plan](../rolling-plan.md).
No `cargo test`, Nextest, Vitest, Playwright suite, smoke suite or Xcode tests
were run. The existing agent binary unit-test target compiled as metadata;
broader integration-target checking encounters the already recorded
`cli_pairing`/`desktop_auth` route imports and `cli_service` argument mismatches.
No unrelated test repair or new CI/release gate was added.

A two-file in-memory slot-inspection regression checks read-only legacy handling
and refusal of a metadata ID pointing outside its directory (expected under
1 ms, no filesystem/network/timing dependency).

The existing six-byte native-transfer case also now checks that inspection
retains files/reservation and the owner reaper releases them after the lease
drops. It adds no fixture/network/timer dependence, expected under one second.

Essential regressions added during Phase 21 protect overlap/hard-link accounting,
blocked/link roots, custom-root attribution, host/schema cache binding, false
locality/double counting, late/wrong-host replies and Worlds/Fleet permission
boundaries. Controlled tiny inputs or pure metadata, no live network/timing;
expected under one second for filesystem fixtures and under 1 ms for pure cases.
The read-only slot regression compiled with `cargo check -p msc-infrastructure
--test world_store`. Compilation does not assert that those cases passed at runtime.

Shared storage/source-walker code type-checks against the installed Windows
MSVC Rust target through an isolated temporary crate. This is not a Windows
product build or physical acceptance. Linux target/physical evidence was not
available. The documented MSC 1 source checkout was missing at its stated path;
new direct Swift parity audit is not claimed.

After Cameron records the observations, the other agent reviews the feature
gate. This implementing agent does not perform that review or advance Phase 21.
