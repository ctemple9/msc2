# Phase 20 native candidate and owner acceptance

Prepared October 17, 2026. **Owner accepted implementation and closed P20.1–P20.10
on October 17; independent gate review pending.**

Cameron requested Done statuses and archiving after the installation-status text
correction, saying he was okay with everything else. This is the current owner
disposition; the dated build records and earlier pending checklists below remain
an account of what was and was not individually observed. It is not evidence
that every physical case passed or that the P20.10 text shipped in P20.9.
Unmeasured comparisons and unresolved observations are retained in
[pending acceptance](../pending-acceptance.md). No further physical checks are
an active execution queue; the other agent still owns independent gate review.
This is the P20.7 handoff, not a release or a claim of measured improvement.
The P20.1 [baseline record](baseline.md) still contains no physical results.
Keep its procedure and record both installed-app and candidate observations there.

## Candidate identity and installation

Implementation source: `b1730415` (`P20.6: show native import progress across supported flows`).
P20.7 changes documentation only. The native candidate is Intel macOS x86_64,
built on macOS 15.7.8 (24G824), Intel Core i9-9880H, 16 GiB RAM. The checkout
volume had 62 GiB free before building; this is not an import-space measurement.
Its displayed desktop/agent version remains **0.1.30**. It is an unpublished
local build containing Phase 20; the published v0.1.30 does not contain Phase 20.
Use the source identity and candidate checksums, not the version label alone.
No first shipped native-transfer version exists yet. Capability negotiation
(`nativeTransfers`, protocol 1 and the individual advertised flags) determines
support; creation-time selection additionally requires `creationContext`.

Build from the repository root using the existing Tauri packaging configuration:

```sh
cd clients/desktop-web
npx tauri build --bundles app
```

This builds the frontend, release agent, pinned renderer, terrain exporter,
Intel Bedrock sidecar/appliance and native desktop bundle. It runs no test suite
and does not install, start, publish or update an agent. Only the macOS app
bundle is requested; this is not the nine-artifact release workflow.

Build output is `clients/desktop-web/src-tauri/target/release/bundle/macos/MSC 2.app`.
The candidate ZIP, checksums and completed build observations are recorded below.
Generated artifacts stay outside version control.

Before changing the installation, record the currently installed desktop and
agent and complete the **before** measurements in baseline.md. Retain a copy
of the current app outside `/Applications`, plus an ordinary backup of MSC data
before acceptance. Use disposable import destinations. Then:

1. Stop Minecraft servers in MSC and wait for Stopped; quit MSC.
2. Expand the candidate ZIP and copy `MSC 2.app` into `/Applications`, replacing
   the previous app. Do not run a second app copy alongside it.
3. Open `/Applications/MSC 2.app`, open Agent and choose **Repair service**
   (also described as **Repair agent** in release installation guidance).
   Complete the normal local administrator authorization. This installs the
   bundled candidate agent; replacing the window alone is insufficient.
4. Confirm the local service is running and connect to the intended host.
   Record the candidate ZIP checksum and original host identity with the run.
   The service identity check compares its installed binary path in a build
   directory keyed by the agent/renderer/exporter content digest. This separates
   same-version builds; successful repair alone is not physical proof of native
   transfer behavior. Confirm native progress and absence of the
   legacy chunk selector on a candidate-capable host during acceptance.
5. For rollback, stop servers, quit MSC, restore the retained app at the same
   location and repair its service again. Do not uninstall or remove data.
   Rollback restores program files; it does not undo imported data, so retain
   backups and use disposable destinations throughout.

The local bundle has an ad-hoc signature and the existing unsigned-beta
limitations; it has no Developer ID signature, is not notarized or accompanied by signed release-update metadata. No installed app,
service, settings, world, release tag or published asset was changed by P20.7.
For remote native comparisons, the destination must also have a matching agent
through its normal platform-local installation flow. This Intel app is not a
Linux/Windows/Apple Silicon agent package; those candidates are not built here.

## Measurements still required

Use the same approximately 8 GB world ZIP, representative pack, stopped raw
server folder/ZIP and available MSC transfer file as P20.1. Source paths and
hashes have not been supplied or selected for this handoff. No import, copy
benchmark, remote transfer or fault exercise was run. Use baseline.md's exact
copy/network reference commands, cache notes, stage timing procedure and MiB/s
formula. Record desktop, webview helper and agent memory separately; sampled
peaks are sampled observations, not guaranteed process high-water marks.

