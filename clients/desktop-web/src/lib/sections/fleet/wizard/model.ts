import type { Schema } from '../../shared/types';

/** The two entry points `AddServerWizardView.swift`'s step 1 offers. */
export type WizardPath = 'importExisting' | 'fresh';

/**
 * Step-chip labels for the wizard's step counter, mirroring
 * `AddServerWizardView.swift`'s `stepLabel(_:)` -- Fresh and Import each walk
 * a different five-step sequence, sharing only step 1 (Choose path) and the
 * final step (Confirm). The oracle also inserts a sixth Add-ons step for
 * Fresh/Java flavors with a plugin or mod ecosystem (`hasAddOnsStep`,
 * `showAddOns` here), between World and Confirm.
 */
export function wizardStepLabels(path: WizardPath, showAddOns = false): readonly string[] {
  if (path === 'fresh') {
    const labels = ['Choose path', 'Configure', 'Network', 'World'];
    if (showAddOns) labels.push('Add-ons');
    labels.push('Confirm');
    return labels;
  }
  return ['Choose path', 'Upload', 'Review', 'Network', 'Confirm'];
}

// ---------------------------------------------------------------------------
// Configure step (P12.18b/c) -- draft state accumulated across the wizard.
// No mutation fires until P12.18g's real POST /v1/servers/create; every step
// past Choose Path only narrows this same draft object.
// ---------------------------------------------------------------------------

/** `AddServerWizardView.swift`'s `ServerType` picker -- Bedrock's own
 *  Configure fields land in P12.18c; this step only builds the selector. */
export type WizardServerType = 'java' | 'bedrock';

/** Ported from `JavaServerCategory.swift`. */
export type JavaCategory = 'standard' | 'modded';

/**
 * Ported from `JavaServerFlavor.swift`'s `CaseIterable` cases, restricted to
 * the ones `createFlowChoices` ever surfaces (`isAvailableInCreateFlow`
 * already excludes spigot/quilt/pufferfish there) -- those three are known
 * to the oracle's model but never reach its Create flow, so this port has no
 * use for them either.
 */
export type JavaFlavor = 'paper' | 'purpur' | 'vanilla' | 'fabric' | 'neoforge' | 'forge';

export interface JavaFlavorInfo {
  readonly id: JavaFlavor;
  readonly displayName: string;
  /** One-line purpose shown on the flavor card, ported verbatim from
   *  `JavaServerFlavor.swift`'s `shortDescription`. */
  readonly shortDescription: string;
  readonly category: JavaCategory;
  readonly isRecommended: boolean;
}

/** `JavaServerFlavor.swift`'s `allCases` declaration order, filtered to
 *  `isAvailableInCreateFlow`. */
export const JAVA_FLAVOR_CATALOG: readonly JavaFlavorInfo[] = [
  {
    id: 'paper',
    displayName: 'Paper',
    shortDescription: 'Performance and bug fixes; the standard plugin server',
    category: 'standard',
    isRecommended: true,
  },
  {
    id: 'purpur',
    displayName: 'Purpur',
    shortDescription: 'Paper plus hundreds of gameplay config options',
    category: 'standard',
    isRecommended: false,
  },
  {
    id: 'vanilla',
    displayName: 'Vanilla',
    shortDescription: "Mojang's unmodified server",
    category: 'standard',
    isRecommended: false,
  },
  {
    id: 'fabric',
    displayName: 'Fabric',
    shortDescription: 'Lightweight mod loader; great for performance mods',
    category: 'modded',
    isRecommended: true,
  },
  {
    id: 'neoforge',
    displayName: 'NeoForge',
    shortDescription: 'Heavyweight loader for big content modpacks',
    category: 'modded',
    isRecommended: false,
  },
  {
    id: 'forge',
    displayName: 'Forge',
    shortDescription: 'Original loader; huge library of mods and modpacks',
    category: 'modded',
    isRecommended: false,
  },
];

/**
 * Flavors whose provisioning is implemented today -- mirrors
 * `AddServerWizardView.swift`'s own `implementedFlavors` set (grows as later
 * milestones land; others render disabled with a "Soon" badge). Today it
 * equals `JAVA_FLAVOR_CATALOG` exactly, same as the oracle's own current
 * state, so the "Soon" badge is dead code in both places until a new flavor
 * is added to the catalog above without being added here too.
 */
