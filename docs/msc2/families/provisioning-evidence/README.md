P7.28 — the one step in Phase 7 that uses the real internet. Every earlier
Phase 7 step (P7.13-P7.27) proved server provisioning against fake,
local-only providers precisely so that testing stays fast and never flaky.
This directory is the counterpart: one real create/start/verify-ready/stop
cycle per family, driven through nothing but `msc-agent`'s own CLI (the same
public surface iOS and any other client uses), against the live PaperMC,
Mojang, PurpurMC, FabricMC, NeoForge Maven, and Forge Maven endpoints — no
`MSC2_PROVIDER_*_BASE` override set anywhere in this run. Originally
captured 2026-08-19 (P7.28); **re-captured 2026-08-20 (P7.37)**, on the
exact-candidate commit, after P7.35 wired real publisher-checksum
enforcement into `jar_provider.rs` and P7.36 completed live startup
diagnostics — the P7.28-era files predate both and are superseded by
this re-capture, not kept alongside it.

One `<family>.json` per family, all six required (`vanilla`, `paper`,
`purpur`, `fabric`, `neoforge`, `forge`; the same six `tools/phase7/
provider-corpus-check.py`'s `FAMILIES` tuple already names). Every file
carries, at minimum: the resolved Minecraft (and, where applicable, loader)
version, the real download URL, a checksum with `matches_provider_published`
recorded (`true`/`false` where a provider actually publishes one to compare
against, `null` where it doesn't — Fabric and NeoForge/Forge's Maven publish
none), **`checksum_verification`** (P7.37: `"enforced_by_production"` when
that create ran through the real, unmodified `msc-agent` binary's own P7.35
checksum enforcement — a mismatch would have refused the download outright,
so the create succeeding at all is the evidence, independently corroborated
below by a fresh re-hash of the on-disk jar — or `"not_published"` when
this family's provider gives `jar_provider.rs` nothing to enforce), the real
launch argv the agent ran, whether the server reached
`Done (...)! For help, type "help"`, and how long create took
(`install_seconds`). `python3 tools/phase7/provider-corpus-check.py
--evidence` enforces this shape: all six families present, no more, no
fewer, every required field non-empty, `checksum_verification` consistent
with `checksum.matches_provider_published`, `reached_ready` literally
`true` for all six — a family that could not be started would fail this
checker rather than let the gate pass silently on five of six, per this
step's own "if a family genuinely cannot be provisioned today, stop and
report it" instruction.

## Result

All six families: created, started, reached a genuine `Done` ready line,
stopped, no server left running. Nothing failed; there is no "if a family
genuinely cannot be provisioned" report to make here.

## What this run found, not just confirmed

- **The Mojang EULA gate is real and MSC2 correctly does not bypass it.**
  Vanilla's first real boot attempt exited immediately with "You need to
  agree to the EULA" — `eula.txt` is written `eula=false` by MSC2 (matching
  every real Minecraft server's first-boot behavior) and needs a human to
  flip it before the server will run. Not a bug: MSC2 leaving that consent
  step to the operator, rather than silently agreeing on their behalf, is
  the correct behavior. (Paper, Purpur, and Fabric had their `eula.txt`
  pre-flipped by this run before their own first `start`, so this only
  surfaced once, on vanilla.)
- **Checksum shape genuinely differs per provider, live.** Mojang publishes
  SHA-1 (not SHA-256) per version in its piston-meta metadata; Paper's fill
  v3 API publishes SHA-256; Purpur's per-build API publishes MD5 only;
  Fabric's composed loader/installer/server-jar endpoint and NeoForge's/
  Forge's Maven publish no checksum for the jar/installer they serve at
  all. Every `checksum.algorithm` in this directory's evidence records what
  the real provider actually publishes, not a checksum type picked for
  consistency across files.
- **Forge/NeoForge's installer jar is deleted by MSC2 after a successful
  real install**, matching MSC 1's own behavior — so this evidence's Forge/
  NeoForge `checksum.value` is a fresh, independent re-download of the same
  URL `jar_provider.rs` composes, hashed separately, not a hash of the byte
  stream MSC2 itself consumed. Documented per-family rather than treated as
  a gap: the stronger evidence for those two families is the real,
  multi-hundred-file `libraries/` tree MSC2's own installer run produced
  (`installed_libraries_total_size_bytes`/`installed_libraries_file_count`
  in each file) plus each installer's own "installed successfully" log line
  and, for Forge, its self-verified per-file patch checksum.
- **Forge's `maven-metadata.xml` under-reporting the newest version (P7.3's
  own finding) still holds against live data today** — MSC2's real create
  flow resolved through `promotions_slim.json` as designed, landing on
  `65.1.0`, not whatever the metadata tag's `<latest>`/`<release>` claims.
- **Every family resolved to the same live Minecraft release, `26.2`**
  (Mojang's, Paper's, Purpur's, and Fabric's `stable`/`release` markers all
  agreed) — the `YY.n` versioning scheme P7.3 first observed in the raw
  corpus is confirmed live and unchanged one day later.
- **P7.37 re-capture: publisher checksum enforcement is real end-to-end,
  not just at the unit level.** Every Vanilla/Paper/Purpur create in this
  re-capture ran through the exact same `jar_provider.rs`/`download_staging
  ::stage_download` path P7.35 wired real enforcement into — a corrupted or
  substituted jar would have been refused before `eula.txt`/
  `server.properties`/`world_slots` were ever written, so each of these
  three creates succeeding *is* production-level proof its own checksum
  check accepted the real bytes. This re-capture also independently
  re-verified all three: Vanilla's on-disk jar hashes to the exact SHA-1
  Mojang's own per-version metadata publishes, Paper's to the exact SHA-256
  its builds response publishes (matching P7.28's original digest
  byte-for-byte — Paper build 112 hasn't moved), and Purpur's to the exact
  MD5 its `/v2/purpur/{version}/latest` endpoint publishes (Purpur's own
  build advanced from 2568/2620 to 2622 between captures — a live rolling
  build, not a pin, so a differing build/size here is expected). Fabric's
  and NeoForge's/Forge's digests (or lack of one) and resolved versions are
  otherwise unchanged from the original capture — Forge/NeoForge's
  installer-jar re-download hashed byte-for-byte identical to P7.28's own
  capture a day-plus earlier.

## iOS manual walkthrough

Not run as part of this step. `rolling-plan.md`'s own "iOS manual walkthrough
— deferred from P7.26, run alongside P7.28" section is a separate, live
pairing session with Cameron (device/simulator control this agent doesn't
have) — still open, tracked there, not here.