| Placement | Before + native world | Pack/provider stages | Server folder/ZIP/transfer | Copy/network reference + peak memory |
|---|---|---|---|---|
| Intel Mac, same volume | pending | pending | pending | pending |
| Intel Mac, cross volume | pending; second volume not selected | pending | pending | pending |
| Remote destination | pending; host/route not selected | pending | pending | pending |
| Windows | unavailable in this handoff | pending | pending | pending |
| Linux | unavailable in this handoff | pending | pending | pending |
| Apple Silicon | unavailable in this handoff | pending | pending | pending |

Record each observed run, including failures. Keep transfer time separate from
verification/inspection, extraction, provider downloads and installation. If a
stage has no measurable boundary, record combined/unknown. For folders, logical
source bytes and generated TAR wire bytes differ. Do not divide allocated disk
size by elapsed time and call it network throughput. The memory bound still
needs physical confirmation with the large input; a successful build cannot
establish it. No improvement factor can be calculated until comparable before,
after and reference measurements exist.

## Owner behavior and failure observations

All rows below are **pending physical verification**. Use selected disposable
sources and destinations; record host, exact source, action, visible result and
cleanup evidence. These are owner acceptance instructions, not an automatically
executed smoke suite. A focused automated command requires separate explicit
owner authorization under the testing policy.

| Case | Observation needed |
|---|---|
| World ZIP and active-world replacement | Native byte progress then separate validation/import; usable result, unchanged source; stopped-target checks and safety backup retained. |
| Creation wizard world, local JAR and Java/Bedrock packs | Native selection before a server exists; correct creation host; retained inputs consumed only by intended creation. |
| Modpack ZIP/mrpack, existing server and wizard | Inspection precedes install; provider downloads/manual waits reported separately; actual terminal operation succeeds. |
| Manual provider file/drop and thumbnail | Purpose/pending-file checks and image validation retained; wrong file refuses without false success. |
| Raw folder, ZIP and MSC transfer | Hidden/unknown supported files preserved; running-source refusal/stop confirmation; no intermediate desktop folder archive; merge/replaceAll review and recovery retained. |
| Export | World/server/client ZIP preparation distinct from download; verified final file; existing destination preserved on failure. |
| Cancel during copying/transfer | Source unchanged; prior active world intact; operation settles and owned partial staging is removed, or cleanup failure is reported on original host. |
| Close/Escape or switch host/server during picker/transfer/inspection | No late result populates another host/server; cleanup remains on original host; return there to see failure notices. |
| Truncated transport or failed digest | Incomplete receipt cannot be redeemed; no installed partial or overwritten world; cleanup confirmed or explicitly unconfirmed. Physical injection method/result pending. |
| Corrupt archive | Use a disposable malformed ZIP; format/CRC refusal after any completed receipt, no false import success, prior destination intact. This is distinct from transport truncation. |
| Wrong credential/host/source handle | Refused before use; host switching alone covers stale UI results, not every authenticated admission rejection. Focused protocol evidence remains pending separately. |
| Older agent | Candidate desktop paired to a retained older agent with capability omitted explains bounded native legacy fallback before transfer; real small import works; missing creationContext refuses creation picks before bytes. |
| Disconnect/restart or failed cleanup | Partial input never becomes a usable import; retry starts a fresh operation; no claim of resumable transfer; original-host cleanup status remains truthful. |

Do not treat source-level regression coverage or metadata compilation from
P20.2–P20.6 as executed evidence for these rows. No tests ran in P20.7.

## Remaining work and review handoff

The local file path makes a bounded ordinary copy/hash into private staging;
it does not currently use a copy-on-write clone. Cross-volume fallback therefore
still copies all bytes. Folder metadata walks and the importer's final copy
remain. Remote folders generate an uncompressed TAR stream, which trades CPU
and a desktop temporary archive for extra wire/destination bytes. Archive
validation/normalization, safe extraction, provider download and destination
installation still cost time. Prepared exports currently require a full digest
read before download. These are source-backed remaining passes, not measured
claims about which bottleneck dominates this machine.

After Cameron records step verification and the physical results above, the
other agent reviews the Phase 20 exit gate in msc2-port-plan.md against the exact
candidate/evidence. Review must assess the gate, including source preservation,
authentication, bounds, cancellation and compatibility, rather than merely
checking off steps. Phase 20 stays open until owner acceptance and independent
review. P20.7 does not start Phase 21 or close Phase 16/17 acceptance.

## Build evidence

Completed successfully on October 17, 2026:

- `npx tauri build --bundles app` in `clients/desktop-web` produced the native
  Intel app with the matching release agent and all expected helper resources.
- Root and desktop `cargo fmt ... -- --check` passed.
- `cargo build --locked -p msc-agent --bin msc` passed (the existing unused
  `auth::forbidden` warning remains).
