# Symbol ledger format

Defines the columns of `docs/msc2/audit/msc2-symbol-ledger.csv`, per
`docs/msc2/rolling-plan.md` P0.25. The ledger is the thing
`msc2-port-plan.md` §1 says doesn't exist yet: a **symbol-level** record of
which behaviors inside Mixed and UI-bucket files belong to the agent
(Rust-bound) versus the replacement client, decided with the deletion test
in that same section — not a file-level bucket like the two audit CSVs.

## Columns

| Column | Meaning |
|---|---|
| `file` | Filename as it appears in `msc2-file-inventory-b.csv` (no path — matches the inventory's own convention; MSC 1 has no filename collisions). |
| `bucket` | Which pass found this file: `mixed` (P0.26, all 59 `bucket=mixed` inventory rows) or `ui-flagged` (P0.27, the UI-bucket files P0.25's scanner flags at the chosen hit threshold). |
| `symbol` | The Swift symbol this row is about — a function/method name, or a short description for a row that isn't one specific symbol (e.g. `(none)` for a file with nothing to extract). |
| `kind` | `parser` (turns text/bytes into structured data), `policy` (a decision/validation/classification rule), `workflow` (an I/O-driving sequence — process launch, file staging, multi-step orchestration), or `none` (file has no agent-owned symbols). |
| `disposition` | `agent` (belongs in the Rust port — fails the deletion test's "client" column) or `client` (legitimately UI-side: file pickers, image cropping, window/navigation state, rendering). `none` when `kind` is `none`. |
| `target_domain` | Which later phase/domain this symbol's Rust translation belongs to (e.g. `worlds`, `java-runtime`, `modpack-client-only`, `n/a` for `client`/`none` rows). Informal — a pointer for Phase 1+, not a committed schedule. |
| `source_line` | Line number of the symbol's declaration in MSC 1 at the time of this row, so a reviewer can jump straight to it. |
| `notes` | The deletion-test reasoning for this row's disposition, or — for a `none` row — why the file has nothing to extract. Every row must be justified here; "looks agent-ish" is not a justification. |

## Coverage rule

Per P0.26/P0.27: a file may not be silently skipped. Every file in scope
(all 59 `mixed` files; every UI-bucket file the P0.25 scanner flags) gets at
least one row. A file with genuinely nothing to extract gets exactly one row
with `symbol=(none)`, `kind=none`, `disposition=none`, explaining in `notes`
why — coverage must be provable by scanning the CSV for that filename, not
assumed because it's absent.

## What is NOT a row

Per the deletion test's own examples (`msc2-port-plan.md` §1): avatar image
cropping/rendering, file-picker presentation, window and navigation state,
and other purely presentational concerns are `client` disposition when they
come up, but routine SwiftUI view bodies, `@Published` property plumbing,
and view-model glue that only forwards to an agent-owned symbol elsewhere
are not separately ledgered — the ledger tracks *behavior* (parsers,
policies, workflows), not every declaration in a file.
