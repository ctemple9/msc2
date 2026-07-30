# Fixture format

Defines the JSON shape every MSC 2 fixture file must have, and the directory
convention that places it. Every later Phase 0 extraction step (P0.3–P0.21)
produces files in this shape; the runner built in P0.2 validates against it.

## Why this shape

Per `msc2-port-plan.md` §5 ("Fixture strategy"), fixtures fall into two
kinds — **pure functions** (input/output, cheap, exhaustive) and **I/O
workflows** (temp directories, fake providers, rollback assertions). Both
kinds use the same six fields below; an I/O workflow fixture just has a
richer `input`/`expected` shape (e.g. a described directory tree instead of
a scalar). The format doesn't distinguish the two kinds structurally —
`domain` does that.

The `source` field exists because every fixture is a **port**, not new test
authorship: `msc2-port-plan.md` requires Rust output to be compared against
expected values, and every expected value here was read off MSC 1, not
invented. `source` is the pointer back to where it came from, so a
reviewer — or Cameron, or Codex — can always check a fixture against the
Swift test it was pulled from.

## Fields

| Field | Type | Required | Meaning |
|---|---|---|---|
| `domain` | string | yes | The logical area under test — e.g. `tps`, `component-version`, `dto-contract`. Matches the fixture's directory (see below). |
| `case` | string | yes | A short, unique-within-domain slug for this test case — e.g. `negative-tps-clamped-to-zero`. Matches the fixture's filename. |
| `source` | object | yes | Pointer back into MSC 1. See below. |
| `input` | any | yes | The value(s) fed to the function or workflow under test. Shape is domain-specific: a scalar or object for a pure function, a described directory/process setup for an I/O workflow. |
| `expected` | any | yes | The value(s) the port must produce. Read from MSC 1's actual behavior (its test assertion, or — where the test only checks a property rather than a literal value — MSC 1 run by hand), never invented. |
| `notes` | string | no | Anything a reader needs that the other fields don't carry: an edge case being deliberately exercised, a known MSC 1 quirk being preserved on purpose, a caveat about how `expected` was derived. |

### `source` object

| Field | Type | Required | Meaning |
|---|---|---|---|
| `file` | string | yes | The MSC 1 Swift file this case was pulled from, e.g. `TpsMonitoringTests.swift`. |
| `test` | string | yes | The Swift test function name, e.g. `testNegativeTPSClampedToZero`. |
| `line` | integer | yes | The line number of that test's `func` declaration in MSC 1, at the time of extraction. |

## Shape

```json
{
  "domain": "tps",
  "case": "negative-tps-clamped-to-zero",
  "source": {
    "file": "TpsMonitoringTests.swift",
    "test": "testNegativeTPSClampedToZero",
    "line": 42
  },
  "input": { "rawSample": -3.2 },
  "expected": { "tps": 0.0 },
  "notes": "MSC 1 clamps rather than rejecting negative samples; preserved as-is."
}
```

Every field above is a JSON object key; `domain`, `case`, `input`, and
`expected` sit at the top level, `source` is a nested object, and `notes`
is optional at the top level.

## Directory convention

```
fixtures/<domain>/<case>.json
```

- `<domain>` matches the fixture's own `domain` field exactly, and is one
  directory per extraction step (e.g. `fixtures/tps/`, one directory per
  source Swift test file, per the grouping in `rolling-plan.md`).
- `<case>.json` matches the fixture's own `case` field exactly, so the
  filename alone identifies the test case without opening the file.
- A reserved domain, `fixtures/_selftest/`, holds the two self-test
  fixtures P0.2 builds to prove the runner works before any real domain
  exists — not a real domain, hence the leading underscore.

## Validation

The P0.2 runner checks a fixture against this format in two ways:

- **Schema-only** (`--schema-only`): confirms the six fields above are
  present with the right types, and that `case`/`domain` match the file's
  own path. This is what P0.3–P0.21 use, since no Rust implementation
  exists yet in Phase 0 to compute an `actual` value to compare.
- **Full run**: additionally computes `actual` from the code under test
  and compares it to `expected`, exiting non-zero on mismatch. This is
  what later phases use once a Rust (or, for the two self-test fixtures,
  a deliberately trivial) implementation exists.
