# MSC 2 — Rolling Plan

**Current priority:** Phase 22 restores a host-local, full-screen TUI one owner-requested step at a time; its decision and phase record are P22.1 below. v0.1.38 is published. P16.136 adds a way to discard hidden first-start progress while keeping the generated world; owner verification is pending. P16.135 implements the World Packs and Datapacks card layout; owner verification is pending. The P18.99 Intel Bedrock decoder change is implemented but still awaits his same-world pre-render check. The other open work from Cameron’s macOS owner testing is listed below. Completed implementation records and release history are in [the archive](rolling-plan-archive.md#october-23-2026--implemented-owner-testing-findings-and-release-work). Owner checks are not implied by a DONE implementation status.

## Phase 22 — Restored full-screen terminal client

**Owner direction (2026-10-26):** Restore the redesigned TUI as a functional
client on macOS, Windows, and Linux. Begin with Manage Agents. Build it one
screen or bounded behavior at a time; there is no separate plan stage. Record
each completed slice as the next P22 step in this section during the same
conversation. Do not prewrite steps for later screens. The one-shot CLI remains
scriptable, and the TUI runs on the agent host in a local or SSH terminal.

**Starting point:** Cameron reviewed an interactive HTML design preview for
the terminal shell, server views, create-server flow, and Manage Agents. That
preview uses sample host data and is not production TUI code. Phase 22 turns
approved slices into a real Rust terminal client against the existing agent
API and local service controls. The first production slice is the Agents page.

**Phase-specific loop:** Cameron names the next screen or behavior. The agent
implements only that slice, adds its P22 record with Status, Files, What,
Verify, Batch, and Commit, makes one commit, and stops for Cameron's manual
verification. When Cameron asks for phase review, a different agent checks the
result. No up-front Plan/Read move, test run, or release gate is implied.

### P22.1 — Record the restored TUI direction and phase workflow

- **Status:** Documentation recorded; owner verification pending.
- **Files:** `docs/msc2/msc2-decisions.md`; `docs/msc2/MSC2-VISION.md`; `docs/msc2/msc2-product.md`; `docs/msc2/msc2-engineering.md`; `docs/msc2/msc2-port-plan.md`; `docs/msc2/rolling-plan.md`; `AGENTS.md`; `CLAUDE.md`.
- **What:** Add owner-approved D-043 to supersede the retired TUI decision, align the controlled vision set with the restored host-local client, and establish Phase 22's one-slice-at-a-time record without a plan stage. Preserve the existing one-shot CLI, local authorization boundary, and browser retirement.
- **Verify:** Run `rg -n 'D-043|Phase 22|P22\.1' docs/msc2/{msc2-decisions.md,MSC2-VISION.md,msc2-product.md,msc2-engineering.md,msc2-port-plan.md,rolling-plan.md} AGENTS.md CLAUDE.md` and confirm the restored TUI and no-plan-stage workflow are described consistently. No test command is part of this step.
- **Batch:** P22.1 only.
- **Commit:** `P22.1: record restored tui direction`

### P22.2 — Open the functional Manage Agents terminal screen

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/Cargo.toml`; `Cargo.lock`; `crates/msc-agent/src/cli/mod.rs`; `crates/msc-agent/src/cli/service.rs`; `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Add `msc tui` with a full-screen, keyboard-controlled Manage Agents page. Read the installed service and boot state through the existing platform service adapter; read configured servers through the same host-local authorized API as the one-shot CLI. Arrow keys choose an action, Enter executes it, letter keys choose an action, and Esc/q exits. Service start/stop and boot enable/disable reuse the existing platform behavior; stopping asks for confirmation. Show API unavailability honestly and keep the page usable when the service is stopped. Restore the terminal on exit. This first slice does not yet include server navigation.
- **Verify:** Run `cargo run -p msc-agent -- tui` in an interactive terminal. Confirm the service, boot, connection, and server information reflects this host; use Up/Down and Enter on Refresh; press Esc and confirm the shell returns normally. Service-changing actions are available for deliberate manual checking. No test command is part of this step.
- **Batch:** P22.2 only.
- **Commit:** `P22.2: add functional agents tui page`

### P22.3 — Match the Manage Agents terminal layout to the approved preview

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Replace the simplified single-column Agents screen with the preview's two-line banner, current-host sidebar, two-column Current Agent and Actions panels, and lower On This Agent and Another Computer panels. Keep real service, boot, connection, and server data. Show the actual server names and software without inventing a running state or version. Use this agent, Manage servers, sidebar Start/Manage, Help, and Settings are visible but report that their destination is not connected; no server action is performed. Service controls and refresh remain functional. The full layout needs at least a 100-by-24 terminal and tells the user when the window is smaller.
- **Verify:** Run `cargo run -p msc-agent -- tui` in a terminal at least 100 columns by 24 rows. Compare the banner, sidebar, four bordered sections, and five action rows with the approved preview. Confirm the host/server values are real, select Use this agent or Manage servers and press Enter to see the unavailable message, then use Refresh and Esc. No test command is part of this step.
- **Batch:** P22.3 only.
- **Commit:** `P22.3: complete agents tui layout`

### P22.4 — Manage the local agent's server selection

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Open a Manage Servers dialog from the Agents action or `m`, using the authenticated local `/v1/servers` list. Show the real active server from `/v1/status`, its known running state, and its name in the sidebar. Arrow keys move through the list without mouse scrolling; the visible window follows the selection. Enter selects that server through the same `/v1/active-server` route as `msc server use`, then refreshes the page. Esc closes the dialog. An empty agent gets an honest empty state; Transfer (`t`) and Create (`c`) remain visible but report that they are not connected yet. Do not invent a state for non-active servers.
- **Verify:** Run `cargo run -p msc-agent -- tui`, press `m`, and confirm the dialog reflects this host's actual server list or its empty state. Use Esc to close it; press `t` or `c` to confirm no operation occurs. If servers are configured, use Up/Down and Enter to select one, then check that the sidebar names it and `cargo run -p msc-agent -- server list` marks it active. No test command is part of this step.
- **Batch:** P22.4 only.
- **Commit:** `P22.4: add manage servers tui dialog`

### P22.5 — Complete the keyboard-driven create-server flow

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/create.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Connect Manage Servers' `c` action to a terminal wizard for fresh Java and Bedrock servers, modpack archives, and imports from local folders or ZIP files. Match the renderer's first page with two large side-by-side choices (“Import or Create from Modpack” and “Start Fresh”); the first opens a keyboard-driven import-versus-modpack choice. Keep the outer step counter and breadcrumb aligned with the renderer. Make Configure an inner sequence matching the renderer: server type, name, standard/modded category, software, source/version, and cross-play; Bedrock has its edition-specific version/player and broadcast pages. Show a visible loading message while Minecraft versions and Java runtimes are fetched, across Java software flavors and Bedrock version lookup, and show lookup failures in the wizard. On the Java source page, Left/Right chooses the source; Enter activates “Choose version…,” then Left/Right cycles Minecraft versions and Enter continues. Finish Java Configure with a Java selection step that lists every detected runtime, marks incompatible ones “Needs newer Java,” and requires an explicitly selected compatible runtime. Its `i` action opens an install step with Java 25/21/17/8 choices; Enter installs through the agent operation route, waits for completion, refreshes detection, and selects the newly installed compatible runtime. Split Network into the renderer's two keyboard-driven pages: select Port Forwarding or Tunnel (playit.gg), then edit the applicable ports. Wrap both connection-card descriptions to fit fully in the terminal cards. Show Java TCP plus Bedrock/Geyser UDP when Java cross-play is enabled, or Bedrock UDP for Bedrock servers, using local-port labels for tunnels. Keep every page usable with arrow keys, Space, `e`, and Enter, with all page fields visible in a 100-by-24 terminal. Read current Minecraft and Java runtime options from the agent, use the capability route to show and submit only world settings supported by the chosen edition/version, and poll durable operations. Allow recovery by checking an already-submitted operation again after a temporary status error, preventing a repeated create request. After creation, apply a selected first-world backup or local world pack/add-on and report follow-up failures without inviting a duplicate server. Keep Modrinth catalog browsing out of this terminal flow as requested; local files remain available. Imported servers expose port forwarding and explain that a tunnel can be enabled after import, because the import route does not apply tunnel settings. New Java server EULA acceptance remains in Settings after creation, matching the existing app flow.
- **Verify:** Run `cargo run -p msc-agent -- tui` in a terminal at least 100 columns by 24 rows. Open Manage Servers with `m`, press `c`, and confirm the first page shows two side-by-side cards matching the renderer. Choose Start Fresh and walk Configure's Java inner pages, checking the edition/name/category/software/source/cross-play layouts against the preview; on Source, select “Choose version…” with Left/Right, press Enter to activate version selection, cycle with Left/Right, and press Enter to continue. Confirm a loading message appears during option lookup, Java runtimes populate for supported Java software choices, and Bedrock versions populate for Bedrock. On Java selection, confirm detected runtimes and compatibility labels; select a compatible runtime, or open Install Java with `i`, choose a Java version, press Enter to install, and confirm MSC selects it after the operation completes. Also check Bedrock's edition-specific pages. On Network, verify Page 1/2 contains the full, uncut connection-card descriptions and Page 2/2 shows the right editable port fields: Java TCP, plus Bedrock/Geyser UDP when cross-play is on; Bedrock UDP for a Bedrock server. Confirm Tunnel changes those labels to local ports, Enter advances, and Esc returns to the prior wizard page. Choose “Import or Create from Modpack,” then use the arrow keys and Enter to choose Import existing or Modpack archive. Walk Fresh Java, Fresh Bedrock, Modpack archive, and Import existing through their confirmation pages. Confirm the selected server type/version, runtime, world capability context, import scan values, and keyboard-only navigation are visible. Then create controlled Java and Bedrock servers, import a test server folder or ZIP, and create from a test modpack archive; verify the new entries and report any post-create world/add-on warning. No test command is part of this step.
- **Batch:** P22.5 only.
- **Commit:** `P22.5: complete create server tui flow`

### P22.6 — Match the create-server World and Add-ons pages

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/create.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Split the first-world setup into the renderer's seven keyboard pages: world source, essentials, world generation basics, advanced generation, gameplay rules, rule text, and local datapacks/behavior packs. Keep the World step in the six-step breadcrumb while its page counter advances independently. New-world options use the agent's edition/version capabilities, Bedrock rules include cheats, coordinates, experiments, and starting map, and world packs are checked and installed after creation. The Java Gameplay Rules page exposes Add Rule and Edit as Text actions. Match the Add-ons page with a local `.jar` path and explain that Modrinth browsing is omitted. Preserve arrow-key field/choice controls, Enter to continue, and Esc to go back; no scrolling is introduced. Local pack and plugin paths can be typed and submitted; a file chooser and adding multiple plugin files are not connected. The imported-backup route remains a shorter source/path path.
- **Verify:** Run `cargo run -p msc-agent -- tui` in a terminal at least 100 columns by 24 rows. Open Manage Servers, start the Create flow, choose Start Fresh, then walk the World pages and Add-ons page. Confirm the World breadcrumb remains step 4, the inner counter reaches 7/7 for a new world, Java and Bedrock show their applicable settings, left/right changes choices, `e` edits text, `a` adds a Java rule, Enter advances, and Esc returns to the previous inner page. Confirm the datapack/behavior-pack path and local plugin `.jar` path appear on their own pages and Modrinth browsing is identified as omitted. No test command is part of this step.
- **Batch:** P22.6 only.
- **Commit:** `P22.6: align create wizard world pages`

### P22.7 — Let the rules editor continue to Datapacks

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/create.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Fix the World page 6/7 action so Enter clearly opens the game-rule editor, Enter saves the rule list, and the next Enter continues to Datapacks instead of reopening the editor. Change the heading, field label, and footer while editing or after saving so each state is visible. Pressing `e` after saving reopens the editor.
- **Verify:** Run `cargo run -p msc-agent -- tui`, navigate to World page 6/7, and press Enter. Confirm the heading and footer indicate rule editing; type a `rule=value` pair and press Enter to save. Confirm the saved state says Enter continues to Datapacks, then press Enter and verify page 7/7 appears. Press `e` to reopen editing and Esc to cancel. No test command is part of this step.
- **Batch:** P22.7 only.
- **Commit:** `P22.7: fix rules editor navigation`

### P22.8 — Add Skip actions to optional create pages

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/create.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Give World page 6/7, World page 7/7, and Add-ons a selectable Skip row. Up/Down moves between the edit/path row and Skip; Enter on Skip advances without opening an editor. Skipping the rule editor leaves any rules already chosen on the preceding gameplay page. Skipping a datapack/behavior pack or plugin clears a previously entered path so that file is not installed. Update the footer to describe the keyboard action.
- **Verify:** Run `cargo run -p msc-agent -- tui`, navigate to World page 6/7, press Down and Enter, and confirm page 7/7 opens without editing rules. On page 7/7, press Down and Enter to reach Add-ons. On Add-ons, press Down and Enter to reach Confirm. Go back and enter a pack path, choose Skip, then return to page 7/7 and confirm the path is empty. Enter a plugin path, choose Skip, and confirm the Confirm page says “Local add-on: None.” No test command is part of this step.
- **Batch:** P22.8 only.
- **Commit:** `P22.8: add skip actions to optional create pages`

### P22.9 — Let the first version lookup finish

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/transport.rs`; `crates/msc-agent/src/cli/tui/create.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Give the create wizard's Minecraft version lookup a 30-second response window so a cold provider request can finish on the first attempt. Keep ordinary CLI requests at their existing timeout and preserve the explicit timeout environment override.
- **Verify:** Run `cargo run -p msc-agent -- tui`, start a fresh Java Paper server, and advance from Configure page 4/7 to the Source page 5/7. Confirm the versions load on the first visit and Choose version can cycle through them without returning to page 4/7. No test command is part of this step.
- **Batch:** P22.9 only.
- **Commit:** `P22.9: allow cold version lookup to finish`

### P22.10 — Add the seven server workspace tabs

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Open the server workspace from Use this agent or a selected server. Add the seven approved tabs—Overview, Players, Worlds, Performance, Components, Settings, and Files—with 1–7 keyboard navigation, Agents and Manage navigation, and a real server summary on Overview. The remaining tab bodies and server controls identify their sections but are not connected yet.
- **Verify:** Run `cargo run -p msc-agent -- tui`, press Enter on Use this agent, and confirm all seven tab names are visible. Press 1–7 and confirm the active tab and heading change. Check that Overview shows the selected server's software, local port, running state, and notes. Press `a` to return to Agents, then `m` to open Manage Servers and select a server; confirm the workspace opens on Overview. No test command is part of this step.
- **Batch:** P22.10 only.
- **Commit:** `P22.10: add seven server workspace tabs`

### P22.11 — Match the Overview tab layout

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Arrange Overview like the approved renderer: Status with Connection Info and Live Stats cards, Activity with Players, Active World, and Chat cards, Notes, and a separate Console bar. Show a distinct empty state when no server is selected. Read available player count, active world, and performance values from the agent; show unavailable values honestly. Keep Chat and Console controls marked as upcoming work.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Overview with the created server, and compare its card layout to the renderer. Confirm the server name, software, local port, running state, notes, and any available live values reflect this host. Open the workspace without an active server and confirm its empty state. Resize the terminal below 40 rows and confirm Overview stays readable. No test command is part of this step.
- **Batch:** P22.11 only.
- **Commit:** `P22.11: match overview tab layout`

### P22.12 — Show live players in the Players tab

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Match the approved Players tab's sparse layout with an online count and roster. Read live names from the agent's players endpoint and saved profiles from its profiles endpoint. Show the preview's empty state when both are empty; identify known players separately when profiles exist because the API does not provide a session-by-session history. Keep the no-server and unavailable states honest.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server, and press `2`. Confirm the online count and player names match the agent's current roster; when no one is connected, confirm the empty message. If saved player profiles exist, confirm they appear under Known Players. Press `a`, return to a workspace without an active server, and confirm the no-server message. No test command is part of this step.
- **Batch:** P22.12 only.
- **Commit:** `P22.12: show live players in tui`

### P22.13 — Manage worlds and backups in the Worlds tab

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/worlds.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Show the selected server's saved world slots and active world, plus backups for that world. Enter opens world actions for rename, duplicate, and confirmed delete; Convert World and World Settings are visible placeholders for later pages. Use Tab to move to backups, Enter to restore or delete with confirmation, `b` to back up now, and `r` to toggle recurring backups. Show operation progress from the agent.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server, and press `3`. Confirm the world slot list and active label match MSC 2. Select a world with Enter and inspect all five actions; confirm Convert World and World Settings say their pages are coming later. Press Tab to see backups, use `b` for a manual backup, and confirm it appears after the operation finishes. Inspect the restore and delete confirmation prompts without accepting them. Check `r` changes the recurring backup state. No test command is part of this step.
- **Batch:** P22.13 only.
- **Commit:** `P22.13: add worlds and backups workspace`

### P22.14 — Show live performance in the Performance tab

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/worlds.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Match the renderer's sparse Performance tab with live TPS, CPU, memory, and online-player values from the selected server's performance endpoint. Show an em dash for unavailable metrics and a clear no-server state. Remove the View Map shortcut from Worlds because a terminal map is not part of the current world screen.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server, and press `4`. Confirm the four metric rows match available live values, and stopped or unavailable metrics show an em dash. Open the workspace without a server and confirm the no-server message. Press `3` and confirm Worlds no longer offers View Map. No test command is part of this step.
- **Batch:** P22.14 only.
- **Commit:** `P22.14: show live performance in tui`

### P22.15 — Add live terminal graphs to Performance

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/performance.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Keep the four numeric performance readings and add scrolling TPS and CPU history plus a terminal-character memory ring. Record bounded samples only while the TUI is open, reset history when the selected server changes, and show missing values honestly. Use the existing five-second agent snapshot refresh; no new agent metrics or history storage are introduced.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server, and press `4`. Confirm TPS and CPU graphs gain one sample about every five seconds and the memory ring percentage agrees with the used/limit numbers. Stop the server and confirm unavailable metrics show gaps or dashes. Select another server and confirm the graphs start fresh. Reopen the TUI and confirm history starts fresh. No test command is part of this step.
- **Batch:** P22.15 only.
- **Commit:** `P22.15: add live terminal performance graphs`

### P22.16 — Graph memory history instead of a ring

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/performance.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Replace the memory circle with a third scrolling graph that spans the Performance content width. Plot the agent's recent used-memory samples against its reported limit, or against the highest observed value when no limit is available; label that relative scale explicitly. Preserve the numeric memory reading and keep the graph visible in a 100-by-24 terminal.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server, and press `4`. Confirm TPS, CPU, and Memory each have a full-width history graph, the memory circle is gone, and the memory graph gains samples as the agent refreshes. Resize to 100 columns by 24 rows and confirm all three graphs remain above the Console line. No test command is part of this step.
- **Batch:** P22.16 only.
- **Commit:** `P22.16: graph memory history in tui`

### P22.17 — Manage server components in the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/components.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Show the selected server's software, applicable mods or plugins, and Xbox Broadcast in the Components tab. Read the agent's local inventories and shared broadcast JAR status. Enter opens contextual actions: select and change a supported server version while stopped; enable, disable, update, or confirm uninstall for an add-on; update Geyser/Floodgate through their managed helper route; and toggle Broadcast or download its JAR. Show agent errors when an add-on has no update source or a version change is unsupported, and follow durable operations to completion.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a Java server, and press `5`. Confirm the software and installed plugins or mods match MSC 2, then press Enter on each category to inspect its actions. For a stopped server, inspect the software version list; use Esc without changing it. Toggle a suitable test add-on off and on and confirm its state refreshes. Inspect Uninstall's confirmation and cancel with Esc. Confirm Xbox Broadcast shows whether its shared JAR is installed and offers the matching download/update action. Switch to a Bedrock server and confirm only applicable categories appear. No test command is part of this step.
- **Batch:** P22.17 only.
- **Commit:** `P22.17: manage components in tui`

### P22.18 — Fill the Performance view with responsive charts

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/performance.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Divide the space below the live readings and above Console evenly among TPS, CPU, and memory history. Draw multi-row terminal plots with scale labels when the terminal is tall, and collapse each to a one-line graph at the 100-by-24 minimum. Clear graph history when the server stops and show an explicit stopped message instead of rows of missing-sample dots.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a running server, and press `4`. At a tall terminal size, confirm three evenly spaced panels fill the Performance area and gain points on refresh. Resize to 100 columns by 24 rows and confirm all three compact plots remain above Console. Stop the server and confirm the graphs are replaced by a stopped message. No test command is part of this step.
- **Batch:** P22.18 only.
- **Commit:** `P22.18: fill performance view with responsive charts`

### P22.19 — Edit server settings in the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/mod.rs`; `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/settings.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Populate Settings from the selected server's agent-provided schema so Java and Bedrock show their own sections and fields. Support boolean, choice, number, and text edits with local bounds checks, report save and restart results, and honor the API's world-safety confirmation token.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a Java server, press `6`, and confirm the Runtime, Server, and Network sections and their current values. Toggle a safe setting and check it updates; edit a number and cancel with Esc. Switch to a Bedrock server and confirm its own sections and choices appear. No test command is part of this step.
- **Batch:** P22.19 only.
- **Commit:** `P22.19: edit server settings in tui`

### P22.20 — Browse and edit server files in the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/transport.rs`; `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/files.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Populate Files from the active server's authorized file-browse route, navigate folders with the keyboard, and launch local nano for previewable text files. Resolve each target and require it to remain inside the selected server directory before opening it; restore the full-screen terminal after nano exits. Show binary files as non-editable and report when nano cannot be launched.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server, and press `7`. Confirm its folders and files appear; open a folder with Enter and return with Backspace. Open a safe text file such as `server.properties` with Enter, make a small edit in nano, save and exit, and confirm the TUI returns and the file contains the edit. Confirm a `.jar` is shown but does not open in nano. No test command is part of this step.
- **Batch:** P22.20 only.
- **Commit:** `P22.20: browse and edit server files in tui`

### P22.21 — Edit world settings from the Worlds tab

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/worlds.rs`; `crates/msc-agent/src/cli/tui/world_settings.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Replace the World Settings placeholder with a slot-local TUI editor. Load the selected world's saved profile and edition/version capabilities, show applicable Essentials, World Generation, and Gameplay Rules fields, stage keyboard edits, and save them through the world profile route. Report the agent's apply status and request its world-safety acknowledgement when required.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server, press `3`, choose a world, then World Settings. Confirm its saved name, seed, difficulty, and game mode appear; move with Up/Down, change a safe value with Left/Right or Enter, and press `s` to save. Reopen the view to confirm readback. Check a Bedrock world to confirm Java-only fields are absent and Bedrock fields appear. Esc closes the view. No test command is part of this step.
- **Batch:** P22.21 only.
- **Commit:** `P22.21: edit world settings in tui`

### P22.22 — Edit selected server configuration in the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/server_editor.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Open a server editor after selecting a server from Manage Servers. General shows and edits display name, directory, RAM for Java, edition-appropriate local ports, and Java EULA acceptance; it also shows folder size. Services shows the selected server's Xbox Broadcast and Playit enablement for the upcoming service setup step. Java exposes executable and JVM flags using the existing host-wide Java configuration route and labels that scope explicitly. Bedrock omits Java-only controls. Validate numeric inputs and use the agent's existing routes for saves; keep the deeper service setup and initiation flow for later owner-requested steps.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage Servers with `m`, and select a Java server. Confirm the editor opens with General, Services, and Java tabs. Check name, directory, RAM, Java port, optional Geyser port, folder size, and EULA status; edit a safe field and confirm its value on reopen. Check that Services reports current Broadcast and Playit enablement and that Java labels its executable and flags as host-wide. Select a Bedrock server and confirm General omits RAM, Java port, and EULA, showing its Bedrock port instead, and that no Java tab appears. No test command is part of this step.
- **Batch:** P22.22 only.
- **Commit:** `P22.22: edit server configuration in tui`

### P22.23 — Detect and install Java runtimes in the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/server_editor.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Add Detected Java Runtimes and Install Java views to the selected Java server editor. Detect lists the host's runtimes and lets the owner select an executable through the existing host-wide Java configuration route. Install offers Java 25, 21, 17, and 8 through the agent's durable install operation, reports progress, and refreshes the editor after completion. These controls are absent for Bedrock servers.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage Servers with `m`, select a Java server, and press `3` for Java. Press `d` to inspect detected runtimes, use Up/Down and Enter to choose one, and reopen the editor to confirm the executable path. Press `i` to inspect the four install choices; Esc cancels. Optionally install a needed runtime and confirm progress and subsequent detection. Select a Bedrock server and confirm no Java tab appears. No test command is part of this step.
- **Batch:** P22.23 only.
- **Commit:** `P22.23: add java runtime views to tui`

### P22.24 — Control services and Xbox Broadcast sign-in in the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-api/src/dto/networking.rs`; `crates/msc-agent/src/routes/networking.rs`; `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/server_editor.rs`; `crates/msc-agent/tests/xbox_broadcast_routes.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Make Broadcast and Playit enablement selectable in the selected server's Services tab. Expose whether the host has saved Xbox Broadcast device sign-in. When it is missing, Setup enables and starts Broadcast and shows the agent's Microsoft link and device code; poll status and close the view after authentication. When sign-in is saved, Clear asks for explicit confirmation because it removes shared host credentials for all servers. Playit account setup remains the later dedicated Services slice.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage Servers with `m`, select a server, and press `2` for Services. Use Up/Down and Enter to toggle Broadcast and Playit, then reopen the editor to confirm each value. With Broadcast sign-in absent, select Setup and confirm the live Microsoft URL and code appear; complete sign-in and confirm the view closes automatically. Reopen Services to confirm Clear is offered. Confirm that choosing Clear requires `y` and that another key cancels. No test command is part of this step.
- **Batch:** P22.24 only.
- **Commit:** `P22.24: control services and broadcast sign-in in tui`

### P22.25 — Stop Xbox Broadcast when TUI setup is cancelled

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/server_editor.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Make Esc in the Xbox Broadcast device sign-in view stop the helper through the existing agent route, which also cancels an in-progress start. Restore the previous per-server enable setting when setup enabled Broadcast temporarily. Keep the sign-in view open if the stop request fails, and restore the previous setting when startup fails before sign-in. Label Esc as Stop setup.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server in Manage Servers, and open Services with `2`. Start Xbox Broadcast setup while sign-in is absent, then press Esc in the device-code view. Confirm the view closes with a stopped message and `cargo run -p msc-agent -- broadcast status` reports it is not running. If Broadcast was disabled before setup, confirm it is disabled again on reopening Services. Repeat with Broadcast previously enabled and confirm the setting remains enabled but the helper is stopped. No test command is part of this step.
- **Batch:** P22.25 only.
- **Commit:** `P22.25: stop broadcast on tui setup cancellation`

### P22.26 — Give TUI services their own control views

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/server_editor.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Show Xbox Broadcast and Playit as two selectable entries in the server editor's Services tab. Enter opens a service detail view. Broadcast detail reads running status, supports Start/Stop, per-server enablement, host-wide autostart, Microsoft sign-in setup or clear, and shared helper JAR download/update. Playit detail reads running and account status and supports Start/Stop and per-server enablement. Use the agent's existing routes and operation status for live progress; Playit account setup remains its later dedicated step.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage Servers with `m`, select a server, and press `2`. Confirm the Services tab lists only Broadcast and Playit with live state. Enter each detail view and use Up/Down and Enter to control enabled state and Start/Stop; verify status changes after refresh (`r`) or the automatic refresh. In Broadcast detail, toggle host autostart, inspect Setup/Clear and the JAR version, and use Update while stopped. Esc returns to the two-service list. No test command is part of this step.
- **Batch:** P22.26 only.
- **Commit:** `P22.26: add service control views to tui`

### P22.27 — Set up Playit from TUI Services

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/server_editor.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Add a masked email/password form to Playit detail, submit native setup through the agent, show operation progress and cancellation, then show completion and public addresses. Let the owner review existing setup and confirm a host-wide local reset. The password is dropped from TUI state after the setup request is accepted.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage Servers with `m`, select a server, press `2`, open Playit, and select Set up Playit. Enter account details and confirm the progress view advances to completion and displays reported addresses. Repeat and press Esc during progress to request cancellation. Reopen Playit setup to inspect the completed state; press `r` and confirm or cancel the host-wide reset prompt. No test command is part of this step.
- **Batch:** P22.27 only.
- **Commit:** `P22.27: add playit setup to tui`

### P22.28 — Initiate and control servers from the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/initiate.rs`; `crates/msc-agent/src/cli/tui/server_editor.rs`; `crates/msc-agent/src/routes/lifecycle.rs`; `crates/msc-agent/src/routes/networking.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Connect `s` in the selected-server workspace to Start, Stop, or the one-time Initiate view. Initiate accepts the Java EULA, follows the agent's first start operation and stopped state, offers Playit and Xbox Broadcast setup through the existing Services controls, lets either connection be deferred, then starts pass two and waits for its operation and stopped state before showing completion. Bedrock skips the EULA. A deferred Playit choice updates the active first-start coordinator, as Broadcast already does. If a stale first-start request becomes an ordinary launch because a world already exists, the TUI stops it and reports the recovery condition instead of sending a second uncontrolled Start. The agent remains the timing authority for readiness, helper timeouts (75 seconds for Playit and 60 for Broadcast), and the 10-minute second-pass safety cap; the TUI polls its state once a second and adds no competing timeout.
- **Verify:** Run `cargo run -p msc-agent -- tui`. Select a new Java server, press `s`, accept the EULA with Space, and press Enter. Confirm pass one creates files and stops before the connection choices appear. Set up or defer each enabled connection, press `c`, and confirm pass two starts, checks the connections, stops, and only then shows completion. Check a new Bedrock server omits the EULA. After initiation, press `s` to start and again to stop a server, and confirm the sidebar follows its state. No test command is part of this step.
- **Batch:** P22.28 only.
- **Commit:** `P22.28: add tui initiate and server controls`

### P22.29 — Connect the server sidebar and hidden initiation controls

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/initiate.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Replace the Agents page's “Start soon” label with the selected server's Initiate, Start, or Stop action and bind `s` to it on both the Agents and server pages. Keep agent service start/stop on its own selectable action. Esc hides an unfinished initiation without dropping its in-session progress; the sidebar then offers `i` to resume and `d` to discard. Discard stops a running server before calling the agent's first-start discard route, which keeps generated world data. Continue polling a hidden flow only while its server is selected.
- **Verify:** Run `cargo run -p msc-agent -- tui`. On Agents, select a fresh server and confirm the sidebar says Initiate; press `s` and confirm the EULA screen opens. Press Esc, confirm Resume and Discard appear on the sidebar, then press `i` and confirm the same initiation screen returns. Hide it again and press `d`; confirm the actions disappear and the existing world remains. For a server that has finished initiation, confirm the sidebar says Start when stopped and Stop when running, and `s` performs each action. No test command is part of this step.
- **Batch:** P22.29 only.
- **Commit:** `P22.29: connect server sidebar and hidden initiation`

### P22.30 — Keep initiation attached during a transient agent status loss

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/initiate.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Read the agent API even when the service manager briefly reports the service stopped. Keep the last selected server visible across a failed API read, mark the connection as reconnecting, and refuse ordinary server actions until the API responds. Let the initiation poll continue while its server remains selected, and open Playit or Xbox Broadcast setup from the initiation's own server record instead of a transiently empty screen snapshot. Host inspection during the reported failure found the launchd agent process still running and the server selected in saved configuration; the TUI had lost its read, not the server data.
- **Verify:** Run `cargo run -p msc-agent -- tui`, initiate a new server with Playit or Xbox Broadcast enabled, and press Enter on the enabled connection after pass one. Confirm its setup view opens. If the agent connection briefly drops, confirm the selected server name remains visible with a reconnecting state and the setup view remains reachable after reconnection. No test command is part of this step.
- **Batch:** P22.30 only.
- **Commit:** `P22.30: retain initiation context across agent status loss`

### P22.31 — Reuse local CLI credentials during Playit setup

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/transport.rs`; `crates/msc-agent/src/cli/tui/server_editor.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Cache the OS-authorized local CLI credential in process memory for four minutes, below the agent's five-minute lifetime, instead of issuing a new credential for every TUI poll and API read. On an unauthorized response after an agent restart, renew once and retry the request. Show Playit submission errors inside the setup form so they are not hidden behind the sheet. This addresses the observed 128-live-credential cap that prevented the Playit request from being sent; no account password is cached by the transport.
- **Verify:** After existing temporary credentials expire, run `cargo run -p msc-agent -- tui`, open Playit setup for an enabled server, enter the account details, and submit. Confirm the form advances to setup progress or shows a specific error inside the sheet instead of appearing stuck with the CLI credential-limit message in the background. Leave the TUI open for several minutes and confirm its agent connection remains usable. No test command is part of this step.
- **Batch:** P22.31 only.
- **Commit:** `P22.31: reuse cli credentials for tui polling`

### P22.32 — Make the Xbox Broadcast sign-in URL clickable

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/server_editor.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Render the fixed Microsoft device sign-in URL in the Broadcast setup sheet as an underlined OSC 8 terminal hyperlink. Supporting terminals can open it with their normal link click, including when the TUI runs over SSH; the visible URL remains available where hyperlinks are unsupported. Only the expected Microsoft URL receives terminal hyperlink control sequences, so a helper-provided string cannot inject terminal controls.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Xbox Broadcast sign-in, and click the underlined Microsoft URL using the terminal's link-click gesture. Confirm the Microsoft device sign-in page opens in the browser and the device code remains visible in the TUI. In a terminal without link support, confirm the URL is still readable. No test command is part of this step.
- **Batch:** P22.32 only.
- **Commit:** `P22.32: link xbox broadcast sign-in in tui`

### P22.33 — Correct the Playit service heading

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/server_editor.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Label the Playit service detail view “PLAYIT.GG” so its heading uses the service's proper name.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage Server → Services → Playit.gg, and confirm the heading reads “PLAYIT.GG”. No test command is part of this step.
- **Batch:** P22.33 only.
- **Commit:** `P22.33: correct playit service heading`

### P22.34 — Finish initiation after connection setup

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/initiate.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Match the renderer's automatic second pass once every enabled connection is configured or deferred. Refresh the completion sheet's connectivity data and show the available public address and Xbox Broadcast gamertag alongside the existing local and Playit addresses; omit rows for disabled or unavailable services.
- **Verify:** Run `cargo run -p msc-agent -- tui` and initiate a server with Playit and Xbox Broadcast enabled. After both setup flows complete, confirm the second pass begins without an extra Continue key, stops, and displays the applicable local, public, Playit, and Broadcast details. Also verify servers with a service disabled omit that service's row. No test command is part of this step.
- **Batch:** P22.34 only.
- **Commit:** `P22.34: finish initiate connection summary`

### P22.35 — Stop the second initiate pass when connections are ready

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/initiate.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Start the configured Playit helper for pass two, then have the TUI check the server's ready status and enabled helper addresses/authentication. Request a server stop once the required connections are ready, matching the renderer's initiate flow so the completion sheet appears without waiting for the safety cap.
- **Verify:** Run `cargo run -p msc-agent -- tui` and initiate a Java server with Playit and Xbox Broadcast enabled. Confirm pass two starts the required helper, then stops the server after the server and enabled connections report ready and shows the completion sheet. Also verify a disabled helper does not block the stop. No test command is part of this step.
- **Batch:** P22.35 only.
- **Commit:** `P22.35: stop initiate pass when ready`

### P22.36 — Reuse saved Broadcast sign-in during Initiate

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/initiate.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Treat host-saved Xbox Broadcast credentials as configured in Initiate, label them as host-configured, and prevent Enter from reopening helper setup when no sign-in is needed.
- **Verify:** On a host with saved Broadcast credentials, open Initiate for a server with Broadcast enabled. Confirm the connection row says it is configured on this host, Enter does not open another setup view or request a code, and the flow continues to pass two. On a host without saved credentials, confirm setup remains available. No test command is part of this step.
- **Batch:** P22.36 only.
- **Commit:** `P22.36: reuse saved broadcast sign-in in initiate`

### P22.37 — Align the initiate connection summary

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/initiate.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Lay out available connection methods in aligned label and address columns. Identify local connections as same Wi-Fi, public and Playit as usable from other networks, and show the Xbox Broadcast gamertag in the same value column. Omit unavailable methods.
- **Verify:** Run `cargo run -p msc-agent -- tui` and finish Initiate with local, Playit, public forwarding, and Xbox Broadcast available. Confirm method names align in the left column and addresses or gamertag align in the right column, with network-use hints; then check a server with fewer configured methods shows only applicable rows. No test command is part of this step.
- **Batch:** P22.37 only.
- **Commit:** `P22.37: align initiate connection summary`

### P22.38 — Detect Broadcast sign-in saved by another server

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/routes/networking.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Make Broadcast status report host sign-in as available when its cache is still stored under another server directory. This matches the existing migration behavior that copies a prior server's cache into the host-wide and newly selected server paths before launch.
- **Verify:** With a valid Broadcast cache under an existing server directory and no host-wide cache, open Initiate for a new server with Broadcast enabled. Confirm the status reports saved sign-in and Initiate treats it as configured without asking for another setup or device code. No test command is part of this step.
- **Batch:** P22.38 only.
- **Commit:** `P22.38: detect saved broadcast auth across servers`

### P22.39 — Show and collapse the selected server console

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Fetch the active server's console tail with workspace refreshes, show recent server output in the console panel, and let `c` collapse or expand it while keeping the running/stopped state visible.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select a server, start it, and confirm the console displays that server's live log lines. Press `c` to collapse and expand the feed, and confirm switching the active server shows its own console. No test command is part of this step.
- **Batch:** P22.39 only.
- **Commit:** `P22.39: show selected server console in tui`

### P22.40 — Show live readings in performance chart headings

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/performance.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Replace the descriptive hints in the TPS, CPU, and memory chart headings with the current reading, using the same units and formatting as the metric summary above.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Performance for a running server, and confirm each chart heading displays its current TPS, CPU percentage, or memory used and limit. Confirm missing metrics display an em dash and the memory scale remains visible on the graph. No test command is part of this step.
- **Batch:** P22.40 only.
- **Commit:** `P22.40: show live readings in performance chart headings`

### P22.41 — Add host-wide settings to the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/Cargo.toml`; `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/app_settings.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Open a General/Services overlay with Shift+S. General supports signed update checks and confirmed installation, host configuration reset, full host reset, and reviewed local uninstall. Services supports setting/clearing the agent's CurseForge API key and managing the shared Playit account, status, and saved addresses. Omit Preload tabs and defer Storage to the follow-up screenshots.
- **Verify:** Run `cargo run -p msc-agent -- tui`, press Shift+S, and confirm General and Services switch correctly. Check updates and confirm current/available/error states; on a staged release, confirm installation requires an explicit y confirmation. Set, replace, and clear a CurseForge key; set up Playit and confirm progress/status refresh, then clear it. Confirm both host reset modes require the exact `RESET AGENT` phrase and uninstall shows its preview and requires `UNINSTALL MSC 2`. No test command is part of this step.
- **Batch:** P22.41 only.
- **Commit:** `P22.41: add host-wide settings to tui`

### P22.42 — Add Storage navigation and reviewed actions to the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/app_settings.rs`; `crates/msc-agent/src/cli/tui/storage.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Add Settings section 3 with cached local computer inventory, explicit bounded scan and cancellation, paged inventory loading, category/server/group navigation, volume capacity and coverage details, local folder opening, and existing candidate/review/confirmation/result APIs for map cache, old downloads, obsolete builds, servers, worlds, and backups. The host-local TUI lists this computer only. Removal authority remains with existing services.
- **Verify:** Run `cargo run -p msc-agent -- tui`, press Shift+S then 3. Open This computer, refresh with r, navigate categories and Servers → server → section, inspect paths with Enter and open eligible local folders with o. Confirm protected candidates cannot be selected; inspect removal candidates using the displayed shortcuts, select with Space, review with Enter, and confirm only with the exact service phrase. Check cancellation with x and per-item cleanup outcomes. No tests were run.
- **Batch:** P22.42 only.
- **Commit:** `P22.42: add storage inventory and reviewed actions to tui`

### P22.43 — Correct Storage selection and macOS scan errors

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/app_settings.rs`; `crates/msc-infrastructure/src/source_tree.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Highlight the active Storage tab. Clear macOS errno before directory enumeration so a normal end of directory does not inherit an unrelated socket error and falsely mark inventory partial.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Shift+S → 3, confirm Storage is highlighted, press r to refresh and inspect coverage. Genuine unreadable sources remain reported. No tests run.
- **Batch:** P22.43 only.
- **Commit:** `P22.43: fix storage highlight and macos directory scan errors`

### P22.44 — Fix Storage rescan and expose removal rows

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/storage.rs`; `crates/msc-agent/src/cli/tui/app_settings.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Decode scan acceptance using its actual operationId response, retain the operation for polling, and display actionable review rows under Servers, Cache, and Agent. Correct Storage highlight position so the trailing character is not duplicated.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Shift+S → 3, press r and confirm scanning completes without a missing-state decode error. Open Servers and select Remove a server with Enter. Space selects eligible candidates; Enter opens the existing review and requires its exact confirmation. Esc cancels. Confirm Cache and Agent also show their applicable review rows. No tests run.
- **Batch:** P22.44 only.
- **Commit:** `P22.44: fix storage rescan and show removal actions`

### P22.45 — Skip IPC endpoints during storage measurement

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-infrastructure/src/source_tree.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Identify Unix socket, pipe, and device entries without opening them during storage traversal. Skip those non-file endpoints so socket-open errors do not terminate the directory inventory. Keep archive validation and symlink protections unchanged. Remove the redundant macOS errno reset introduced in P22.43; the original reset already existed.
- **Verify:** With the rebuilt background agent installed, open Storage and refresh. Confirm directories containing agent/WebKit IPC endpoints no longer produce socket-open error 102 and remaining ordinary files are measured. Genuine installation/capacity/permission coverage warnings remain visible. No tests run.
- **Batch:** P22.45 only.
- **Commit:** `P22.45: skip ipc endpoints during storage scans`

### P22.46 — Correct storage discovery coverage and service capacity

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/routes/storage/inventory.rs`; `crates/msc-infrastructure/src/storage/capacity.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Replace the preliminary desktop-not-inspected sentinel after successful native discovery while retaining actual native findings and failures. Fall back to filesystem available blocks when macOS important-usage capacity is unavailable or zero in a background service. This remains a dated availability observation, not reclaimable space.
- **Verify:** Install the prepared rebuilt app and Repair agent, then refresh TUI Storage. Confirm the redundant not-inspected warning is gone, filesystem capacity appears, and any actual source or discovery errors remain visible. No tests run.
- **Batch:** P22.46 only.
- **Commit:** `P22.46: correct storage coverage and capacity reporting`

### P22.47 — Make Storage folder and removal controls explicit

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/storage.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Label removal rows and shortcuts consistently, including Remove servers, Remove worlds, and Remove backups. Display candidate titles and empty-state feedback. Resolve server/group rows to their inventory directory for folder opening and report opener errors instead of discarding them. Keep server/world selection singular and require explicit candidate selection before review.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Storage → This computer → Servers, select a server and press o; confirm Finder opens its folder. Press t/w/b and confirm the matching Remove view appears, including a clear message for an empty backup list. Select an eligible candidate with Space and Enter to review; Esc cancels. No tests run.
- **Batch:** P22.47 only.
- **Commit:** `P22.47: clarify storage removal and repair folder controls`

### P22.48 — Remove duplicate server removal rows

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/storage.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Remove selectable Remove servers, Remove worlds, and Remove backups rows from Storage navigation. Retain their t/w/b shortcuts and footer labels.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Storage → This computer → Servers, confirm the three removal rows are absent and t/w/b still open their respective removal views. No tests run.
- **Batch:** P22.48 only.
- **Commit:** `P22.48: remove duplicate storage removal rows`

### P22.49 — Allow installed component inspection to finish

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Give component and local add-on inventory requests a 30-second response budget instead of five seconds. A read-only request confirmed the agent returns the installed Paper/Geyser/Floodgate versions with the longer budget. Preserve previously loaded inventory on a failed refresh only for the same selected server and show a retry message. Keep the offline add-on query.
- **Verify:** Run `cargo run -p msc-agent -- tui`, select test 9 and open Components. Confirm Paper 26.3/build 229 and installed plugin rows appear. Switch servers and confirm another server's inventory is never reused. Shift+R retries failed reads. No tests run.
- **Batch:** P22.49 only.
- **Commit:** `P22.49: allow component inventory requests to finish`

### P22.50 — Make removal confirmations typeable

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/storage.rs`; `crates/msc-agent/src/cli/tui/app_settings.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Display middle-dot and bullet separators as keyboard hyphens in removal confirmations. Require the exact displayed phrase and submit the original preview-bound API phrase. Route number keys to confirmation input instead of switching Settings sections. Show feedback on an incomplete phrase.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open a server removal review, and confirm the required phrase uses hyphens and accepts digits 1/2/3 without switching tabs. Type only part of the phrase and press Enter: removal must not submit. Esc cancels. Complete an intended removal only with the exact displayed phrase. No tests run.
- **Batch:** P22.50 only.
- **Commit:** `P22.50: make storage removal confirmations typeable`

### P22.51 — Add Manage Servers transfer controls

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/transfer.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Connect Manage Servers t to transfer import/export using the existing local CLI archive workflows. Support path entry, merge or replace-all import, required replacement backup path, and explicit confirmation. Existing archive validation, credential exclusion, and mutation permissions remain with the CLI/API. Confirm Shift+R reaches the reload path and add successful refresh feedback. Transfer command execution currently waits for the CLI to return.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage and press t. Export to a new archive path with e, finish editing with Enter, select Export and confirm y. Inspect the result. Import an intended transfer with i, choose merge or replace and provide a backup path for replace; confirm only when ready to modify the server set. Press Shift+R after closing to reload the server list. Confirm Shift+R refreshes the main workspace and reports successful completion. No tests run.
- **Batch:** P22.51 only.
- **Commit:** `P22.51: add manage servers transfer controls`

### P22.52 — Correct Manage Servers transfer hint

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Replace the stale Transfer soon footer with Transfer. Confirm the existing t handler opens TransferUi.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage Servers, confirm the footer says t Transfer and press t to open the transfer sheet. No tests run.
- **Batch:** P22.52 only.
- **Commit:** `P22.52: correct manage servers transfer hint`

### P22.53 — Separate transfer import and export views

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/transfer.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Begin with Export/Import choices. Export shows only save destination and Export. Import shows existing archive path, merge/replace explanation, and backup destination only for Replace All. Include full-path examples. Keep focus fixed during text editing; navigation skips hidden fields. Retain explicit transfer confirmation.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Manage → t. Choose Export and confirm only export fields appear. Esc returns to choices. Choose Import, toggle Merge/Replace, confirm backup path appears only for Replace and navigation skips hidden fields. Type a path and confirm arrow keys cannot switch fields while editing. No tests run.
- **Batch:** P22.53 only.
- **Commit:** `P22.53: separate import and export transfer views`

### P22.54 — Keep navigation responsive during snapshot refresh

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Run automatic snapshot reload and Shift+R reload in one bounded background task while continuing keyboard handling and drawing the last snapshot. Do not queue overlapping reloads. Guard publication with selected server identity and a generation advanced by explicit refreshes after mutations. Abort the task when the TUI exits. Initial loading, explicit mutation refreshes and sheet-specific requests retain their existing behavior.
- **Verify:** Run `cargo run -p msc-agent -- tui`. While the automatic refresh is inspecting components, switch Players/Overview/Performance and open/close Manage Servers; confirm keypresses respond without waiting for inspection. Press Shift+R and continue navigation while Refreshing is shown. Switch selected servers during a refresh and confirm late results never restore the previous server. No tests run.
- **Batch:** P22.54 only.
- **Commit:** `P22.54: refresh tui snapshots without blocking navigation`

### P22.55 — Show the desktop chat feed in Overview

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/chat.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Review desktop HomeSection, ChatCard and chatFeed parsing and mirror their console-tail source (200 lines), prefix removal, unsigned Java chat, advancement/challenge/goal, Java join/leave and Bedrock connection events. Render the latest three messages in chronological order with chat/event/advancement colors and meaningful empty/unavailable/stopped states. Ordinary console lines stay out of chat; Bedrock has connection events only. Replace both manual refresh pending messages with Refreshing…. No separate chat endpoint is introduced.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Overview for a running Java server, send an in-game chat message and trigger a join/leave or advancement; confirm matching desktop feed content after refresh. Confirm ordinary logs do not appear, switching server does not retain previous chat, Bedrock explains connection-only coverage, and stopped/empty states give a useful hint. Press Shift+R during an automatic refresh and confirm Refreshing… until completion. No tests run.
- **Batch:** P22.55 only.
- **Commit:** `P22.55: show selected server chat in overview`

### P22.56 — Cover console contents and edit server notes

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/notes.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Clear the expanded console rectangle before drawing so underlying Notes and other content cannot bleed through. Add Overview n to edit the selected server's notes, with multiline entry, Ctrl+S/F2 save via the existing server-ID-bound notes API, Esc cancel, and visible save errors. Invalidate pending snapshot publication during editing; update the saved server record locally. Show the first note line on Overview without emitting embedded newlines into its layout.
- **Verify:** Run `cargo run -p msc-agent -- tui`, expand an empty console with c and confirm Notes is covered. Collapse, press n on Overview, enter multiline notes and save with Ctrl+S or F2. Reopen notes and confirm persisted content; Esc cancels changes. Switch servers and confirm notes remain separate. No tests run.
- **Batch:** P22.56 only.
- **Commit:** `P22.56: cover expanded console and edit server notes`

### P22.57 — Bring Handbook and Router Guide into the TUI

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui.rs`; `crates/msc-agent/src/cli/tui/guides.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Review desktop HelpSection, HandbookBrowser, RouterGuidePanel, RouterGuideReader, RouterGuideTroubleshooting, catalog ordering, markdown behavior and help DTOs. Replace the ? placeholder with a two-pane guide overlay containing Handbook and Router Guide only. Read the same API catalogs and article/resolved-router content. Handbook includes desktop reading order, persisted TUI favorites, title search, related/internal-help links, history, previous/next chapters, structured prose/checklists/tables and scrolling. Router supports catalog/category browsing, generic fallback, server-side search/fallback results, runtime-resolved sections and unavailable-value warnings, intro/notes toggles, step selection/completion/all-reset, symptom selection, real analysis with causes/actions/escalation and suggested-guide navigation. Local completion state stays in this guide session. Keep Help out of other modal text inputs. Long text wraps within the reader.
- **Verify:** Run `cargo run -p msc-agent -- tui` and press ?. Browse Handbook, use Tab to focus the reader, scroll with arrows/PgUp/PgDn, search with / then Enter, favorite with f, open related topics with l, navigate chapters with brackets and history with b. Select 2 for Router Guide; browse/search and read a generic or named guide with a selected server. Check actual runtime values and missing-value warnings. Use j/k and Space for steps, x all/reset, i/n intro/notes, t for symptoms, Space to select, a to analyze, g to open the suggested guide, r to reset symptoms, and Esc to return. Confirm no onboarding view exists. No test command run; live navigation verification remains with Cameron.
- **Batch:** P22.57 only.
- **Commit:** `P22.57: add handbook and interactive router guides to tui`

### P22.58 — Space Components sections and expand long add-on lists

- **Status:** Implementation committed; owner verification pending.
- **Files:** `crates/msc-agent/src/cli/tui/components.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Add blank space between Server, Mods/Plugins, and Crossplay headings and rows. Limit the summary to the first ten add-ons; only when more exist, add See all with the total. Open a separate scrollable full add-on view with existing component actions and Esc back to summary. Preserve all original add-on indices for mutations.
- **Verify:** Run `cargo run -p msc-agent -- tui`, open Components for a server with a few plugins and confirm spacing. For a server with 11 or more add-ons, confirm only the first ten are in summary, See all appears, Enter opens the full scrollable list, actions target the selected add-on, and Esc returns to summary. No tests run.
- **Batch:** P22.58 only.
- **Commit:** `P22.58: space component sections and expand addon lists`

## macOS owner testing — 0.1.32

**Started:** October 21, 2026
**Artifact:** `/Users/camerontemple/Downloads/msc2-0.1.32-macos-x86_64.dmg`
**State:** In progress. Cameron is testing a clean install after uninstalling MSC 2 and clearing its remaining app data.

### Finding 5 — Xbox Broadcast does not explain where its JAR goes or what “shared” means

**What Cameron asked/experienced:** On the Xbox Broadcast page, the “Broadcast Helper” section says the tool is downloaded once and shared across all servers, and offers a **Download Now** button. It does not say where the file is stored, who publishes it, or whether “shared” means one copy is used by every server or each server gets its own JAR. Cameron wants to know what happens in a setup with three Java servers and one Bedrock server, all using Xbox Broadcast, and where the downloaded JAR comes from.

**Investigation:** MSC downloads the `MCXboxBroadcastStandalone.jar` asset from the latest release published by the [MCXboxBroadcast/Broadcaster project](https://github.com/MCXboxBroadcast/Broadcaster). MSC resolves the latest release through GitHub’s release API and verifies the asset using the upstream SHA-256 digest before caching it. The shared file lives below the agent data directory at `helpers/xbox-broadcast/<release tag>/MCXboxBroadcastStandalone.jar`. On macOS, the default agent data directory is `~/Library/Application Support/MSC2`; an installed agent can override this through `MSC2_DATA_DIR`, so the actual path may differ on a configured install.

The JAR is stored in the shared agent cache, not copied into each server directory. If a user skips **Download Now** and later enables Xbox Broadcast, MSC acquires the JAR as part of starting that server’s helper and puts it in the same shared cache; the server directory gets its `.msc2-broadcast` working/configuration folder, not another JAR. In Cameron’s example, all four server processes launch the cached JAR path. One implementation detail needs attention: the acquisition routine currently fetches the latest release asset on every acquisition, even when that version is already in the cache, then replaces the same cached path. So there is only one cached copy per release version, but the current code does not yet guarantee a one-time network download. **That means the setup download’s practical value today is only to confirm that the JAR can be fetched and verified early; it does not save the user a download or wait when they first start Xbox Broadcast, because startup fetches the asset again.** The page’s “downloaded once” sentence describes the shared storage model, but is too strong about actual download frequency and leaves the location unstated.

**Cameron’s corrected test sequence and filesystem check:** Cameron confirms he *did* select **Download Now** during first-time setup and saw the verified-download message, then created the Paper server with Xbox Broadcast enabled. The downloaded file is currently at `/Users/camerontemple/Library/Application Support/MSC 2/helpers/xbox-broadcast/159/MCXboxBroadcastStandalone.jar`. Paper has a per-server `.msc2-broadcast/config.yml`, but no copy of `MCXboxBroadcastStandalone.jar` in its server directory. When MSC starts the broadcast helper for Paper, it performs another acquisition and launches the JAR from the shared cache path; it does not move the setup download into Paper’s folder. The acquisition currently fetches the release asset again and replaces the file at that same versioned cache path.

**Cameron’s revised experience and desired behavior (2026-10-21):** Downloading Xbox Broadcast during First-time Setup gave Cameron a verified JAR, but creating his Paper server still downloaded the helper again. That makes the setup download feel disconnected from the server that later uses it. He wants one current Xbox Broadcast JAR managed by each MSC agent, whether the user downloads it during initial setup, while creating a server, or much later. Every server with Xbox Broadcast enabled should launch that same agent-owned JAR. Updating it from one server should update the agent’s JAR for every server. After a user signs in to Xbox Broadcast once on that agent, enabling it for another server should use the same Microsoft sign-in without asking for a new device code. A server created without Xbox Broadcast should have no Xbox Broadcast entry in its Components tab; enabling it later should make that entry appear. Cameron clarified that **the sign-in is shared**, not the entire server target configuration.

**Additional code investigation:** The current download and startup paths both use the agent’s versioned helper cache, but startup calls acquisition again; the acquisition code fetches the latest release and asset even when that version is cached. A server’s broadcaster launches from its own `.msc2-broadcast` working folder and writes a `config.yml` with that server’s target host, port, and display name. MSC already stores the alternate Microsoft account identity and password at agent scope, but the actual standalone helper runs in each server’s working folder; its sign-in state therefore does not automatically follow another server. MSC also has a per-server auth-token secret key. The Components tab currently renders an Xbox Broadcast row for eligible Java and Bedrock servers even when that server has not enabled it. The [upstream standalone instructions](https://github.com/MCXboxBroadcast/Broadcaster/blob/master/README.md) likewise place the target address in `config.yml` and launch the JAR from a working directory. Shared sign-in needs to account for the helper’s own cached authentication files; merely sharing the JAR or MSC’s displayed account name would not accomplish it.

**Updated user outcome required:**

1. The agent has one current, checksum-verified Xbox Broadcast JAR. All download entry points use that agent-owned installation. A valid installed JAR is reused on later starts without another asset download. An explicit update from any server replaces the agent’s current JAR safely, and every server then reports the same installed version. Temporary staging during a verified update is acceptable; failed updates must leave the working JAR usable. Account for running helper processes and Windows file locking before replacing a JAR they use.
2. Sign-in belongs to the agent. The first enabled server can complete the Microsoft device-code flow; subsequent enabled servers on that agent reuse the authenticated account. Keep each server’s target host, port, display name, enabled setting, and running broadcast process separate so changing one server does not redirect another. Determine how the upstream helper reads and refreshes its auth cache, then migrate existing per-server sign-ins without exposing tokens or silently forcing a new login. Verify shared sign-in when multiple enabled servers run at the same time before promising that case.
3. Show the Xbox Broadcast component only for a server that has Xbox Broadcast enabled, on eligible Java and Bedrock servers. The server creation and later server settings controls must still let someone enable it. Once enabled, the Components row reflects the one agent-owned JAR and offers an agent-wide download or update action; disabling the feature removes the row for that server.
4. Explain in setup that the JAR comes from the MCXboxBroadcast project, is stored with the agent, and is used by every enabled server. Explain that skipping the early download simply defers installation until the feature is enabled or started later. State clearly that Microsoft sign-in is shared on this agent while each server advertises its own destination.

**Implementation order:** First establish the agent-owned JAR and safe reuse/update path. Then move helper authentication to agent scope with migration from existing server folders while preserving per-server targets and processes. Finally, align setup, server creation, settings, and Components UI with the shared state. Cameron will verify both a clean setup and an existing signed-in installation, including a server enabled later and more than one running broadcaster. No release or test suite run is authorized by this design record.

**Shared-JAR implementation work (2026-10-21):** First-time Setup and the explicit download action still fetch and verify the current upstream JAR, but now record which release is current for the agent. Starting Xbox Broadcast reuses that verified cache entry without contacting GitHub again. Existing 0.1.32 caches have no selection record, so the agent discovers the most recent valid cached release rather than requiring a fresh download. The JAR status endpoint reports the selected, checksum-verified release. An explicit download updates that agent selection for all servers; it refuses the update while a broadcaster is starting or running, avoiding replacement of an in-use JAR, especially on Windows. A failed download leaves the previous selection in place. The agent still stores JARs in its shared helper cache rather than in server folders. Older cached release directories may remain on disk, but only one release is selected for starts. Agent-wide sign-in, existing-sign-in migration, and conditional Components visibility remain separate work under this finding.

The focused new test protects the concrete 0.1.32-to-next-build case Cameron encountered: after a setup download, starting while offline must reuse the verified file; the same must work for a legacy cache without a selection record, and damaged bytes must be rejected. It uses an in-memory filesystem and transport with no network or sleeps, and should run in milliseconds. It was added but not run, in keeping with the test policy.

**P16.101 — Define shared Xbox Broadcast behavior**

- **Status:** Design recorded; awaiting owner review before implementation.
- **Files:** `docs/msc2/rolling-plan.md`.
- **What:** Record the clarified agent-owned JAR and sign-in behavior, per-server targets and enablement, Components visibility, migration needs, and safe update requirements for Finding 5.
- **Verify:** Run `sed -n '99,145p' docs/msc2/rolling-plan.md` and review Finding 5 against the intended Xbox Broadcast experience.
- **Batch:** P16.101 only.

### Finding 9 — The Add Server success page is blank after creating a Paper server

**What Cameron experienced:** After creating his first Paper server, the Add Server wizard reaches its final **Confirm** step and shows a **Done** button, but the large page body is empty. Cameron expected the page to say that the server was created and explain what to do next. With only the progress row and **Done** visible, the result is unclear and the user has no visible confirmation or next-step guidance.

**Investigation:** The success state is present in `ConfirmStep.svelte`: when `createSucceeded` is true it should show “<server name> created,” followed by instructions based on the server’s world and software settings. `AddServerWizard.svelte` sets that state after its create operation succeeds, passes it to the confirmation component, and changes the footer action to **Done**. The screenshot shows the success-state footer but none of the expected body, so the visible behavior does not match the component’s intended success branch. The screenshot confirms the rendering defect; it does not establish why the success content failed to appear.

**User outcome required:** After successful creation, show a clear server-created confirmation and relevant next steps in the page body, with the server’s name visible. Investigate why that success content is absent in the tested build while the **Done** button is present, then verify the final page for Paper and other creation paths.

### Finding 10 — Playit setup claims an agent but stops before creating tunnels

**What Cameron experienced:** While connecting Playit for the first time, MSC said it could not start the Playit agent because the helper exited with code 1. Playit had already created an agent, but no tunnels appeared. The setup screen offered another sign-in attempt, which makes it look as though the account setup may have partly worked while leaving the server without a usable Java or Bedrock address.

**Investigation:** The installed MSC agent confirms that a Playit agent ID and key were saved, while the helper is stopped and both tunnel addresses are empty. In `PlayitAccountSetup::run_with_tunnels`, MSC first claims the cloud agent and saves its ID/key, then starts `playitd` and waits for that helper to confirm it connected to the same agent. MSC only lists or creates tunnels after that connection succeeds. So the observed “agent, but no tunnels” state follows the current order: the helper failed before tunnel provisioning began.

The installed macOS helper is the expected universal x86_64/arm64 binary, and its SHA-256 matches MSC’s pinned release digest. Running its help screen confirms it accepts both launch options MSC supplies (`--secret-path` and `--socket-path`), so the evidence does not point to a corrupt download or unsupported command-line arguments. I could not recover the helper’s output from this failed attempt. The setup error is built from the process exit status alone, even though the helper manager captures redacted output; the visible “code 1” does not identify whether Playit rejected the saved key, hit an account limit, or encountered another startup problem. The app’s current diagnostic endpoint also had no retained Playit lines for this attempt.

**Retest result:** Cameron deleted the failed agent from his Playit account and retried setup; it then worked. This confirms the flow can complete after removing that agent, but it does not establish why the first helper exited or whether deleting the agent itself fixed the underlying condition. The failure has not been reproduced, so treat the cause as unknown rather than a confirmed code defect.

**Follow-up if it recurs:** Preserve the user-facing error’s useful, redacted helper output so the actual startup reason can be identified. Do not report tunnel provisioning as the cause until the helper connection succeeds. No code change is currently justified by this one non-reproducible failure.

### Finding 15 — A new Bedrock world map has large holes despite a complete cache count

**What Cameron experienced:** While playing in a newly created Bedrock world, Cameron opened its 3D map and saw separated columns of trees and terrain with large empty gaps and sheer vertical walls between them. Some gaps are close to his live player marker even though he is standing on normal, rendered ground in Minecraft. The view did not visibly fill in as he waited. Clicking **Refresh terrain** changed the map, but it still looked patchy. **Pre-render map** reported “12 / 12 saved tiles cached” and “All saved terrain tiles are cached,” which made the missing ground especially confusing. On a newly created Paper server, joining the world produced a much more complete nearby map without this obvious pattern. Cameron expects Bedrock’s saved terrain around a player to appear with comparable coverage and responsiveness, or for MSC to explain accurately why it cannot yet show that terrain.

**Investigation (2026-10-21):** The Bedrock map reads a consistent *saved snapshot*, not the live terrain that the game client is drawing. The agent takes a Bedrock save hold/query/copy/resume snapshot when preparing the map, converts the saved LevelDB chunks into private Anvil region files, and keeps using that prepared generation until **Refresh terrain** captures another snapshot. The viewer polls for changes to the prepared map, but that polling does not capture a newer Bedrock save by itself. The pre-render count comes from the prepared renderer manifest's tile progress: it says all tiles *listed in that saved-terrain manifest* are cached, not that all terrain visible in Minecraft is present or that every saved chunk was converted. The first screenshot showed 12/12 tiles before the later, wider view.

The current private converted cache was inspected read-only. Its five Overworld region files contain 365 nonempty chunk positions in a sparse pattern, with missing positions among populated ones near Cameron's reported player location (approximately X -217, Z 120). Thus the conspicuous gaps are already present in the converted terrain input; waiting for those same tiles to draw will not fill them. The agent log shows initial conversion of four saved regions and later refreshes that rebuilt changed regions, so refresh is seeing changing saved input. This establishes that the count is misleading for coverage and that no automatic snapshot update is occurring. It does **not** yet establish whether the missing chunks are absent from the captured Bedrock LevelDB data or whether the conversion/indexing path omits saved chunks. The Paper comparison is Cameron's observed difference, not proof that Java captures live terrain automatically.

**User outcome required:** Compare chunk positions in a fresh, consistent Bedrock snapshot with the converted region files at the same location, particularly beside the player. If chunks are in the snapshot but not the converted output, repair conversion or indexing. If they are not in the snapshot, investigate why the Bedrock save boundary yields sparse terrain while the game shows ground, and improve snapshot freshness or the map's explanation accordingly. Make the map visibly distinguish “all known tiles cached” from “nearby saved terrain complete”; do not claim the latter from the pre-render count. Verify that newly explored Bedrock ground appears within a reasonable, stated interval or after an explicit refresh, and compare that behavior with Java on the same build. Preserve the source world and inspect snapshots read-only during diagnosis.

**P16.97 — Record the Bedrock map coverage finding**

- **Status:** DONE — investigation recorded; cause and fix remain open.
- **Files:** `docs/msc2/rolling-plan.md`.
- **What:** Describe the owner-visible gaps, the limited meaning of the 12/12 cache count, the sparse converted output observed locally, and the remaining source-versus-conversion question.
- **Verify:** In the current Bedrock world, compare the in-game ground near the player with the 3D map before and after **Refresh terrain**; the gap remains open until the map reliably covers saved nearby ground and its cache status accurately describes that coverage.
- **Batch:** P16.97 only.


### P16.130 — Archive implemented owner-testing work

**Status:** DONE — documentation cleanup recorded; owner review pending.
**Files:** `docs/msc2/rolling-plan.md`; `docs/msc2/rolling-plan-archive.md`.
**What:** Mark implemented steps DONE, archive their full findings and release records, and retain open findings in the rolling plan.
**Verify:** Run `rg -n "^### Finding|^### P16\.130" docs/msc2/rolling-plan.md` and confirm only Findings 5, 9, 10, and 15 remain, alongside this cleanup step. Check the archive for Findings 1–4, 6–8, 11–14, and 16–25 and their original Verify lines.
**Batch:** P16.130 only.
**Commit:** `P16.130: archive implemented owner-testing work`

### Finding 26 — Initiate tries to start Paper while it is already running

**What Cameron experienced:** On the Intel Mac, initiating a Paper server with Geyser, Floodgate, and Xbox Broadcast repeatedly ends with `illegal lifecycle transition: running -> starting`. Restarting the agent and trying three times did not clear it. The console shows Paper reached `Done` and Geyser started its UDP listener, so the diagnosis incorrectly says the server could not start. The JNA hardware-report warnings and Geyser's NetherNet warning are separate from this lifecycle error.

**Investigation:** The agent enters its coordinated first-start flow only while no world has been generated and the first-start popup has not been completed. An earlier attempt can leave a generated world while the popup remains incomplete. On the next attempt the agent treats Paper's start as ordinary and marks the operation successful at `Ready`, while the process remains running. The desktop sheet assumes every successful first pass stopped the process, moves to connection setup, then sends a second Start. The agent correctly rejects that transition from Running to Starting. This is independent of port forwarding; Xbox Broadcast may still need a reachable Bedrock target for outside players.

**P16.131 — Stop an ordinary first pass before connection setup**

- **Status:** Fix prepared; unsigned Intel candidate packaged; owner verification pending.
- **Files:** `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** When the agent's first Start succeeds without a coordinated first-pass completion marker, stop that running process and confirm it is stopped before connection setup and the second Start. Preserve the existing coordinated first-start path.
- **Verify:** Install `target/release-artifacts/MSC-2-intel-initiate-candidate.dmg` on the Intel Mac, repair the agent, then initiate the same Paper server whose world folder already exists. Confirm the first pass stops before the connection stage, the second pass does not report `running -> starting`, and the sheet reaches completion or reports a specific connection failure. Check the server is stopped at completion.
- **Batch:** P16.131 only.
- **Commit:** `P16.131: stop ordinary first pass before connection setup`

**Retest (2026-10-23):** Cameron installed the P16.131 Intel candidate and reproduced initiation failure on a brand new Paper server. The message changed to `illegal lifecycle transition: stopping -> stopping`; the server had begun loading Floodgate and Geyser. This disproves the earlier explanation as a complete diagnosis. The last 400 lines of `agent.log` contain map-renderer reuse and service startup messages but no lifecycle request sequence for the attempt. Code inspection identifies a separate duplicate-Stop path: first-start can request an automatic stop, while the desktop can request Stop before the first process exits. The Java stop route drains process events, then unconditionally calls `request_stop`; that method rejects a second transition to Stopping. The Bedrock stop path already treats Stopping as an in-progress success.

**P16.132 — Accept an in-progress Java server stop**

- **Status:** Fix prepared; unsigned Intel candidate packaged; owner verification pending.
- **Files:** `crates/msc-agent/src/routes/lifecycle.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Make the Java stop route accept a process that is already Stopping or Stopped after draining pending events. Keep the original graceful stop for Starting or Running, and let the first-start coordinator finish its existing operation. Add one assertion to the existing lifecycle route test for the repeated-stop case; it uses the same fake process and adds no timing or network dependency, with negligible added runtime. Do not run the test without owner instruction.
- **Verify:** Install `target/release-artifacts/MSC-2-intel-initiate-stop-candidate.dmg` on the other Mac, repair the agent, and initiate the Paper server again. Confirm the first run stops, the connection stage opens, and no `stopping -> stopping` or `running -> starting` error appears. Complete or defer Broadcast and confirm the second run stops with the setup sheet complete.
- **Batch:** P16.132 only.
- **Commit:** `P16.132: accept an in-progress java stop`

**P16.133 — Reuse Xbox Broadcast sign-in across servers on one agent**

- **Status:** Implementation prepared; unsigned Intel candidate packaged; owner verification pending.
- **Files:** `crates/msc-agent/src/routes/networking.rs`; `docs/msc2/rolling-plan.md`.
- **What:** On the first Broadcast start after upgrading, copy a single existing server sign-in into agent-owned storage, then seed each later server's standalone helper from it. Keep each server's target configuration and non-auth cache separate. Capture refreshed sign-in data while a helper runs. Clear the host and server copies when credentials are cleared. If multiple existing sign-in files differ, preserve them and report the conflict instead of selecting an account silently. The shared JAR acquisition path remains independent of when setup first downloaded it. Concurrent standalone processes may refresh the same Microsoft account independently; this build does not establish a single refresh owner.
- **Verify:** Install `target/release-artifacts/MSC-2-intel-broadcast-auth-candidate.dmg`, repair the agent, start Broadcast on the existing signed-in server, then initiate the second Broadcast-enabled server. Run `test -s "$HOME/Library/Application Support/MSC 2/helpers/xbox-broadcast/auth/cache.json"` on that Mac; it should exit successfully. Confirm the second server starts without another Microsoft device code and its target still points to itself. On a host that skipped the setup download, enabling Broadcast later should acquire the shared JAR before sign-in. Keep the original server's `.msc2-broadcast/cache` during this check.
- **Batch:** P16.133 only.
- **Commit:** `P16.133: reuse xbox broadcast sign-in across servers`

**Retest (2026-10-23):** Cameron installed the P16.133 candidate on the Intel Mac. Starting Broadcast for Test 2 reported that several existing sign-ins differ. This is a false conflict when the same Xbox account has separate refreshed token files: P16.133 compared entire cache files rather than account identity. No existing cache was deleted.

**P16.134 — Recognize the same Xbox account across refreshed caches**

- **Status:** Implementation prepared; unsigned Intel candidate packaged; owner verification pending.
- **Files:** `crates/msc-agent/src/routes/networking.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Read the stable `xboxUserProfile.id` saved by the broadcaster's MinecraftAuth library. During upgrade migration, accept multiple cache files for that account even when their tokens differ, and promote the newest file to agent storage. Keep rejecting a genuine account conflict or caches without enough identity evidence. Preserve every original per-server file.
- **Verify:** Install `target/release-artifacts/MSC-2-intel-broadcast-auth-migration-candidate.dmg`, repair the agent, then start Broadcast on Test 2. Confirm the differing-sign-ins error is gone and no Microsoft device code is requested. Run `test -s "$HOME/Library/Application Support/MSC 2/helpers/xbox-broadcast/auth/cache.json"` to confirm the host copy exists.
- **Batch:** P16.134 only.
- **Commit:** `P16.134: recognize matching xbox broadcast accounts`

**Owner check (2026-10-23):** Cameron reported that the P16.134 Intel candidate reused the Xbox Broadcast sign-in successfully on Test 2.

**P16.135 — Give each world pack and datapack its own action card**

- **Status:** Implementation committed; owner verification pending.
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Render each Bedrock world pack and Java datapack in its own card. Make the whole row open an action menu, put a chevron at the far end, and offer Enable/Disable and Delete for both editions. Keep Java's View action and the existing delete confirmation. Preserve the Bedrock pack summary and Required control above the cards.
- **Verify:** Open a Bedrock world with no packs and confirm the count, Browse Packs control, and empty state share one card. Open a Bedrock world with packs and a Java world with datapacks; confirm each pack has its own clickable card and trailing arrow. Open each menu and confirm Enable/Disable and Delete are present. Run `cd clients/desktop-web && npm run check`.
- **Batch:** P16.135 only.
- **Commit:** `P16.135: show world packs as action cards`

**P16.136 — Discard hidden first-start progress**

- **Status:** Implementation prepared; owner verification pending.
- **Files:** `clients/desktop-web/src/App.svelte`; `clients/desktop-web/src/lib/components/ApplicationShell.svelte`; `clients/desktop-web/src/lib/components/shell/ControlSidebar.svelte`; `clients/desktop-web/src/lib/sections/server-editor/model.ts`; `crates/msc-agent/src/main.rs`; `crates/msc-agent/src/routes/lifecycle.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Add a **Discard setup** action beside **Resume setup**. It stops the server when needed, clears the matching agent-owned first-start coordinator and networking helpers, and drops the desktop's saved sheet state. Keep generated world data. A later start cannot continue the discarded coordinator; if no world exists, Initiate opens a new sheet from its first screen.
- **Verify:** Hide first-start setup on a server that has completed pass one, choose **Discard setup**, and confirm the server stops, the Resume setup notice disappears, the existing world remains, and the sidebar shows Start. Click Start and confirm it performs an ordinary start. On a server with no world yet, confirm discarding leaves Initiate available and reopening it starts at the EULA screen. Do not run tests without a specific owner request.
- **Batch:** P16.136 only.
- **Commit:** `P16.136: discard hidden first-start progress`
