# MSC 2 — Vision Set

**Set revision:** 1.3 · **Date:** 2026-07-29
**Owner:** Cameron Temple
**Baseline:** MSC 1 at commit `fccd61f0ed743086f1f5db6bef58e228a36010f3` (246 production Swift files, 97,357 lines)

This is the entry point. The five documents below are **one controlled set** and are meant to be read together. If you are returning to MSC 2 with no memory of the project, read this page, then `msc2-decisions.md`.

---

## The set

| Document | Revision | Contains | Changes |
|---|---|---|---|
| **`MSC2-VISION.md`** (this file) | 1.3 | Index, revision state, owner-confirmed requirements, precedence rules | Rarely |
| **`msc2-product.md`** | 1.3 | What MSC 2 is in plain language: purpose, audience, experience, guarantees, non-goals | Rarely |
| **`msc2-engineering.md`** | 1.3 | Architecture, API contract, module boundaries, platform matrices, security, verification guarantees, resource requirements | Occasionally |
| **`msc2-decisions.md`** | 1.3 | Numbered decision register with origin, approval, rationale, rejected alternatives | Append-only |
| **`msc2-port-plan.md`** | 1.2 | Execution sequencing and the fixture inventory | **Often — deliberately separated** |

**Why the port plan is separate.** The vision defines the destination and its guarantees; the port plan defines a route. Routes change. Keeping them apart means a rescheduled phase never forces an edit to the vision.

---

## Precedence during conflict

**Approval status outranks document.** An **Approved** requirement always beats a **Proposed** one, whichever file each lives in. A proposal cannot override an owner-approved requirement merely by appearing in the decision register.

Within the same approval status:

1. **`msc2-decisions.md` wins.** It records reasoning and approval state; the other documents are downstream of it.
2. **`msc2-engineering.md` wins over `msc2-product.md`** on any technical claim.
3. **`msc2-product.md` wins over `msc2-engineering.md`** on intent, audience, and what MSC is *for*.
4. **`msc2-port-plan.md` never wins.** If it conflicts with anything, the port plan is wrong.
5. **`~/Desktop/msc2.md`** — the original vision written with Codex — is **superseded on architecture**, but remains a valid source of *owner intent*: several requirements in this set (headless everywhere, full mobile capability, the memory objective) originate there and are approved on that basis.

Any conflict discovered between documents is a defect. Fix it by adding or amending a decision entry, then propagating.

---

## Approval state

Decisions carry `Origin`, `Approved by`, and `Approval date`. Two statuses matter:

- **Approved** — the owner personally confirmed it. Do not reopen without evidence of the kind named in its *Revisit if* clause.
- **Proposed** — analysis-derived recommendation, documented and reasonable, **not owner-confirmed**. Safe to build against provisionally; must be approved before it constrains anything expensive.

### Owner-confirmed requirements

Everything below was decided by the owner directly, not inferred.

| # | Requirement | Entry |
|---|---|---|
| 1 | MSC 2 runs on macOS, Windows, and Linux, including **native Windows**. This is the requirement that determines the engine language. | D-002 |
| 2 | The engine is **Rust**; the desktop and web interfaces are **one Svelte frontend**, shipped as a Tauri shell and as a served page. | D-002, D-003 |
| 3 | The existing **Swift iOS app is kept** and re-pointed at the new API. | D-004 |
| 4 | MSC 2 is a **completely separate app and project**. It never touches MSC 1. Migration is by **import only**. | D-001, D-009 |
| 5 | Version skew is handled by a **supported-version floor with capability degradation**, and a clear refusal below it. *(The specific N-3 value is proposed, not approved.)* | D-010 |
| 6 | On desktop, the **app installs and manages the agent**; headless installs separately. | D-011 |
| 7 | Browser sessions use an **httpOnly cookie**; the desktop shell injects a **local token**. *(The rest of the auth design is proposed.)* | D-012 |
| 8 | The client is **multi-host from day one** — state keyed by host, minimal switcher in v1. | D-013 |
| 9 | **Minecraft 1.20** is the version floor. | D-014 |
| 10 | The **v1 non-goals** are approved as written: TUI deferred; no third-party plugin API; no per-person identity yet; no TempleTech-hosted backend, ever; no proxy/network orchestration; no Android. | D-015 |
| 11 | **Complete headless mode on every platform** — macOS, Windows, and Linux — with the GUI optional everywhere. *(From `msc2.md`.)* | D-011 |
| 12 | **Resource efficiency is a requirement**, not an aspiration. *(Founding motivation, from `msc2.md`. Specific benchmark values remain proposed.)* | D-021 |
| 13 | **Full mobile capability** — the phone is not a status-only remote. *(From `msc2.md`. The matrix as tracking mechanism remains proposed.)* | D-023 |

### Awaiting approval

These are load-bearing but not yet owner-confirmed. Review them before they constrain expensive work.

