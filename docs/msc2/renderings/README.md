# S0 design renderings — the visual source of truth

Each `.html` here is a **standalone, locked reference specimen** for MSC 2's
refreshed design system (Phase 12, section S0). Open any file directly in a
browser to see it rendered (they pull the Tabler icon webfont from a CDN, so view
them online for icons; the colors, sizes, weights, and spacing are all inline and
work offline regardless).

These are the **authority for exact values** — copy the tokens straight from the
markup. They are governed by, and must always stay consistent with,
[`../antiAIslop.md`](../antiAIslop.md) (the design law) and the design-system spec.
If a specimen and the spec ever disagree, that's a bug — fix it, don't guess.

## The specimens

| File | What it locks |
|---|---|
| [`status-card.html`](status-card.html) | The **card language** — flat `#1C1C21` on tier contrast (borderless), neutral icon, status carried only by dot + colored label, one whisper shadow. The base every card inherits. |
| [`buttons-and-type.html`](buttons-and-type.html) | The **button system** (2 shapes: filled Primary/Start/Stop + quiet Secondary/Destructive/Ghost-icon; sizes md 32 / sm 26) and the **7-role type scale** (weights 600 titles / 500 mid / 400 body). |
| [`primitives.html`](primitives.html) | The **shared primitive kit** — segmented control, toggle (green-on), field/number/select, category-vs-status badges, divided list rows, empty state. |
| [`shell.html`](shell.html) | **The S1 shell skeleton** — window chrome + terrain banner, sidebar control rail, host-aware picker, header, tab strip, docked console. The frame every screen lives in. Shown at ~680px; the real window is wider (full tab labels, roomier). |
| [`decorated-vs-disciplined.html`](decorated-vs-disciplined.html) | **Teaching reference, not a component.** Left = the AI-slop version we reject (side rail, colored icon, border); right = the locked card. Shows *why* the rules exist. Build the right one. |

## Locked token quick-reference

Pulled from the specimens so an agent doesn't have to parse markup:

- **Surface tiers:** atmosphere `#0D0D0F` · chrome `#141417` · content `#1C1C21` · terminal `#0A0A0C`
- **Status ramp:** ok `#4DC778` · warn `#FF9140` · error `#E24B4A` · bedrock-blue `#59A1FF`
- **Text (white opacity):** primary `.95` · secondary `.55` · tertiary/overline `.40`
- **Card:** `#1C1C21`, radius 12, no border (tier contrast separates), shadow `0 1px 2px rgba(0,0,0,.3)`, padding 15–16
- **Buttons:** radius 8 (md) / 7 (sm) · filled = solid fill + no gradient/shadow · quiet = 1px `rgba(255,255,255,.14)` hairline · md 13px pad 8×16, sm 12px pad 5×12
- **Type:** page 21/600 · section 15/500 · card 13/500 · body 13/400 · meta 11/400 · overline 10/600 caps +0.8 tracking · mono 12/400
- **Spacing scale (4pt):** 2 4 8 12 16 20 24 32 · **radius scale:** 6 10 14 18

## Adding specimens

As S0 grows (shared primitives — segmented control, toggle, field, badge, list
row, empty state, sheet frame — then whole screens), add each as a new
standalone `.html` here and a row in the table above. Keep them minimal and
self-contained; they are the reference agents build against.
