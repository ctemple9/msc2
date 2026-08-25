# MSC 1 → MSC 2 UI difference inventory

**Purpose:** Record observed differences between the MSC 1 reference app and
the current MSC 2 desktop/web client before deciding how MSC 2 should change.
This is an evidence log, not an implementation plan or a decision record.

## Agent handoff instructions

Each difference ID is handled in its own conversation. When Cameron says
“clean up” a specific step, use this document as the context for that step:

1. Read the ID’s MSC 1 observation, MSC 2 observation, user impact, evidence,
   and notes before changing anything.
2. Inspect the relevant MSC 2 code and, when needed, the corresponding MSC 1
   source. Preserve the product outcome described here; do not broaden the
   work to unrelated gaps.
3. Implement the smallest parity change that closes that ID, then run the
   relevant targeted verification and report what was checked.
4. Update that row’s notes with the result. Cross off the ID and add `DONE`
   only when the change is implemented and verified. Leave it open if work is
   partial, blocked, or awaiting Cameron’s verification.
5. Commit the implementation and this row update together, following the
   repository’s normal phase/step commit rules. Do not edit MSC 1.

The entries below are the source of truth for the observed UI gaps. They are
not permission to redesign unrelated areas or to silently change approved
product decisions.

## Setup parity boundary

**In scope:** the first time an operator opens MSC on a host, from the opening
presentation through a trustworthy handoff to creating or importing their
first server. This includes required host readiness, server-type guidance,
optional access helpers, the summary of choices, and the next action.

**Adjacent, but outside this first pass:** the Concept Guide, guided tour, and
full Server Handbook. Setup must hand off to them coherently, but their
separate content and visual parity will be assessed after setup is resolved.

### Defined setup gaps

1. **MSC 2 has a completion gate, not a setup workflow.** Its first modal says
   “Set up MSC” but its sole action, “Finish setup,” records completion without
   asking, checking, or configuring anything. MSC 1 guides the operator through
   a real sequence of choices and readiness checks.

2. **The MSC 2 splash intro is missing.** MSC 1 opens with a branded video;
   MSC 2 previously rendered only a short CSS mark. The shared client now plays
   the reviewed `splash_intro.mp4` asset once, falls back safely if the asset
   cannot load, and skips it for reduced-motion users.

3. **The opening does not establish what the product is or what the operator
   will accomplish.** MSC 1 explains the app’s role, gives a compact capability
   overview, offers an accent choice, and sets a time expectation. MSC 2 gives
   no equivalent orientation, personalisation, or sense of scope before asking
   for completion.

4. **MSC 2 does not guide the first server decision.** MSC 1 explains Java and
   Bedrock, distinguishes supported Java server families, and gives enough
   practical context—plugins, mods, cross-play, and intended use—to make a
   first choice. The current MSC 2 flow reaches no server-type decision at all.

5. **MSC 2 does not establish host readiness before declaring setup complete.**
   The MSC 1 flow chooses a server root, finds or validates Java, and reports
   Bedrock readiness. The MSC 2 flow currently has no visible equivalent for
   agent availability, server storage, runtime availability, or platform
   constraints.

6. **Optional connection helpers have no first-launch context.** MSC 1
   introduces Playit.gg, Xbox Broadcast, and Tailscale as optional tools,
   explains why each exists, and provides truthful account, download, or
   installation-state information. MSC 2 does not currently surface these
   choices in setup.

7. **There is no reviewable setup result or explicit first-server handoff.**
   MSC 1 ends by summarising the chosen root, Java executable, and supported
   server types, then points directly to creating the first server. MSC 2 moves
   into generic educational cards and a tour without confirming what, if
   anything, is ready or identifying the next real task.

8. **The setup interaction design has lost operational meaning.** MSC 1 uses a
   visible step indicator, staged progression, contextual cards, checks,
   optional-step skips, and motion to make setup state and choices legible. MSC
   2's modal is visually clean but static and generic; its later tour shows
   progress but is not connected to setup work.

### Constraints for closing these gaps

- Preserve the operator outcome, not macOS-only implementation details. The
  shared desktop/web client must obtain readiness and availability from the
  selected MSC 2 agent.
- Do not claim a runtime or helper is ready when the agent cannot prove it on
  the current host. In particular, Bedrock, Java, and helper availability must
  stay capability- and platform-aware.
