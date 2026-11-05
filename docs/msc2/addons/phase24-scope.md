# Phase 24 — Modpack management and content exports

**State:** planned; autonomous workflow owner-authorized, implementation not started.
**Execution:** [order and review checkpoints](phase24-execution.md).
**Owner direction:** discussion and explicit answers, 2026-11-05.
**Decision:** D-045. Implementation steps: [rolling plan](../rolling-plan.md).

## Agreed product contract

MSC manages server content and a separately editable player collection. One base
pack per server; individual additions remain possible. Items target Server,
Players, or Both; player content is Required or Optional. A player pack includes
what players need plus owner-selected optional content, not all server files.
Server-only plugins/mods stay out of Player exports. Client-only content stays
out of active server directories. Unknown classification is visible and reviewed.
Adding/updating a server mod checks player requirements and dependencies; MSC
asks about uncertain evidence rather than silently claiming completeness.

### Creation and discovery

Keep the current local pack creation/import flow. Also offer modpack discovery
in the existing add mods/plugins step, after Minecraft and loader selection.
Browse Modrinth and CurseForge; do not hide incompatible packs. Resolve release
compatibility, not just project tags. Default the release picker to matching
Minecraft and loader releases. Selecting a mismatch offers Try current setup
(with a warning and truthful installation result) or Change setup to match pack.
The second action updates Minecraft, loader and loader version automatically,
rechecks staged individual content, and reviews conflicts before creation.
Do not silently replace an explicitly chosen release or exceed MSC's supported
Minecraft/platform boundaries under Try anyway. No arbitrary scripts from a
server-pack archive are executed.

CurseForge is implemented wherever Modrinth currently participates: search,
project details, release selection, install, dependencies, identification/manual
linking, updates, modpacks and Java world datapacks, including creation and
existing-server screens and existing CLI/TUI equivalents. Preserve Hangar,
direct-file and existing Bedrock support. Provider feature differences are
explicit; unsupported categories must not be presented as working catalogs.
Use provider-qualified identities; do not interchange same-named projects or
substitute another provider's file without exact identity evidence and review.

### Retention, recovery and customization

Retain the original pack and full client content, including manifests, exact
versions, overrides, config, scripts, resource packs and shaders. Store retained
content separately from active server content. A published server-only pack
cannot prove the missing client pack: offer the corresponding client release
when provable, otherwise request the original or report missing content.

Persist upstream baseline, current owner state and published revisions
separately. Track ownership per path/hash, owner additions, edits and removals
(including intentional exclusions), provider identity and classification evidence.
Detect outside edits before update/publication. Preserve unknown config fields.
Existing servers are inspected non-destructively; identify installed content and
offer an original-pack import to recover missing player content without changing
the live server. Do not invent provenance or recover absent files by filename.

Update comparison uses old upstream, owner state and new upstream. Preserve
customizations by default; review conflicts with an explicit restore-upstream
choice per item. Never silently overwrite owner configuration or resurrect
removed items. Replacement reviews removal of old pack-owned files, retains
compatible owner additions and resolves conflicts. Worlds are outside both
operations; explain that switching packs may make an existing world incompatible.
Stage changes, journal operations, require stopped server when active content
changes, and preserve/recover the prior usable content on failure/cancellation.
Revalidate the review against changed files, host, server and permissions.

### Publication and export

Export pack opens a sheet with Player/Server, included categories/items, date,
revision, optional version name, estimated size, completeness, optional embedded
.mrpack and destination. Optional player content is included by default and
labeled; leave optional shaders/resource packs inactive where supported. Do not
promise that optional mod jars are disabled automatically by every launcher.

Publish creates an immutable snapshot with pinned bytes/hashes and metadata.
Date plus revision prevents same-day collisions. Current edits show Unpublished
changes. Export a previous publication without reading today's changed files;
re-export preserves its content, though ZIP timestamps need not be byte-identical.
Publish a new revision explicitly. Stream archives with bounded memory and disk
preflight; do not hold entire packs or base64 archives in API JSON.

