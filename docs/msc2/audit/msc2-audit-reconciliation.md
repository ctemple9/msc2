# MSC 2 Audit Reconciliation — Claude vs Codex

**Date:** 2026-07-29
**Inputs:** `msc2-audit-claude.md`, `msc2-codex-independent-audit.md` (both produced blind)
**Method:** every disagreement re-tested against the code; verdict recorded

---

## Score

**Codex's classification is better than mine.** Four material disagreements; Codex is right on three, I'm right on one, and the one I won doesn't change Codex's conclusion.

| # | Disagreement | Verdict |
|---|---|---|
| D1 | Whole-file vs symbol-level classification | **Codex right** — my method silently discarded engine logic |
| D2 | `BedrockServerBackend.swift` disposition | **Codex right** — it's the legacy Docker backend; I said port it |
| D3 | Platform bucket sizing (4,832 vs 1,984 LOC) | **Codex right** — I misfiled 2,153 LOC by import |
| D4 | "140 files reference AppViewModel" | **Claude right on the number, Codex right on the conclusion** |
| — | Pure/domain sizing (44/12,796 vs 29/7,272) | **Not a disagreement** — see below |

---

## D1 — The Mixed bucket. Codex is right, and this is the important one.

I forced every file into exactly one bucket. Codex created a **Mixed** bucket (59 files / 30,942 LOC) for files that genuinely straddle domain logic and presentation.

I explicitly warned about this failure mode when critiquing import-based counting — *"a file that imports SwiftUI may contain a perfectly portable parser sitting next to a view"* — and then committed it anyway by classifying whole files.

**Verified consequence.** Five files Codex flagged as high-risk landed in my `4-UI` bucket, which my report labels *"replaced by Svelte, not ported"*:

| File | LOC | What my method would have thrown away |
|---|---:|---|
| `AddServerWizardView.swift` | 2,780 | staging, process checks, archive handling, source inspection |
| `ServerSettingsView.swift` | 788 | draft models, Java/Bedrock settings validation and mapping |
| `MinecraftCommandRegistry.swift` | 543 | full command catalog, arguments, server compatibility rules |
| `JavaInstaller.swift` | 371 | provider selection, archive extraction, runtime verification |
| `OverviewChatCardView.swift` | 244 | **a complete console chat parser** |

Codex's sharpest single find is that last one. Confirmed at `OverviewChatCardView.swift:168`:

```swift
static func parseEntry(_ entry: ConsoleEntry) -> ChatFeedMessage?
static func parseLine(_ raw: String, time: Date) -> ChatFeedMessage?
    // "[Not Secure] " prefix, <player> chat extraction,
    // advancement markers, "Player connected:", "Player disconnected:"
```

That is engine behavior — Java *and* Bedrock join/leave semantics — sitting underneath a SwiftUI card. My audit would have deleted it and rediscovered it later from user bug reports.

**Scanning my whole UI bucket for engine-shaped symbols** (`FileManager`, `Process(`, `URLSession`, `func parse*/detect*/validate*/resolve*`, `JSONDecoder`, string range extraction), comments stripped:

| Density threshold | Files | LOC |
|---|---:|---:|
| ≥3 engine symbols | 15 | 14,685 |
| ≥5 | 10 | 9,849 |
| ≥8 | 5 | 4,859 |

So **10–15 of my 109 "UI" files require symbol-level extraction before retirement.** Not all 14,685 lines are engine logic — these are large files with small embedded cores — but the extraction task is real and my report didn't contain it.

**Adopt Codex's taxonomy.**

---

## D2 — `BedrockServerBackend.swift` is the legacy Docker backend. Clean miss on my part.

My audit classified it as Platform (AppKit), then recommended *"reclassify to 2a I/O orchestration"* — i.e. port it.

Codex classified it as **Legacy: do not port without an explicit compatibility reason.**

Verified: **116 references to Docker**, and the file header describes streaming via `docker logs -f` and sending commands via `docker exec <name> send-command`.

