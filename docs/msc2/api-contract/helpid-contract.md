# The `helpId` contract — content format and wire shape

**Status: Confirmed** by Cameron Temple, 2026-07-31. Both decisions below were proposed during P2.2 and confirmed during the Read move that followed, per D-026's own "Open" list and `msc2-engineering.md` §18/§19 ("Educational content format; embedded vs on-disk … blocks Phase 2 contract freeze"). D-026 is amended accordingly.

---

## 1. Content format: Markdown with YAML front-matter

Each topic is one `.md` file: YAML front-matter for structured fields, Markdown body for prose.

```markdown
---
id: settings.difficulty
title: Difficulty
category: settings
analogy: "Difficulty is how aggressive the world's monsters and hunger are — not how hard it is to log in."
relatedIds:
  - settings.hardcore
  - health.mob-griefing
---

Difficulty controls monster spawn rates, damage, and whether players lose
hunger. `peaceful` disables hostile mobs entirely...
```

Why: it's D-026's own "obvious candidate" (§ Consequences), it's diffable and reviewable in git (the decision's stated goal), every router guide and handbook topic MSC 1 already writes as prose maps onto it directly, and Cameron already reads Markdown comfortably (per `CLAUDE.md`, he works in Python day to day and reads Swift, but no existing tooling knowledge is needed to *edit* a help topic — that's the point).

Rejected alternative: plain JSON strings for body text. Works for the wire format but is hostile to write and review — nobody hand-edits multi-paragraph prose inside a JSON string value.

## 2. Storage: embedded in the agent binary for v1

Content files live under `content/help/**/*.md` in the repo and are compiled into the `msc-agent` binary at build time (`rust-embed` or `include_str!` — a build-time choice, not a contract concern; either produces the same runtime behavior).

Why: matches the product promise in D-026's origin quote — "the application teaches without forcing the user to leave the interface" — which only holds unconditionally if help is *always* present, including on a freshly-installed agent with no network access and no separate content package to fetch. On-disk override remains available to add later (Phase 5+, configuration/migration territory) if hot-editing content without a release turns out to matter in practice; nothing in this contract forecloses it — a future on-disk path can shadow the embedded copy by `id` without changing the `GET /v1/help/{helpId}` response shape.

Rejected for v1: read from disk only. Requires the content directory to ship and stay in sync with the binary as a second artifact, and produces a broken help experience on any installation where that directory goes missing or drifts — the exact duplication-and-drift failure mode D-026 exists to eliminate, just moved from "four clients" to "binary vs content directory."

## 3. The `helpId` shape

A dotted-namespace string, lowercase, hyphenated within segments: `<namespace>.<name>` or `<namespace>.<subnamespace>.<name>`. The namespace mirrors the DTO family the field lives on, not the UI screen it happens to render in today (screens change; the data shape is the stable anchor D-026 argues for).

Resolved via a new route:

```
GET /v1/help/{helpId}
→ 200 HelpTopicDTO { helpId, title, analogy?, body, category, relatedIds[] }
→ 404 ErrorDTO   (per P2.4's error envelope — unknown helpId is a normal, expected case,
                   not a server fault: an older agent may not yet have a topic a newer
                   client's schema references)
```

`body` is the raw Markdown; clients render it, per D-026 point 1 ("Clients render; they do not author").

## 4. Where `helpId` attaches — every DTO field `msc2-engineering.md` §18 names

§18 names six categories in prose ("settings fields, health cards, diagnostics, performance metrics, connection methods, crash-analysis findings"). Read against the actual MSC 1 DTOs (`RemoteAPIModels.swift`, the iOS mirror of the wire format), each maps onto a concrete existing field:

| §18 category | MSC 1 DTO (today) | Field carrying the pointer | `helpId` namespace | Example |
|---|---|---|---|---|
| Settings fields | `SettingFieldDTO` | replaces the existing free-text `help: String?` field | `settings.<key>` | `settings.difficulty`, `settings.view-distance`, `settings.max-players` |
| Health cards | `HealthCardDTO` | new `helpId: String?` alongside `detail` | `health.<card-id>` | `health.tick-lag`, `health.disk-space` |
| Diagnostics / crash-analysis findings | `StartupProblemDTO` (drives `HealthProblemsResponseDTO`; covers both `StartupCrashAnalyzerTests` and `ConnectorCrashAnalysisTests` findings) | new `helpId: String?` keyed off `kind` | `diagnostics.crash.<kind>` | `diagnostics.crash.forge-dep`, `diagnostics.crash.missing-mod` |
| Performance metrics | `PerformanceSnapshotDTO` | new sibling `PerformanceMetricHelpDTO` map, or per-field `helpId`s the client looks up by metric name (metrics are scalars today, not sub-objects, so the pointer can't live *inside* `tps1m`/`cpuPercent` the way it lives inside `SettingFieldDTO`) | `performance.<metric>` | `performance.tps`, `performance.cpu`, `performance.ram`, `performance.world-size` |
| Connection methods | `ConnectivityResponseDTO.method` (`playit \| duckdns \| public-ip \| none`) | new `helpId: String?` alongside `method` | `connectivity.method.<method>` | `connectivity.method.playit`, `connectivity.method.duckdns` |

Note for P2.8 (contract assembly): `SettingFieldDTO` already carries a `help: String?` field in the MSC 1 baseline (short inline text, e.g. "Controls monster damage and hunger loss"). The v1 DTO **replaces** that field with `helpId`, it does not add a second field alongside it — carrying both forward would let the two drift, which is the exact failure D-026 exists to prevent. The short inline text becomes the `analogy` front-matter field on the resolved topic instead, so nothing is lost, only relocated.

`PerformanceSnapshotDTO` is the one category that doesn't fit the "add a `helpId` field next to the value" pattern used everywhere else, because its fields are bare scalars (`tps1m: Double?`, not a sub-object). P2.8 should decide between (a) a fixed, contract-documented mapping of metric name → `helpId` that ships as static client knowledge (cheapest, but a second place the mapping must be kept in sync — mildly re-introduces the duplication D-026 removes elsewhere) or (b) wrapping each metric in a small `{ value, helpId }` shape matching the other five categories (consistent, but changes a DTO shape P0.24/P0.32 already catalogued from the baseline). Flagged here as a P2.8 decision, not resolved by this step — this step's job is the `helpId` shape and content model, not re-designing `PerformanceSnapshotDTO`.

## 5. Router guides and onboarding

D-026 point 3 keeps the router guide catalog as data separate from its matcher/resolver/composer (executable behavior, ported to Rust, not content). Router guide steps use the same embedded-content storage as handbook topics and are reached through their own catalog routes. The first-launch tour is structured agent data; its UI anchoring remains client-owned. This step defines the pointer mechanism (`helpId` → `GET /v1/help/{helpId}`) that DTOs use to reach into served content.
