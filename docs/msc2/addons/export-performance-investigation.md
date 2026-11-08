# Local export performance investigation — 2026-11-07

Scope: investigate Cameron's existing ATM10 Lite export. Read-only source,
filesystem metadata and a three-second sample of the already-running agent.
No new export, test, benchmark, build, restart or cancellation was performed.
No production code was changed.

## Measured evidence

- Selected content: 1,558 files, 310,856,939 bytes (296.5 MiB).
- The retained publication contains 1,454 unique blobs (deduplication is active).
  Their write timestamps span 455.87 seconds, from 00:22:08 to 00:29:44 UTC
  on November 8 (November 7 locally). This establishes at least 7m36s of
  snapshot creation; it does not isolate copy, hashing and flush durations.
- 1,159 blobs are under 64 KiB. Median gap between blob write timestamps is
  48 ms, p95 1.174 s, largest 27.725 s. Larger gaps include large payloads.
  These measurements do not prove fsync alone dominates snapshot creation.
- The in-progress archive was created at 00:39:02 UTC. At 00:53:14 it was
  still being written, at 400,077,797 bytes: over 14m12s of archive writing.
  Earlier metadata showed 394,592,928 bytes at 00:53:01, confirming growth.
  It was working slowly rather than frozen during observation.
- The app config is 4,629,116 bytes on disk. The pack's content collection
  serializes to approximately 3.6 MB and contains thousands of map entries.
- Running-agent SHA256 matches target/debug/msc exactly:
  9366d2274d16bfd755fd163ba4af8d36bdc8a44c3598ab6ed1616c82d2273866.
  This is the staged development build; Cargo.toml has no dev optimization override.
- A three-second macOS process sample captured ZIP deflate execution and deep
  configuration cloning in status handling. CPU usage was over two cores at
  one observation. The sample captured only two stacks per sampled thread;
  it cannot assign reliable percentages to the individual bottlenecks.

Raw diagnostic sample remains outside the repository at
/tmp/msc-export-profile.txt. Config contents/credentials are not reproduced.

## Confirmed implementation costs

1. **Full config copies inside the archive's chunk loop.**
   content_archives::copy_entry invokes the cancellation callback before each
   128 KiB read, including EOF. The route callback invokes publication_bound,
   which calls app_config_servers -> AgentAppConfigStore::servers -> snapshot.
   snapshot clones the whole AppConfig, including the pack's large collection.
   This is an in-memory clone, not a JSON file read on each chunk.
   Calculating from this pack's recorded file sizes gives 5,227 callback checks
   for one copy, twice for ZIP plus .mrpack, plus 1,558 nested per-file checks:
   at least 12,012 potential full-config copies. This count is calculated,
   not a profiler allocation measurement. Ordinary status polling also clones
   this enlarged config, and appeared in the live process sample.

   Sources: crates/msc-agent/src/routes/components/content_archives.rs:78, :293;
   crates/msc-agent/src/routes/lifecycle.rs:1895, :3987;
   crates/msc-application/src/content_archives.rs:68;
   crates/msc-application/src/content_mrpack.rs (per-file loop).

2. **Repeated full-file verification and a disk flush per unique snapshot file.**
   Publish loads and verifies the review before freeze and again before registry
   commit. Each load recomputes the content fingerprint and rehashes prepared
   sources. Freeze also identifies each source, streams a hashed copy, identifies
   the copy again, and copy_exact calls sync_all for every unique blob. These
   checks overlap; the existing snapshot protocol must be preserved when reducing
   them. The observed long snapshot span is real, but no per-stage timers exist
   to attribute its 455.87 seconds precisely.

   Sources: crates/msc-agent/src/routes/components/content_publications.rs:231, :253;
   crates/msc-application/src/publication_review.rs (read_review);
   crates/msc-application/src/content_publications.rs:384;
   crates/msc-application/src/pack_retention.rs:39, :91.