The outer ZIP contains README.html, README.txt, offline instructions/images,
organized Minecraft content folders and a manifest. Include only used folders;
client config/scripts retain their required relative paths. Server datapacks
are world-local under D-030: record/select the world slot explicitly and package
with installation instructions, not a copy of the world. Server export is
content only: no runtime, Java, startup files, worlds, credentials, logs, player
lists, backups or unrelated server data. Configuration/scripts are reviewed
allowlisted content, not a recursive copy of config directories. Preview and
manifest explain exclusions and missing items.

Optional .mrpack represents the same revision and audience with exact Minecraft
and loader dependencies, hashes, sizes, environment flags and correct overrides.
Do not rename the generic ZIP to .mrpack. Use public provider downloads only
when compatible and authorized; permitted embedded overrides can carry local
files. Never embed an API key or authenticated download URL. Mixed-source
content may prevent a complete launcher-importable .mrpack; report why, offer
resolution or explicitly incomplete output, never silently change versions.
Server .mrpack export is offered only where the format can faithfully represent
the content; plugin-only bundles remain ordinary ZIPs with clear explanation.

Destination is This computer (including when paired remotely) or Selected host.
The former uses native local save/download; the latter uses agent-authorized
export destinations, not arbitrary remote filesystem writes. Review names the
computer and location. Protect against host switching, stale reviews, overwrite,
partial writes, disconnect and cancellation. No link publishing, public download
endpoint, player management credentials or TempleTech sharing backend.

Required unresolved content prevents a complete/ready-to-play publication;
explicitly incomplete exports remain possible with conspicuous missing-file
instructions. Unknown local files require review, not automatic prohibition.
Download authorization is not redistribution permission. Record distribution
eligibility/evidence for embedded bytes and respect author restrictions; manual
download does not itself authorize redistribution. No promise that every pack
can be shared fully offline. Owner-supplied file review never bypasses a known
restriction. No external links are needed to read the bundled guides; .mrpack
provider download URLs are technical metadata, not an invitation-link feature.

### Instructions and supported clients

Initial offline screenshot guides: Minecraft Launcher, Prism, Modrinth App,
CurseForge App and SKlauncher. Cameron supplies screenshots; do not fabricate
screenshots or mark guides accepted before they arrive. Record launcher version,
OS applicability and screenshot caption/alt text. Teach a separate installation,
exact game/loader setup, manual folder placement, optional content, safe updates
without mixing old jars, and verification before joining. .mrpack instructions
appear only for launchers/formats actually accepted. HTML has no remote assets
or executable scripts; text fallback remains useful. Server exports have their
own content-install instructions, not player launcher instructions.

Agent owns behavior. Desktop is the main visual flow; CLI exposes equivalent
catalog/content/review/publish/export actions, and host-local TUI provides the
interactive equivalent. Local terminal exports target that host; desktop can
save locally or remotely. Update capability discovery and matrix so old agents
refuse/degrade new actions clearly. Preserve supported Bedrock functionality;
this Java modpack feature does not invent Bedrock .mrpack support.

## Implementation anchors and investigation limits

Observed 2026-11-05 (inspection only):

- `msc-application/src/modpacks.rs` filters client-only imports; extend retention
  before classification drops them, including failed/manual-download paths.
- `msc-domain/src/modpack.rs`, `modpack_manifest.rs`, `app_config_schema.rs` own
  classification, manifest parsing and pack identity; individual mutation is
  already permitted, so do not reintroduce a pack-wide read-only guard.
- `msc-infrastructure/src/addon_provider.rs` already supplies Modrinth discovery,
  hash lookup and versions, CurseForge file/project resolution, secret-backed
  API key handling, and Bedrock-specific CurseForge browsing. Extend shared
  machinery rather than duplicating it or treating Bedrock search as Java search.
- `msc-application/src/{addons,addon_dependencies,addon_updates,add_on_inventory,
  client_export}.rs` and agent `routes/{components,worlds,servers}.rs` are affected.