const IMPLEMENTED_JAVA_FLAVORS = new Set<JavaFlavor>([
  'paper',
  'purpur',
  'vanilla',
  'fabric',
  'neoforge',
  'forge',
]);

export function isJavaFlavorImplemented(flavor: JavaFlavor): boolean {
  return IMPLEMENTED_JAVA_FLAVORS.has(flavor);
}

/** `JavaServerFlavor.swift`'s `AddOnKind` -- `plugin` installs to `plugins/`
 *  (server-side only), `mod` to `mods/` (server and every client). */
export type AddOnKind = 'plugin' | 'mod';

/** `JavaServerFlavor.swift`'s `addOnKind` -- Vanilla has no plugin/mod API
 *  (datapacks only) and returns `undefined`; standard-category flavors
 *  (Paper/Purpur) take plugins, modded-category flavors (Fabric/NeoForge/
 *  Forge) take mods. */
export function javaAddOnKind(flavor: JavaFlavor): AddOnKind | undefined {
  if (flavor === 'vanilla') return undefined;
  const info = JAVA_FLAVOR_CATALOG.find((entry) => entry.id === flavor);
  return info?.category === 'standard' ? 'plugin' : 'mod';
}

/** `AddServerWizardView.swift`'s `hasAddOnsStep` -- Fresh/Java servers whose
 *  flavor accepts add-ons get an extra wizard step between World and
 *  Confirm (always skippable once shown, matching the oracle's own
 *  `canAdvance` case 5). */
export function hasAddOnsStep(draft: WizardDraft): boolean {
  return draft.serverType === 'java' && javaAddOnKind(draft.javaFlavor) !== undefined;
}

export const JAVA_CATEGORY_INFO: Readonly<
  Record<JavaCategory, { displayName: string; subtitle: string }>
> = {
  standard: { displayName: 'Standard', subtitle: 'Players join normally · add plugins' },
  modded: { displayName: 'Modded', subtitle: 'Adds new content · players need the mods' },
};

/** `JavaServerFlavor.createFlowChoices(in:)` -- recommended flavor first,
 *  catalog order preserved otherwise (a stable sort, like Swift's). */
export function javaFlavorChoices(category: JavaCategory): readonly JavaFlavorInfo[] {
  return JAVA_FLAVOR_CATALOG.filter((flavor) => flavor.category === category).sort(
    (a, b) => Number(!a.isRecommended) - Number(!b.isRecommended),
  );
}

/** `AddServerWizardView.swift`'s `selectCategory(_:)` default-flavor pick. */
export function defaultFlavorForCategory(category: JavaCategory): JavaFlavor {
  const choices = javaFlavorChoices(category);
  return (choices.find((flavor) => isJavaFlavorImplemented(flavor.id)) ?? choices[0]).id;
}

/** Cross-play is unavailable for Modded (Bedrock can't load Java mods) and
 *  for Vanilla (no plugin API to host Geyser) -- `crossPlayUnavailable`. */
export function crossPlayUnavailable(category: JavaCategory, flavor: JavaFlavor): boolean {
  return category === 'modded' || flavor === 'vanilla';
}

/** `AddServerWizardView.swift`'s `FreshWorldSourceMode`, minus `.folder` --
 *  see `WizardDraft.worldSourceMode`'s own doc comment for why. */
export type WorldSourceMode = 'fresh' | 'backupZip';

/** `ServerDifficulty`'s raw values (`AppViewModelModels.swift`), which are
 *  also the exact wire strings `ServerCreateRequestDTO.difficulty` expects. */
export type WorldDifficulty = 'peaceful' | 'easy' | 'normal' | 'hard';

/** `ServerGamemode`'s raw values, Spectator excluded (oracle's own
 *  `.filter { $0 != .spectator }` on this specific picker). */
export type WorldGamemode = 'survival' | 'creative' | 'adventure';

export const WORLD_DIFFICULTY_OPTIONS: readonly { value: WorldDifficulty; label: string }[] = [
  { value: 'peaceful', label: 'Peaceful' },
  { value: 'easy', label: 'Easy' },
  { value: 'normal', label: 'Normal' },
  { value: 'hard', label: 'Hard' },
];

export const WORLD_GAMEMODE_OPTIONS: readonly { value: WorldGamemode; label: string }[] = [
  { value: 'survival', label: 'Survival' },
  { value: 'creative', label: 'Creative' },
  { value: 'adventure', label: 'Adventure' },
];