```
BedrockServerBackend.swift:11  //   - Output is streamed via "docker logs -f" ...
BedrockServerBackend.swift:22  /// Lightweight, synchronous Docker CLI wrapper.
```

This is the backend the de-Dockerization work replaced with the VZ VM. Codex is right: 657 LOC that should not be ported at all. Porting it would have carried a dead runtime into MSC 2.

---

## D3 — Platform bucket. Codex is right; I flagged my own error and then didn't fix the table.

I reported Platform as 13 files / 4,832 LOC. Codex reported 12 files / 1,984 LOC.

The gap is almost entirely two files I bucketed by import and then contradicted in my own prose:

- `WorldSlotManager.swift` (1,495) — AppKit use is 2× NSImage, 1× NSSavePanel, NSError. ~30 LOC of macOS in 1,495 LOC of world logic. Belongs in Mixed.
- `BedrockServerBackend.swift` (658) — see D2, belongs in Legacy.

2,153 LOC misfiled. **Codex's number is the correct one.**

---

## D4 — The AppViewModel count. My number, Codex's conclusion.

Codex: *"140 of 246 production files reference the `AppViewModel` symbol."*

Measured both ways:

| Measure | Files |
|---|---:|
| References including comments | **140** |
| References in code only | **118** |
| **Comment-only references** | **22** |

Codex's 140 is comment-inflated by 22 files (16%) — the same artifact that produced my own bad first pass, where headers reading *"Pure service — no AppViewModel dependency"* registered as coupling.

**But Codex's conclusion stands on the corrected number.** 118 of 246 files is still overwhelming, and *"dependency direction, not imports, is the real portability issue"* is correct. That was my argument originally; Codex applied it more consistently than I did.

---

## Not a disagreement — pure/domain sizing

| | Files | LOC |
|---|---:|---:|
| Claude "Pure logic" | 44 | 12,796 |
| Claude, minus API files folded into it | ~36 | **7,144** |
| Codex "Pure/domain logic" | 29 | **7,272** |

We agree within **128 LOC**. The apparent gap was entirely that Codex broke `RemoteAPIServer*` (5,652) into its own bucket and I folded it into pure logic. Both audits reached the same disposition for those files independently: *the API becomes the spec, not ported code.*

Independent convergence on the API-as-contract finding is the strongest signal either document produced.

---

## Where Codex went deeper than I did

**The test-coverage finding.** I reported: 21 files, 270 tests, extract to fixtures — and treated that as reassuring. Codex reported the same corpus and then asked what *isn't* covered:

> Most current tests exercise pure parsing and API contracts. The most destructive workflows have much less coverage than their risk warrants.

That reframes Wave 0 entirely. The gap list — backup while running with `save-off`/timeout/resume, interrupted restore, rollback after partial rename, ZIP path traversal, process tree cleanup, agent restart while a server is running, historical config corpus, old-client/new-agent fixtures — is characterization testing **that does not exist yet and must be written**, not extracted.

My framing ("free the 270 tests into fixtures") understated Wave 0 by a large margin. Codex's framing is correct and this is the single most valuable finding across both documents.

**The substrate phase.** Codex's Phase 3 — path safety, atomic writes, versioned config + migrations, secret-store trait, audit log, download staging, operation journal — is a real improvement. I folded these into individual waves; they must exist *before* any domain touches user files. Adopt it.

---

## Where I'd still push back on Codex

1. **Mixed at 31.8% needs a severity gradient.** Any file with 2+ concerns lands in Mixed, so `WorldSlotManager` (~98% portable world logic, ~30 LOC of AppKit) sits in the same bucket as `AppViewModel.swift` (irreducibly entangled). Those are different projects. Split Mixed into **Extract-and-keep** (dominant domain core, thin UI skin) vs **Decompose** (genuinely interleaved). The 59 files are not 59 equal units of work.

2. **`HeadlessScriptGeneratorTests` rated "Medium" is too low.** Headless script generation encodes launch-command construction — Java args, args-file resolution, memory flags — which the Wave 4 Java vertical slice depends on directly. That's High.