- Agent Clippy passed with `-D warnings -A dead_code`; desktop Clippy passed
  with the existing `needless_borrow`, `needless_borrows_for_generic_args` and
  `needless_return` allowances. No new allowance or release gate was added.
- Frontend production build passed with existing unused CSS, mixed dynamic/static
  import and bundle-size warnings. Swift helper build passed with the existing
  non-Sendable relay capture warning. No tests were added or executed.
- Resource inspection confirmed `com.ctemple.msc2`, version 0.1.30, x86_64 main
  executable and both pinned appliance SHA-256 values. The packaged helper
  retains `com.apple.security.virtualization = true`.
- The initial outer app was unsigned. Applied `codesign --force --sign -` to
  the bundled agent and then the outer app, preserving the signed helper.
  `codesign --verify --deep --strict` then passed. This is ad-hoc local signing,
  not notarization or release signing. Packed with `ditto -c -k --sequesterRsrc
  --keepParent` to preserve the app bundle.

Candidate ZIP (relative to repository root):

`target/native-upload-candidate/msc2-0.1.30-p20.7-b1730415-macos-x86_64.zip`

Size: **30,925,018 bytes**. SHA-256:

```text
f995fbb85bb2b2c7c2d4d19d16acc06522f527e0df35e375e6e493bf67cb9719
```

The adjacent `target/native-upload-candidate/SHA256SUMS` records the ZIP digest.
Checksum verification passed; extracting the ZIP with `ditto -x -k` and running
`codesign --verify --deep --strict` on the extracted app also passed.
Verify the exact ZIP before installing:

```sh
cd /Users/camerontemple/msc2/target/native-upload-candidate
shasum -a 256 -c SHA256SUMS
```

Post-signing packaged desktop SHA-256:
`16bb02cb524b61f0aa5a88b1c8b55aa969d20630caf4d51db36fbb453ba3c15a`.
Post-signing packaged agent SHA-256:
`f6b50276492e2f6ed4515d88756a36a74b35f4db0f25032a8fac7e4f2019edba`.
These identify the generated candidate only; rebuilding or resigning can change
bytes. The ZIP is ignored/generated and remains uncommitted. No app launch,
service replacement, physical transfer or performance measurement was performed.

## Owner session observations — October 17

Cameron confirmed Repair service and local connection worked. Development-build
large-world transfer displayed about 7.7–9.5 MB/s. The optimized P20.7 candidate
later displayed 115.1 MB/s at 6.6 GB of 9.2 GB (70%); this is an intermediate
UI rate, not total elapsed import time or an old-release comparison. The world
attempt subsequently failed with “The selected source changed.” Content versus
metadata change has not been established; successful large-world import remains
pending. No complete peak-memory/copy-baseline measurement was supplied.

Complete-server acceptance then hit the 10 GiB source ceiling: the selected
`tectonic.zip` was 20,066,596,805 bytes. A large live-world subfolder hit the same
folder restriction, but that world subfolder is not a complete-server input.
P20.9 corrects the server-source bound and staging error details. Retest using
the complete stopped server folder or its complete ZIP. The source-change
failure and local HTTP-versus-direct-copy selection remain separate open findings.

P20.8 removes the native alert icon using an MSC confirmation sheet. Both that
visual correction and P20.9 require the newer candidate below, not the original
P20.7 ZIP. Neither correction closes physical acceptance or independent review.

### Updated optimized Intel candidate — P20.9

Contains the P20.8 icon-free folder confirmation and P20.9 large-server
admission correction. Source is P20.8 commit `13e47d3d` plus the changes committed
as `P20.9: admit large server sources with disk space checks`. Version remains
0.1.30; this is an unpublished local candidate, not a new official release.

ZIP: `target/native-upload-candidate/msc2-0.1.30-p20.9-macos-x86_64.zip` (30,906,297 bytes).
SHA-256: `dfdadfbf3d100fde5f2e771053b63a04e1189cd85a730bac740c272cee9bcb0d`.

Use the installation/rollback procedure above, including Repair service after
replacing the app. The old P20.7 ZIP remains unchanged and lacks these fixes.
Check the new ZIP from its directory with `shasum -a 256 -c P20.9-SHA256SUMS`.

Locked agent build, root/desktop formatting and agent/desktop Clippy passed
with the previously recorded allowances. The new focused admission regression
compiled as test metadata; no tests ran. The existing Tauri app build completed
with the existing frontend and Swift warnings. Applied ad-hoc signing to the
bundled agent and outer app, preserving the helper's entitlement;
`codesign --verify --deep --strict` passed. ZIP checksum and extracted signature
verification passed. No installation, live import or release publication ran.
Owner large-server retest remains pending, including actual extraction space.