- Desktop anchors: `fleet/wizard/{AddOnsStep,WorldStep,UploadStep,
  ModpackCreationSummarySheet}.svelte`, wizard `model.ts`,
  `components/{PluginBrowserSheet,ProjectDetailSheet,VersionPickerSheet,
  ImportModpackSheet,CurseForgeManualDownloadSheet}.svelte`,
  `addons/AddonsSection.svelte`, `worlds/WorldPackBrowserSheet.svelte`.
- Existing `/components/client-export` has links/ZIP behavior. New exports must
  replace user-facing sharing behavior without breaking old API clients silently.
- CLI is in `msc-agent/src/cli/mod.rs`; TUI has `components.rs`, `create.rs`,
  `worlds.rs`, `world_settings.rs`, `transfer.rs`. Native transfers live under
  desktop `src-tauri/src/native_transfer/`. Storage ownership is Phase 21.

These are anchors, not a completed line-by-line integration audit. P24.1 freezes
that inventory, precise API/error semantics and format limits before migrations.
New filenames in the steps are proposed; existing anchors retain their names.

## Sources and external prerequisites

Checked 2026-11-05; recheck provider/launcher contracts during P24.1:

- [Modrinth server/player content](https://modrinth.com/news/article/server-play/)
- [Official .mrpack format](https://support.modrinth.com/en/articles/8802351-modrinth-modpack-format-mrpack)
- [CurseForge API](https://docs.curseforge.com/rest-api/) — search, files,
  dependency relationships, fingerprints and distribution metadata.
- [CurseForge download authentication](https://blog.curseforge.com/introducing-api-key-authentication-for-curseforge-file-downloads/)
- [Distribution restrictions](https://support.curseforge.com/support/solutions/articles/9000207877)
- [Prism import](https://prismlauncher.org/wiki/help-pages/Zip-import/)
- [SKlauncher versions](https://docs.skmedix.pl/faq/launcher-related)

An approved usable CurseForge API key and disposable provider content are
prerequisites for Cameron's live verification. Reuse the host SecretStore;
do not add key proxying or assume a public application key can remain secret.
Screenshots are a prerequisite for P24.20 guide completion and final acceptance,
not for independent earlier steps. Provider limitations must be resolved/reported
in P24.1; do not narrow the owner's browsing scope silently.

## Verification and exit evidence

The owner amended the solo workflow on 2026-11-05: routine steps continue
autonomously in the execution protocol order, one commit per step, awaiting
owner verification. New sheets/views stop for runnable owner review. This is
a plan review and revision, not the independent phase-gate review.
No tests are authorized by this plan. Use focused non-test builds/type checks;
Rust changes also require formatting and relevant Clippy before committing.
Any essential new test must state its concrete failure, coverage gap, controlled
inputs and expected runtime before being added; Cameron alone authorizes its
specific execution. No CI, release gates, tags or publication work here.

Build/type-check evidence permits routine continuation, not owner acceptance.
Start with the current imported-pack baseline so future provenance tracking
does not mislabel existing files. Freeze snapshot content AND instruction asset
versions: adding screenshots later must not silently change old publications.
Package optional-but-inactive assets without modifying shared user options;
launcher-specific limitations are explained, not hidden.

The acceptance record will map each step to manual scenarios and observations:
compatible/incompatible creation, both mismatch choices, both providers and local
imports, mixed/local/unidentified/restricted/missing files, individual server
mods and dependencies, optional players, existing-server recovery, upstream
update/edit/removal conflicts, replacement, immutable re-export, incomplete
publication, ZIP and .mrpack installs, local/remote save and changed-host refusal,
world-slot datapacks, exclusions, restart/cancel/disk-full behavior, bounded
large-pack resource use, and macOS/Windows/Linux. Record actual launcher/OS
versions and screenshots. A clean build is not product acceptance. Cameron
verifies; the other agent reviews the phase gate when requested.