export interface WizardDraft {
  serverName: string;
  serverType: WizardServerType;
  javaCategory: JavaCategory;
  javaFlavor: JavaFlavor;
  /** `undefined` means "download latest" -- the oracle's own `nil` sentinel
   *  on `selectedVersionEntry`. Set only when the Source picker pins one. */
  versionId: string | undefined;
  enableCrossPlay: boolean;
  enableXboxBroadcast: boolean;
  /** `AddServerWizardView.swift`'s `bedrockVersion` -- free text, not a
   *  picker; see `BEDROCK_VERSION_NOTE`'s own doc comment for why. */
  bedrockVersion: string;
  /** `AddServerWizardView.swift`'s `bedrockMaxPlayers`; sent as a number,
   *  matching `ServerCreateRequestDTO.maxPlayers` and the agent's own
   *  `run_create_bedrock_server` range check (1-10000, default 10). */
  bedrockMaxPlayers: number;
  /** `AddServerWizardView.swift`'s `enablePlayit` -- Network step's
   *  Port Forwarding vs Tunnel(playit.gg) choice. */
  enablePlayit: boolean;
  /** `AddServerWizardView.swift`'s `javaPort` -- kept numeric (unlike the
   *  oracle's plain `String`) since `ServerCreateRequestDTO.port` and
   *  `crossPlayBedrockPort` are both numbers and `NumberField` already
   *  enforces a valid 1-65535 range, matching `settings/model.ts`'s own
   *  `server-port` field precedent. */
  javaPort: number;
  /** `AddServerWizardView.swift`'s `crossPlayBedrockPort` -- the Geyser port
   *  shown alongside `javaPort` only when cross-play is on. */
  crossPlayBedrockPort: number;
  /** `AddServerWizardView.swift`'s `bedrockPort` -- the standalone port for
   *  a Bedrock-serverType server (no Java port involved at all). */
  bedrockPort: number;
  /** `AddServerWizardView.swift`'s `FreshWorldSourceMode` -- New World or an
   *  external backup ZIP. The oracle's third case, an existing world
   *  *folder*, is not offered at all; see `WorldStep.svelte`'s own note for
   *  why (the same folder-to-archive gap `worlds/ReplaceWorldSheet.svelte`
   *  already found and dropped for the identical reason). */
  worldSourceMode: WorldSourceMode;
  /** `AddServerWizardView.swift`'s `initialWorldName` -- blank means "use
   *  the server name," resolved when P12.18g actually creates the server. */
  worldName: string;
  /** `AddServerWizardView.swift`'s `initialWorldDifficulty`. */
  worldDifficulty: WorldDifficulty;
  /** `AddServerWizardView.swift`'s `initialWorldGamemode` (Spectator
   *  excluded from the picker, matching the oracle's own `.filter`). */
  worldGamemode: WorldGamemode;
  /** `AddServerWizardView.swift`'s `initialWorldSeed`. */
  worldSeed: string;
  /** Set once "From backup (.zip)" has staged a file via
   *  `api.upload('world-import', ...)` -- the same staged-upload primitive
   *  `worlds/ImportWorldZipSheet.svelte` already uses. Held client-side
   *  only; nothing is redeemed until P12.18g's real create call exists to
   *  redeem it against. */
  stagedWorldBackup: { fileName: string; stagedUploadId: string } | undefined;
  /**
   * Set once the Add-ons step has staged and inspected a modpack archive via
   * `POST /v1/modpacks/inspect` (`addonPaths.inspectPack`) -- the same
   * staged-upload-then-inspect primitive `ImportModpackSheet.svelte` already
   * uses, stopped short of its own `POST /v1/modpacks/import` call (which
   * always targets an already-existing "active server" and would be wrong
   * here; see `AddOnsStep.svelte`'s own note). `stagedUploadId` carries
   * forward for P12.18g's real create call to redeem directly as
   * `ServerCreateRequestDTO.stagedModpackUploadId` -- the one field the
   * frozen contract actually offers for a pre-create staged pack. Only
   * offered for mod-kind flavors (Fabric/NeoForge/Forge); see
   * `AddOnsStep.svelte`'s own note for why plugin-kind flavors have no
   * pre-create equivalent at all.
   */
  stagedModpack:
    | {
        fileName: string;
        stagedUploadId: string;
        inspection: Schema['ModpackInspectionResultDTO'];
      }
    | undefined;
}