3. **Compression of the same payload twice, in a development build.**
   ZIP entry options use Deflated for all files, including JAR/ZIP/image assets
   that are already compressed. With .mrpack checked, the same selected content
   is read, hashed and compressed again into the inner package. The outer
   .mrpack container entry is already Stored, so there is no extra compression
   of the entire completed .mrpack; the duplication is at payload level.
   The live sample caught deflate code. The unoptimized agent amplifies CPU
   overhead, but no optimized-build comparison was run, so its exact share is
   unknown. Merely changing the build would leave the algorithmic waste intact.

   Sources: crates/msc-application/src/content_archives.rs:23 and outer/nested loops;
   crates/msc-application/src/content_mrpack.rs (write and payload loop).

4. **Progress does not describe the active stage.**
   The sheet carries preparation's 100% into publish, where the backend does
   not report copy progress. ZIP creation already sends current/total bytes,
   but exportArchive consumes only statusLine. The bar is hidden once the
   selected publication changes from new. This explains the misleading 100%
   and subsequent absent ZIP percentage independently of actual runtime.

   Source: clients/desktop-web/src/lib/sections/components/ContentExportSheet.svelte
   (publish, exportArchive, progress rendering).

## Recommended correction order — not implemented in this investigation

1. Replace the per-chunk full-config clone with a narrow protected binding check
   or configuration generation token. Keep cheap cancellation/revocation checks
   responsive and validate binding at commit; do not remove security checks.
2. Consolidate copy and verification into streaming passes. Review per-file
   fsync/durability requirements before changing the publication commit protocol.
3. Use suitable compression for already-compressed assets and retain the approved
   ZIP-plus-optional-.mrpack layout. Do not silently remove either payload copy.
4. Give snapshot creation, ZIP writing and local transfer separate truthful
   progress, using byte totals where available and product-facing stage labels.
5. After the code correction, obtain an explicitly authorized timed verification
   on this pack. No speed estimate or improvement claim is justified yet.

The export path is local; no CurseForge/Modrinth acquisition is called here.

## Follow-up — Desktop zero-byte save (P24.40)

Read-only investigation of the owner's save at 01:18–01:20 UTC November 8.
The source ZIP was already complete at 546,473,206 bytes. The Desktop private
.msc-transfer partial file was still zero bytes 96 seconds after creation.
A two-second sample of the active agent directly captured
components::download_staged_bytes at components.rs:1623 inside SHA256 software
compression (703 samples in that path). The handler hashes the entire staged
file before sending HTTP headers/body, then seeks back and streams it. The native
client creates its private partial file before waiting for that response.
This explains both the zero-byte file and the 0 B saved display during hashing.

By the next observation the save had finished. The Desktop file
msc-ATM10-Lite-2026-11-08-r1-players.zip was 546,473,206 bytes, with last-write time
01:20:22 UTC. Its ZIP directory was readable: 1,575 entries, README.txt, the
nested .mrpack and expected content folders. README.txt names the missing
moreoverlays JAR. This inspected directory metadata/README, not a full ZIP CRC
or test-suite run. No file was changed, cancelled, copied or repaired manually.

Further correction should calculate and retain a verified archive digest during
creation, bind it to the immutable staged download, and avoid an opaque full-file
pre-transfer pass where safe. Preserve receiver-side verification and ownership/
revocation checks. If pre-transfer verification remains necessary, expose it as
a separate stage instead of displaying 0 B saved without explanation. The sample
is /tmp/msc-export-download-profile.txt; no production implementation changed.

## P24.41 implementation candidate

The correction removes full-config clones from archive chunk checks, removes one
full pre-freeze review scan and the unique-source pre-copy hash pass, uses fast
text compression and stores already compressed payloads. Source verification
while copying, destination readback, per-file durability and final review
validation remain. Development builds optimize sha1/sha2/flate2/zlib-rs only.
A streaming ZIP writer hashes the complete archive as it writes; the owner-bound
staged download reuses that digest, while the desktop still checks received
bytes before committing the save. Other staged download types retain their
original checksum path. Each active UI stage has its own percentage.

No timing run or new live export was performed. The earlier observed timings
are the baseline, not a claim for the new implementation. Optional .mrpack still
contains a second copy of the payload to preserve the owner's chosen ZIP layout.
