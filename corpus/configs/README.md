See `../README.md` (§ `configs/`) — empty, needs real `server_config_swift.json` files from Cameron's own MSC 1 installs.

`tools/phase5/real-corpus-check.py` (P5.2) is the dependency-free gate that
checks this directory before P5.4 onward may translate against it. In
inventory mode it requires, directly in this directory:

- At least two `*.json` config files, from distinct schema eras.
- A `manifest.json` alongside them with a `files` entry per config file
  recording that file's `era` and `sanitized` description.
- No two config files sharing a SHA-256 hash — a duplicate isn't a second
  sample.
- `$MSC2_PHASE5_TRANSFER_PACKAGE` naming a real, existing `.msctransfer`
  file (supplied through the environment, per `phase5-scope.md`'s "Evidence
  required" section — never committed here, since it carries real world
  data).

**This directory itself stays empty until P5.3 supplies real MSC 1
evidence.** The checker's own passing and deliberately-broken self-test
cases live under `tools/phase5/fixtures/` instead, precisely so nothing
invented ends up here standing in for the real thing.