| Entry | Subject | Why it matters |
|---|---|---|
| D-005 | MSC 1 is the compatibility oracle | Determines whether behavior is ported or redesigned |
| D-006 | MSC 1's API as compatibility **baseline** — superset allowed, bugs may be corrected | Determines the shape of everything downstream |
| D-007 | macOS Bedrock stays a Swift sidecar | Affects packaging on macOS |
| D-008 | The Docker Bedrock backend is not ported | Removes 658 lines from scope |
| D-012 | The six unspecified authentication areas | **Largest open design gap** |
| D-016–D-018 | Port strategy, Windows timing, evidence-before-translation | Shape the whole schedule |
| D-019 | Formalizing MSC 1's existing permission model | Corrects a factual error in revision 1.0 |
| D-021 | The specific memory *targets* (the requirement itself is approved) | Must be measured, not estimated |
| D-022 | Separate MSC / Java / Bedrock support matrices | Prevents an unsupportable promise |
| D-023 | The matrix *mechanism* (full capability itself is approved) | Prevents repeating MSC 1's iOS parity gap |
| D-024 | Power management: two policies by host role | Remote-starting a stopped server needs the host awake |
| D-020 | Repository name and location | **Open, blocks everything** |
| D-025 | **Service identity and privilege boundaries** | **Open.** Blocks the substrate and the D-012 local-auth design |

---

## The shortest possible summary

MSC 2 extracts MSC's engine into a cross-platform Rust service that runs with or without a screen, wrapped in one interface that ships as a desktop app and a web page, with the existing iOS app and a CLI as first-class peers — so a modest computer spends its memory on Minecraft instead of on a desktop nobody is looking at.

**It is not a blank-slate rewrite.** MSC 1 is the executable specification. Two independent audits agreed at file level on **88.6%** of 246 files, and identified roughly **33,000–36,000 lines** of engine behavior to translate — about one third of the tree. That figure measures preserved behavior only; the genuinely new work (cross-platform services, operation journaling, secret stores, native Bedrock runtimes, client state) may exceed it.

---

## Known gaps in this revision

Recorded honestly so they aren't mistaken for completeness.

1. **Authentication is incomplete.** Six areas are unspecified — most importantly how a desktop app pairs with *remote* hosts, which multi-host (D-013) makes a first-class case. `msc2-engineering.md` §10.
2. **The permission-category vocabulary is unvalidated** against all 87 routes. D-019.
3. **Linux secret storage is unresolved.** The `keyring` crate resolves to the freedesktop Secret Service, which minimal Debian does not have. A headless fallback — `systemd` credentials or protected root-owned storage — must be chosen. `msc2-engineering.md` §8.
4. **The symbol ledger does not exist.** The two audit CSVs are file-level *inputs* to it; they say which files to open, not which symbols must survive. Building it is a Phase 0 deliverable.
5. **Service identity is undesigned.** Which account runs the agent, who owns server directories, when escalation is permitted, machine-scoped secret storage, macOS TCC consent without a GUI, and how updates cross the privilege boundary. MSC 1 never faced this — it is a user-session app. D-025, `msc2-engineering.md` §8.
6. **Idle-agent memory targets do not exist.** They must be measured, not estimated. D-021.
7. **The MSC 2 repository does not exist.** D-020.
8. **The capability matrix is defined but unpopulated.** D-023.
9. **The version-skew floor is undecided.** An earlier draft asserted N-3; that was an estimate. Set it from real update-adoption data. D-010.

---

## Supporting artifacts

Not part of the controlled set; evidence behind it. Currently in `~/Desktop/`.

| File | Contains |
|---|---|
| `msc2.md` | The original vision, written with Codex. Superseded on architecture. |
| `msc2-audit-claude.md` | Claude's independent portability audit |
| `msc2-codex-independent-audit.md` | Codex's independent portability audit |
| `msc2-audit-reconciliation.md` | Disagreements re-tested against the code, with verdicts |
| `msc2-audit-comparison.md` | Codex's comparison of both audits |
| `msc2-audit-exact-diff.md` | File-level confusion matrix; 88.6% agreement; 28 adjudicated files |
| `msc2-claude-file-inventory.csv` | 246-row per-file disposition |
| `msc2-codex-file-inventory.csv` | 246-row per-file disposition |

The two CSVs join on `file`. They are **file-level inputs to the future symbol ledger**, not the ledger itself — they say which files to open, not which symbols inside them must survive. Building the ledger is a Phase 0 deliverable (D-016).

---

## Revision history

| Rev | Date | Change |
|---|---|---|
| 1.0 | 2026-07-29 | Initial three-document set. |
| 1.3 | 2026-07-29 | Third Codex review: symbol-ledger contradiction removed set-wide; Phase 0 reduced to baseline + harness + ledger with per-domain characterization moved next to each translation; cross-platform tests moved out of MSC 1 characterization; product permissions corrected to name scoped tokens; D-024 (power management) and D-025 (service identity, Open) added; wording polished — graphical desktop environment, unmeasured memory figure removed, drift claim softened. |
| 1.2 | 2026-07-29 | Second Codex review: owner-origin requirements promoted to Approved (headless everywhere, resource efficiency, full mobile capability); N-3 and mechanical-gate contradictions removed; API restated as baseline + extension + correction; symbol-ledger overclaim corrected; LaunchDaemon and headless sidecar specified; Linux secret storage opened as a named gap; macOS desktop example corrected; Phase 0 split from cross-platform acceptance tests; Windows added to the Phase 4 exit criteria; precedence now ranks approval status above document. |
| 1.1 | 2026-07-29 | Codex review incorporated: approval state separated from analysis; tri-platform headless; auth gaps named; MSC 1 permission model corrected; "no cloud" principle restated; execution split into `msc2-port-plan.md`; extraction rule relaxed; resource efficiency, support matrices, and capability matrix added; this index created. |
