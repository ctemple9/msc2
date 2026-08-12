See `../README.md` (§ `configs/`). Populated by P5.3 with one real, sanitized
`server_config_swift.json` from Cameron's own MSC 1 install.

`tools/phase5/real-corpus-check.py` (P5.2) is the dependency-free gate that
checks this directory before P5.4 onward may translate against it. In
inventory mode it requires, directly in this directory:

- At least one `*.json` config file (two preferred, from distinct schema
  eras — see the era gap noted below).
- A `manifest.json` alongside them with a `files` entry per config file
  recording that file's `era` and `sanitized` description.
- No two config files sharing a SHA-256 hash — a duplicate isn't a second
  sample.
- `$MSC2_PHASE5_TRANSFER_PACKAGE` naming a real, existing `.msctransfer`
  file (supplied through the environment, per `phase5-scope.md`'s "Evidence
  required" section — never committed here, since it carries real world
  data).

## What's here

| File | Source | Notes |
|---|---|---|
| `server-config-2026-08-11.json` | `~/Library/Application Support/MinecraftServerController/server_config_swift.json`, Cameron's live macOS MSC 1 install, `config_version: 1`, collected 2026-08-11 | Sanitized: local account name replaced with `msc1-owner` in every absolute path. No plaintext secrets were present to redact — this install's Remote API token and Xbox Broadcast password already live in Keychain, not this JSON. |

**Real `.msctransfer` package** (supplied via `$MSC2_PHASE5_TRANSFER_PACKAGE`,
not committed — carries real world data):

| Property | Value |
|---|---|
| Source | MSC 1's own Export Servers function, run by Cameron, 2026-08-11 |
| Format version | 2 |
| App config version | 1 |
| Size | 629,955,199 bytes |
| SHA-256 | `ea6dfe757c1d8cf3bcb20b348da58fbf32fee160706c40f898df07ef02562845` |
| Servers bundled | 2 (`campack` — Fabric 1.20.1 — and `Paper`) |

## Known gap: only one schema era

P5.3's original evidence bar called for **two** `server_config_swift.json`
files from distinct schema eras, so defaulting/rename behavior would be
exercised against more than one snapshot. After checking this Mac's other
Application Support copies, local Time Machine snapshots, MSC 1's own git
history (the config path is gitignored there, correctly, since it's runtime
data), and iCloud, Cameron confirmed no second-era config exists anywhere.
He approved relaxing the bar to one real config rather than inventing a
second — recorded in `docs/msc2/config-migration/phase5-scope.md`'s
"Evidence required" section. Era-diversity coverage for the decode-time
defaulting/rename/duplicate-ID/duplicate-path behavior a second era would
have exercised is carried instead by P5.4/P5.5's dedicated characterization
fixtures (extracted from MSC 1's own test assertions), not by this corpus —
this corpus's remaining job is proving the Rust reader actually parses and
round-trips a real production file (P5.24/P5.25), which one real file still
does.

The checker's own passing and deliberately-broken self-test cases live under
`tools/phase5/fixtures/` instead, precisely so nothing invented ends up here
standing in for the real thing.

## P5.25 — real corpus exercise results

Run 2026-08-12 against the evidence above, via
`tools/phase5/real-corpus-check.py --exercise --configs-dir corpus/configs
--transfer-package "$MSC2_PHASE5_TRANSFER_PACKAGE" --require-configs 1
--require-transfer` (see "Known require-configs mismatch" below for why
`--require-configs 1`, not the `2` the P5.25/P5.26 step text's own Verify
line names).

| Evidence | Check | Result |
|---|---|---|
| `server-config-2026-08-11.json` | `load_app_config` → `save_app_config` → `load_app_config`, decodes equal | `ok server-config-2026-08-11.json` |
| `server-config-2026-08-11.json` | source bytes unchanged after the run | SHA-256 `b836862eaf77a4a8e31dec4e8e20ac00bc7b92bc10ff514264654c1a58e5f365` before and after — unchanged |
| `MinecraftServers-2026-08-11.msctransfer.msctransfer` | `inspect_transfer_package` + `apply_transfer_import` into a fresh temporary root; at least one server and its world/config payload arrive | `ok` — both bundled servers (`campack`, `Paper`) arrived with non-empty destination directories |
| `MinecraftServers-2026-08-11.msctransfer.msctransfer` | package hash unchanged after inspect+apply | SHA-256 `ea6dfe757c1d8cf3bcb20b348da58fbf32fee160706c40f898df07ef02562845` before and after — unchanged |

Exit code 0. Full checker output (including the two `cargo test` runs) is in
the P5.25 commit's own `rolling-plan.md` entry, not reproduced here.

### Known require-configs mismatch

The Verify lines written into `rolling-plan.md` for both P5.25 and P5.26
pass `--require-configs 2` to exercise mode. That predates P5.3's discovery
that only one real config era survives anywhere on Cameron's machines —
inventory mode's own default (and its text above) was updated to require
one, but the two `--require-configs 2` call sites in the plan's Verify text
were not. Run literally, `--require-configs 2` fails immediately
(`found 1 config file(s), need at least 2`) without ever reaching the Rust
readers — not evidence of a bug in the checker or the readers, just a stale
argument. This step ran with `--require-configs 1` instead, matching the
evidence bar P5.3 already got Cameron's approval to relax. Flagged for
Cameron to resolve in the plan text itself (P5.26 inherits the same
mismatch); not changed here since editing another step's Verify line is
outside this step's own scope.