export function defaultWizardDraft(): WizardDraft {
  return {
    serverName: '',
    serverType: 'java',
    javaCategory: 'standard',
    javaFlavor: 'paper',
    versionId: undefined,
    enableCrossPlay: false,
    enableXboxBroadcast: false,
    bedrockVersion: 'LATEST',
    bedrockMaxPlayers: 10,
    enablePlayit: false,
    javaPort: 25565,
    crossPlayBedrockPort: 19132,
    bedrockPort: 19132,
    worldSourceMode: 'fresh',
    worldName: '',
    worldDifficulty: 'normal',
    worldGamemode: 'survival',
    worldSeed: '',
    stagedWorldBackup: undefined,
    stagedModpack: undefined,
  };
}

/**
 * `GET /v1/versions/create?serverType=bedrock` (`versions_for_create` in
 * `crates/msc-agent/src/routes/versions.rs`) always resolves to
 * `bedrock_versions_response(None, None)` for this query regardless of any
 * state -- it never looks at the active server, because there isn't one yet
 * during Configure. The response is a compile-time constant:
 * `supportsVersions: false`, empty `versions`, and this note. Calling it over
 * HTTP from this step would be a network round trip for a fixed string, so
 * this step renders the note directly instead and ports the oracle's own
 * free-text field (default "LATEST") rather than inventing a picker the real
 * route can never populate before the server exists.
 */
export const BEDROCK_VERSION_NOTE =
  'Bedrock versions are limited to the verified distribution selected for this runtime. Leave as LATEST unless you need a specific build.';

/** `AddServerWizardView.swift`'s `canAdvance` case 2, Fresh branch. */
export function canAdvanceConfigure(draft: WizardDraft): boolean {
  if (draft.serverName.trim().length === 0) return false;
  return draft.serverType === 'java' ? isJavaFlavorImplemented(draft.javaFlavor) : true;
}

/** `AddServerWizardView.swift`'s `canAdvance` case 3, Fresh branch: the
 *  server-type-appropriate port field must parse as a real port number.
 *  `NumberField` already constrains input to 1-65535, so this mostly guards
 *  against an emptied field, matching the oracle's own `Int(javaPort) != nil`
 *  / `Int(bedrockPort) != nil` check. */
export function canAdvanceNetwork(draft: WizardDraft): boolean {
  const port = draft.serverType === 'java' ? draft.javaPort : draft.bedrockPort;
  return Number.isInteger(port) && port >= 1 && port <= 65535;
}

/** `AddServerWizardView.swift`'s `canAdvance` case 4, Fresh branch. New
 *  World always advances; the backup-ZIP path needs a file already staged.
 *  There is no `folder` case here -- `WizardDraft.worldSourceMode` never
 *  takes that value. */
export function canAdvanceWorld(draft: WizardDraft): boolean {
  return draft.worldSourceMode === 'fresh' || draft.stagedWorldBackup !== undefined;
}

/** `GET /v1/versions/create?serverType=&javaFlavor=` (P7.24) -- the
 *  flavor-aware version list the Source row's "Choose version…" picker
 *  reads, independent of whether any server exists yet. */
export function versionsForCreatePath(
  serverType: WizardServerType,
  javaFlavor?: JavaFlavor,
): string {
  const params = new URLSearchParams({ serverType });
  if (serverType === 'java' && javaFlavor) params.set('javaFlavor', javaFlavor);
  return `/v1/versions/create?${params.toString()}`;
}

/**
 * Source picker's row label. For the four download-and-go flavors
 * (Paper/Purpur/Vanilla/Fabric), `displayLabel` alone (a bare Minecraft
 * version) is unambiguous. For NeoForge/Forge, `create_flow_choices`/
 * `neoforge_build_entries`/`forge_parse_maven_metadata`
 * (`server_versions.rs`) list every stable loader build, not just the
 * newest per Minecraft version, so several rows can share the exact same
 * `displayLabel` (e.g. several "26.2" entries, one per loader build) --
 * `buildLabel` (e.g. "NeoForge 26.2.15") is the only field that tells them
 * apart, so it's appended whenever the entry carries one.
 */
export function versionEntryLabel(entry: Schema['VersionEntryDTO']): string {
  return entry.buildLabel ? `${entry.displayLabel} · ${entry.buildLabel}` : entry.displayLabel;
}
