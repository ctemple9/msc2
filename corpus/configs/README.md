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
