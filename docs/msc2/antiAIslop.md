# Anti-AI-Slop — a guiding principle for MSC 2

> **Status:** Guiding principle, owner-approved 2026-08-25 (Cameron Temple).
> **Mandatory reading** for every agent before any design, styling, or frontend
> work on MSC 2. If your task touches how the app *looks*, you read this first.
> **This is a living document** — the Sources list at the bottom is open. Append
> new articles and findings as they surface; the rules grow with them.

---

## The principle, in one line

**MSC 2 must not look vibe-coded, generic ("regular"), or like any other app.**

Every visual decision must be *deliberate and specific to MSC*. If a choice could
have come from "the average of a million training-data websites," it is wrong
here by default. The absence of intention is the thing we are guarding against.
Taste and intention are the product; carelessness reads instantly and cheapens
everything around it.

This sits alongside the project's other hard commitments (behavior parity with
MSC 1, the design system in `MSCStyles.swift` as the *starting* palette, the
"same shape, refreshed" goal). Where a screen could be built the easy/average
way or the considered way, it is built the considered way — and the considered
way is defined here.

## Why this document exists

"Vibe coding" means generating an interface from prompts without applying design
judgment. The result is a recognizable set of visual patterns that are
*statistical artifacts of training data*, not choices that reflect the product's
purpose. Multiple designers, working independently, keep identifying the **same**
tells — which is exactly why they're detectable, and why we can pre-empt them.

MSC 1, though hand-built, already trips several of these (heavy glassmorphism,
gradient-stacked buttons, layered shadows, whole-chrome accent tinting). The
MSC 2 redesign is our chance to keep MSC 1's *shape and personality* while
removing the slop. This document is the checklist that keeps us honest.

---

## The catalog of tells

Synthesized from the Sources below and de-duplicated. For each: **the sign**
(how to spot it), **why it happens** (so we understand the failure mode), and
**the fix** (what we do instead). Grouped by concern.

### Color

**1. Homogeneous color goo / neon palettes.**
- *Sign:* Elements mushed together in near-identical hues — a cyan icon in a
  sky-blue box inside a slightly-different-blue card with a minty-blue border.
  Or the opposite failure: several high-saturation colors all competing, none
  winning.
- *Why:* Models generate toward the mean; neon and "software-y" palettes were
  over-represented (they perform on social media).
- *Fix:* A **color budget**. One dominant neutral, one supporting tone, one
  accent that pops — the **70 / 20 / 10** rule (70% neutral, 20% complementary,
  10% accent). Background contrast alone can separate elements; you often don't
  need borders at all.

**2. Purple gradients (and trend-chasing color in general).**
- *Sign:* Purple-to-indigo gradients used as generic "innovative software"
  shorthand.
- *Why:* Influential products used them; the model treats the trend as a default.
- *Fix:* Base color on the product's own identity, never on trend replication.

### Surfaces & depth

**3. Decorative glow / aurora / radial gradients in dark mode.**
- *Sign:* Ambient glows, aurora backgrounds, radial gradients behind content that
  serve no functional purpose.
- *Why:* Gaming/crypto aesthetics were heavy in training data for "dark + modern."
- *Fix:* Get depth from **typography, contrast, and intentional surface levels** —
  not from light effects.

**4. Glassmorphism everywhere.**
- *Sign:* Semi-transparent frosted-glass panels, especially paired with a gradient
  background and a 1px light border. Also creeps into badges and buttons.
- *Why:* Trendy, easy to generate, over-represented. "The new purple gradient."
- *Fix:* Flat, opaque surfaces on defined tiers. Reserve blur for the rare case
  where it genuinely aids focus (a true modal scrim), never as decoration. Glass
  also wrecks readability when done casually — Apple needed many betas to make it
  legible; a one-shot generation will not.

**5. Gradients and shadows out of place.**
- *Sign:* Linear-gradient fills on buttons and highlighted words; soft shadow
  "backdrops" behind buttons that muddy the section and invent false hierarchy.
- *Why:* Cheap to generate, so they get sprayed everywhere.
- *Fix:* Solid accent fill for a primary button, no gradient, no shadow. Use a
  shadow only when an element genuinely floats above another plane.