3. **"Extract behavior before retiring a UI file" needs a trigger rule**, or it won't happen consistently across 109 files. Proposed: a file may not be deleted until it has zero `func`s containing string parsing, `FileManager`, `Process`, or `URLSession`. That's mechanically checkable in CI against the Swift tree during migration.

---

## Reconciled numbers

| Bucket | Files | LOC | Disposition |
|---|---:|---:|---|
| UI — pure presentation | ~94 | ~38,000 | Rebuild in Svelte |
| **Mixed — symbol-level split** | **59** | **30,942** | **Highest risk; split before retiring** |
| I/O orchestration | ~35 | ~8,100 | Reimplement idiomatically, preserve semantics |
| Pure/domain | 29 | 7,272 | Translate against fixtures |
| API + wire contracts | 8 | 5,652 | Becomes the OpenAPI spec |
| Platform | 12 | 1,984 | Per-OS adapters; one Swift sidecar |
| Legacy (Docker Bedrock) | 1 | 657 | **Do not port** |

**Revised engine logic to translate: ≈33,000–36,000 LOC**, up from my 31,500 — the increase is the engine behavior buried in UI files, less the Docker backend.

The headline holds: **roughly a third of the tree, not the whole tree.**

---

## Merged port order

Both orders were already close. This is Codex's spine with my wave-1 vertical-slice bias and the one addition we both missed.

| Phase | Content | Source |
|---|---|---|
| **0** | Fixture corpus: extract 270 tests → language-neutral fixtures **and write the missing destructive-workflow characterization tests**. Freeze API + DTO examples as the oracle. | Codex (expanded) |
| **0.5** | **Symbol-extraction pass over the 59 Mixed files + 10–15 UI files** — move parsers and policies out of view models and views *while still in Swift*, guarded by the CI rule above. Cheap, reversible, and it's what makes every later phase safe. | **Neither audit had this as a phase** |
| **1** | Domain types + pure rules → Rust. Flavors, versions, Java policy, property models, TPS, crash analysis, slug normalization, router logic, command catalog. | Both |
| **2** | Versioned OpenAPI + WebSocket contract; skeletal agent; operation IDs, progress, errors, capabilities, cancellation. | Codex |
| **3** | Substrate: path safety, atomic writes, config + migrations, secret-store trait, audit log, download staging, operation journal. | Codex |
| **4** | Java lifecycle vertical slice — import, start, console, command, status, graceful stop, restart — driven from CLI **and the existing iOS app**. | Both |
| **5** | Worlds + backups. Highest data-loss domain; goes before breadth. | Both |
| **6** | Provisioning + server families + startup diagnostics. | Codex |
| **7** | Mods, plugins, modpacks, client export. | Both |
| **8** | Networking + helpers: playit, resource-pack hosting, DuckDNS, broadcast, Geyser/Floodgate. | Both |
| **9** | Bedrock: Linux native → Windows native → macOS VZ Swift sidecar last. | Both |
| **10** | Tauri + web frontend; TUI after. | Both |

Phase 0.5 is the one thing neither audit proposed and both imply. It de-risks the Mixed bucket *before* any Rust exists, in a language where the compiler still checks the refactor, and it produces exactly the extracted pure functions that Phase 1 then translates.

---

## Bottom line

Neither audit found a reason to stop. Both independently reached: Rust is justified by the deployment model rather than by memory; a blank-slate rewrite is not justified; the existing API is the contract; the VZ backend stays Swift behind a sidecar; UI comes last.

The disagreements were worth running. Three of four went to Codex, and two of them — the Mixed bucket and the Docker backend — would have caused real damage: silently deleted parsers, and a dead container runtime ported into MSC 2.

**Next artifact: the Phase 0 fixture corpus, starting with the destructive workflows that have no coverage today.** That is the piece that is cheap now, impossible later, and the only thing that makes "same behavior" a checkable claim.