- Optional services remain optional. Their setup may be deferred, but the user
  should understand what they solve and where to return later.
- Setup completion must mean that required local prerequisites have either
  succeeded or been honestly deferred with a clear next action; it cannot mean
  only that a modal was dismissed.

## Entries

| ID | Area / task | MSC 1 observation | MSC 2 observation | Difference type | User impact | Evidence | Notes / later decision |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ~~FL-01~~ | Application opening | A full-window branded splash video plays while the application opens. | MSC 2 did not play the splash intro; its `SplashGate` rendered only a brief CSS mark (`◆ ◆ ◆`) before entering the app. | Visual language; workflow | Establishes a deliberate, recognisable opening moment rather than dropping immediately into the application. | MSC 1 user screenshot 1; `clients/desktop-web/src/lib/help/SplashGate.svelte`; user MSC 2 walkthrough (2026-08-24). | **DONE** — Implemented in this parity pass with the shared `splash_intro.mp4` asset, bounded fallback, and reduced-motion skip. |
| ~~FL-01a~~ | Splash handoff | MSC 1’s opening is presented against its dark splash background. | MSC 2 briefly showed a white native/WebView surface before the splash video appeared. | Visual language; workflow | Breaks the continuity of the opening animation and makes the desktop shell feel unfinished for a moment. | User report after splash implementation (2026-08-24). | **DONE** — Fixed by matching the Tauri window background and the pre-app HTML background to the splash surface. |
| ~~FL-02~~ | First-launch introduction | A multi-step, modal “First-time Setup” begins with a plain-language explanation of MSC, a concise capability summary, selectable shell accent colours, and an estimated completion time. | A modal says “Set up MSC” and offers only “Finish setup”; it gives no product explanation, capability summary, personalisation choice, setup estimate, or visible configuration task. | Workflow; content; visual language | Orients a new operator and lets them personalise the shell before configuration begins. | MSC 1 user screenshot 2; MSC 2 user screenshot 12 (2026-08-24). | **DONE** — Replaced the one-button gate with a shared first-launch introduction covering MSC’s purpose, four core capabilities, six accent presets plus a custom color, and the two-minute estimate. Accent choice persists in the shell and is restored on later launches; “Next” continues into the existing Concept Guide sequence. Targeted help tests and the production client build pass. |
| ~~FL-03~~ | Server platform and family choice | The setup explains Java versus Bedrock, then presents supported Java families (Paper, Purpur, Vanilla, Fabric, Forge, NeoForge) with concise purposes and cross-play guidance. The choices animate into view. | No server-platform or server-family choice is presented in the shown MSC 2 setup or tour. | Workflow; content; capability | Helps a new operator choose an appropriate server family before creating their first server. | MSC 1 user screenshot 3; MSC 2 user screenshots 12–18 (2026-08-24). | **DONE** — Added an animated Server Type page after the introduction. Java and Bedrock can be selected independently; Java expands to Paper, Purpur, Vanilla, Fabric, Forge, and NeoForge with staggered rows and Geyser guidance, while Bedrock explains BDS and its player platforms. The choice persists for the later setup steps, and Next is disabled until at least one platform is selected. |
| ~~FL-04~~ | Essential host configuration | The setup collects a server-root folder, checks or locates Java, and reports Bedrock’s built-in readiness before proceeding. | No server-root, Java, or runtime-readiness step is presented in the shown MSC 2 setup or tour. | Workflow; capability | Resolves prerequisites early and shows whether the host is ready for each server type. | MSC 1 user screenshot 4; MSC 2 user screenshots 12–18 (2026-08-24). | **DONE** — Added the Server Setup page. The agent now reports and validates the servers root, persists the selected root, checks Java 21+ through the real runtime route, and gates Bedrock on the agent’s advertised runtime state. |
| ~~FL-05~~ | Optional Playit.gg connection | The setup explains Playit.gg in product terms, links to account creation, states its role in tunnelling, and makes clear that this step may be skipped and completed later. | No Playit.gg introduction or optional access setup is presented in the shown MSC 2 sequence. | Workflow; content; capability | Introduces remote player access without making router configuration a prerequisite. | MSC 1 user screenshot 5; MSC 2 user screenshots 12–18 (2026-08-24). | **DONE** — Added the optional Playit.gg explanation, account link, tunnel guidance, and Skip path. |
| ~~FL-06~~ | Optional Xbox Broadcast setup | The setup explains Xbox Broadcast, warns against using a personal Microsoft/Xbox account, provides account guidance, and exposes a download action for the shared helper. | No Xbox Broadcast explanation, account guidance, or helper state is presented in the shown MSC 2 sequence. | Workflow; content; capability | Makes console-player discovery understandable while surfacing account and helper-install requirements. | MSC 1 user screenshot 6; MSC 2 user screenshots 12–18 (2026-08-24). | **DONE** — Added the optional Xbox Broadcast explanation, dedicated-account warning, Outlook link, real helper status, and download action. The agent now permits helper download before a server exists. |
| ~~FL-07~~ | Optional Tailscale check | The setup explains Tailscale’s remote-access role and offers an explicit check for whether it is already installed. | No Tailscale explanation or installation check is presented in the shown MSC 2 sequence. | Workflow; content; capability | Helps the operator evaluate a private remote-access option without requiring it. | MSC 1 user screenshot 7; MSC 2 user screenshots 12–18 (2026-08-24). | **DONE** — Added the optional Tailscale explanation, real agent-side installation probe, Check action, download link, and Skip path. |
| ~~FL-08~~ | Setup completion and handoff | A final “You’re All Set” screen summarises the selected server root, Java executable, and enabled server types, then hands the operator to first-server creation. | The shown MSC 2 flow moves from a one-button setup modal into a minimal Concept Guide and four-step tour; it provides no equivalent prerequisite summary or explicit first-server-creation handoff. | Workflow; information architecture | Gives the operator a confidence check and a clear next action instead of ending setup abruptly. | MSC 1 user screenshot 8; MSC 2 user screenshots 12–18 (2026-08-24). | **DONE** — Added the summary page and explicit Get Started handoff into the existing Concept Guide. |
| FL-09 | Post-setup teaching | After setup, MSC 1 starts an illustrated, animated “How MSC Works” walkthrough. The first shown concept teaches that a server needs a reachable address and contrasts port forwarding with a Playit.gg tunnel. | MSC 2 presents a short, text-only Concept Guide card (“One server. Your worlds.”) followed by a text-only guided tour. The shown content does not explain player connectivity, addresses, port forwarding, or tunnels. | Content; workflow; visual language | Teaches the networking mental model at the point it becomes relevant, using visual explanation rather than only settings text. | MSC 1 user screenshot 9; MSC 2 user screenshots 13–17 (2026-08-24). | Direct difference observed. |
| FL-10 | Onboarding handoff | The final “How MSC Works” slide teaches MSC’s server → world slots → active world model, then offers two deliberate next paths: start the guided tour or open the Server Handbook. | MSC 2 proceeds directly from the Concept Guide into the tour. The tour can be skipped, but no equivalent choice between tour and Handbook is presented; Handbook is only visible as a sidebar destination behind the modal. | Workflow; information architecture; content | Lets a new operator choose between an interactive orientation and a reference-oriented learning path. | MSC 1 user screenshot 10; MSC 2 user screenshots 13–14 (2026-08-24). | Direct difference observed. |
| FL-11 | Server Handbook presentation | The Server Handbook opens as a detailed visual reference: a Minecraft-scene banner, clear purpose statement, prominent getting-started handoff, searchable topic navigation, grouped topic catalogue, illustrated callouts, long-form detail, and topic progress/navigation. | Handbook is visible in the MSC 2 sidebar, but its contents were not opened in this walkthrough; it is not surfaced as part of the first-launch flow. | Information architecture; content; visual language | Makes extensive host-management guidance approachable and browsable rather than presenting it as an unstructured help page. | MSC 1 user screenshot 11; MSC 2 user screenshots 12–18 (2026-08-24). | Inspect the MSC 2 Handbook before making a visual-content parity finding. |

## Difference types

- **Information architecture** — navigation, grouping, or what is visible together.
- **Workflow** — actions, confirmations, feedback, and recovery.
- **Information density** — at-a-glance operational context.
- **Visual language** — hierarchy, color, spacing, typography, and iconography.
- **Content** — labels, terminology, help, and state copy.
- **Capability** — missing, added, deferred, or intentionally changed behavior.