### Icons & imagery

**6. Simple icon inside a rounded, same-hue box.**
- *Sign:* A generic icon (or emoji) wrapped in a small tinted rounded square,
  usually the same hue as the icon, on *informational* elements.
- *Why:* Templates over-indexed on this layout; the model generalizes it.
- *Fix:* On informational elements, **drop the icon** — it rarely conveys anything
  the label doesn't. Icons earn their place on **actions** (buttons), not on
  status readouts. If an icon stays for scanning, keep it **neutral**, not colored.

**7. Emojis as icons/UI.**
- *Sign:* Emojis used for navigation, headers, bullets, section markers.
- *Why:* Onboarding-flow training data.
- *Fix:* Never use emojis as interface elements. Use one consistent, restrained
  icon set. Reserve emoji for actual human communication, if ever.

### Typography

**8. Excessive / reflexive serif.**
- *Sign:* A serif hero headline (Instrument Serif, DM Serif) used as a stand-in
  for "elegant." Especially a Claude tell.
- *Why:* A trend that spiked and got over-learned.
- *Fix:* Don't reach for serif as a shortcut to sophistication. MSC 2 is a
  system sans with restrained weights — regular (400) for body, and emphasis
  kept to 500 (section/card titles) and 600 (page/sheet titles) only, nothing
  heavier. Serif only if there were ever a deliberate editorial reason — there
  isn't one here.

### Layout & hierarchy

**9. Nested cards / excessive layers.**
- *Sign:* Cards inside cards, 2–4 containers deep. A sub-card just to hold a
  count and a button.
- *Why:* The model applies "group = wrap in a card" without weighing the visual
  cost.
- *Fix:* One card depth. Group with **whitespace and type weight**, not nested
  boxes. A card should mean "an actionable/bounded object," not "a generic
  container." Remove extra containers; make secondary text visually *quieter*
  than primary.

**10. Weak visual hierarchy generally.**
- *Sign:* Nothing clearly first. Font sizes, weights, and colors don't guide the
  eye. Spacing is off; the page is a flat wall of equal-weight elements.
- *Why:* Averaging produces "fine everywhere, sharp nowhere."
- *Fix:* Every screen has a deliberate first-, second-, third-read. Achieve it
  with size, weight, and *restraint* (quieting the non-essential), so a glance
  lands where it should.

### Status & accents

**11. Multicolored side tabs / accent rails on every block.**
- *Sign:* A different-colored accent bar on the left (or top) of every content
  block. Frequently the `border-left` + `border-radius` combination that leaves
  a telltale mismatched corner.
- *Why:* The model learned "colored accent = selection/emphasis" but ignores the
  page-level color budget, so *everything* gets emphasized — which means nothing
  is.
- *Fix:* Treat color as a **shared, scarce resource.** Decide what actually
  deserves emphasis and let only that carry accent. No per-card rails. (Simplest
  correct fix for the mismatched-corner artifact: remove the border entirely.)

**12. Meaningless status dots.**
- *Sign:* Colored circles sprinkled on nav items, headers, labels with no defined
  meaning.
- *Why:* Extracted from developer-tool UIs, stripped of the context that gave
  them meaning.
- *Fix:* A status dot must map to a **defined state** and be paired with a **text
  label**. Used that way it's correct and useful; used as decoration it's slop.
  (MSC's health dots — OK / Warn / Error, always labeled — are the *correct*
  usage; keep them, don't multiply them.)

### Motion

**13. Gratuitous or broken animation.**
- *Sign:* Everything animates. Hover effects that move *and* grow in different
  directions; slow staggered card-appear entrances; scroll animations that break
  when elements are already in view.
- *Why:* Easy to add, so it's added without purpose.
- *Fix:* Animation must **serve a purpose** — communicate a state change or
  spatial relationship. Keep MSC's functional transitions (tab switch, save HUD,
  press feedback). Nothing decorative, nothing slow, nothing that fights itself.

---

## How this binds MSC 2 — the design law

The catalog above, translated into the concrete rules the redesign is built and
reviewed against. These use MSC's real tokens (see `MSCStyles.swift`).

1. **Color budget 70/20/10.** ~70% neutral tiers (`#0D0D0F` atmosphere →
   `#141417` chrome → `#1C1C21` content → `#0A0A0C` terminal), ~20% quiet
   complementary (white-opacity text steps), **~10% accent** — the per-server
   `bannerColor` plus the status ramp (`#4DC778` ok, `#FF9140` warn, red error,
   `#59A1FF` bedrock). Accent is spent *only* on: running state, active tab,
   primary action, a live-stat fill, a defined status dot. Nothing else is
   colored.

2. **Depth from tiers, not effects.** No glassmorphism, no specular highlights,
   no aurora/radial glow, no gradient fills. Separation comes from the tier
   value-step; add a border only where contrast alone can't carry it.

3. **No decorative color-carriers.** No side rails / accent bars on cards. No
   colored-icon-in-tinted-box. Status is shown through **dot + text label**
   only. Informational icons are neutral or absent; colored icons are reserved
   for actions.

4. **Flat containment.** One card depth — never cards-in-cards. Group with
   whitespace and type weight. A card = an actionable/bounded object.

5. **Deliberate hierarchy on every screen.** A clear first/second/third read,
   achieved by size, weight, and quieting the non-essential.

6. **Motion with purpose only.** Functional transitions kept; decorative motion
   removed.

7. **System sans, restrained weights, no serif.** Regular (400) body; emphasis
   only at 500 (section/card titles) and 600 (page/sheet titles), nothing
   heavier. No serif reflex.

8. **One deliberate flourish:** the live terrain banner (running state). It is
   the single bold expression of the 10% accent budget — and keeping everything
   else disciplined-neutral is precisely what earns it. Do not add a second
   "signature" moment that competes with it.

## The anti-slop review checklist

Run this against every screen before it is considered done. Any "yes" is a
defect to fix, not a preference to debate.

- [ ] Does any element carry accent color that isn't running-state, active tab,
      primary action, a live stat, or a defined+labeled status?
- [ ] Any glass / blur / specular / gradient fill / glow used decoratively?
- [ ] Any card with a colored side rail or accent bar?
- [ ] Any colored icon on an *informational* (non-action) element? Any emoji?
- [ ] Any card nested inside another card?
- [ ] Any shadow on an element that doesn't actually float above another plane?
- [ ] Any serif type? Any weight other than regular/medium?
- [ ] Is there a single, glanceable first-read — or does everything weigh the
      same?
- [ ] Any animation that doesn't communicate a state or spatial change?
- [ ] Could this exact screen have come from a generic template? If yes, it's not
      done.

## Enforcement

- **Every agent reads this document** before any design or frontend step in the
  redesign. The Phase 12 plan (and any later design work) names this file as
  required reading in its steps.
- **Cameron's visual verification** for redesign screens includes running the
  checklist above, not just "does it match MSC 1."
- When in doubt between two treatments, the quieter, more intentional one wins.
  "Too cluttered / too decorated" is the far more common failure than "too plain."

---

## Sources

*Open list — append new articles and findings here as they surface.*

1. Yuwen Lu (@yuwen_lu_), "Signs of vibe coded UI," 2026-04-06.
   <https://x.com/yuwen_lu_/article/2041187936738447565>
   — Categories: color goo (70/20/10), icon-in-rounded-box, emojis, serif
   overuse, glassmorphism, out-of-place gradients/shadows, nested layers,
   gratuitous animation. Notes the `border-left` + `border-radius` accent tell.

2. The Fountain Institute, "7 signs a UI has been vibe coded."
   <https://www.thefountaininstitute.com/blog/signs-vibe-coded-ui>
   — Seven tells: neon palettes, decorative dark-mode glow, emoji icons, purple
   gradients, nested cards, multicolored side tabs, meaningless status dots.
   Frames color as a shared page-level budget; depth from type/contrast/surfaces.

<!-- Add new sources above this line. When you add one, fold any *new* tells into
     the catalog and, if needed, the design law and checklist. -->
