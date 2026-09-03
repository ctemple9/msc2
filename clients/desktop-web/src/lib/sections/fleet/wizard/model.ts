import type { Schema, ScreenApi } from '../../shared/types';
import { errorMessage, mutate } from '../../shared/types';
import { fleetMutationPaths } from '../model';
import { worldPaths, worldSettingsProfile, type WorldSettingsValues } from '../../worlds/model';
import { addonPaths } from '../../addons/model';

/** The three server-creation meanings exposed by the wizard's path picker. */
export type WizardPath = 'importExisting' | 'modpack' | 'fresh';

/**
 * Step-chip labels for the wizard's step counter. Fresh, existing-server
 * import, and modpack creation each walk their own sequence, sharing only
 * step 1 (Choose path) and the final step (Confirm). Fresh also inserts a
 * sixth Add-ons step for
 * Fresh/Java flavors with a plugin or mod ecosystem (`hasAddOnsStep`,
 * `showAddOns` here), between World and Confirm.
 */
export function wizardStepLabels(
  path: WizardPath,
  showAddOns = false,
  isModpackImport = false,
): readonly string[] {
  if (path === 'fresh') {
    const labels = ['Choose path', 'Configure', 'Network', 'World'];
    if (showAddOns) labels.push('Add-ons');
    labels.push('Confirm');
    return labels;
  }
  return path === 'modpack' || isModpackImport
    ? ['Choose path', 'Upload', 'Network', 'World', 'Confirm']
    : ['Choose path', 'Upload', 'Review', 'Network', 'Confirm'];
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
  /** Complete first-world profile collected by the shared World Settings
   *  form. The legacy fields above remain for compatibility with the
   *  existing create request and confirmation copy. */
  worldSettings?: WorldSettingsValues;
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
  /**
   * Individual add-on picks staged during the Add-ons step -- either a
   * Modrinth catalog pick (`PluginBrowserSheet.svelte`/`ProjectDetailSheet.svelte`
   * in `mode="stage"`) or a local `.jar` file staged via
   * `POST /v1/staged-uploads` (purpose `addon-local-file`). Mirrors
   * `AddServerWizardView.swift`'s `stagedAddOns`/`WizardStagedAddOn`, but
   * unlike the oracle (which downloads/copies files directly), nothing
   * installs until P12.18g's real create call, once the server these are
   * for actually exists -- each entry redeems through the same
   * `POST /v1/components/install` route `ProjectDetailSheet.svelte` and
   * `install_from_staged_local_jar` already use for an existing server,
   * just called once per pending item right after creation instead of
   * pre-create (that route hard-requires an active server; see
   * `AddOnsStep.svelte`'s own note).
   */
  pendingAddOns: PendingAddOn[];

  /**
   * Import path (P12.18h) -- `AddServerWizardView.swift`'s `sourceURL`/
   * `isSourceZip`, the real local path handed to `POST /v1/servers/import`
   * (`action: 'scan'` then `action: 'importExisting'`). `undefined` until
   * Upload picks or drops one.
   */
  importSourcePath: string | undefined;
  importIsZip: boolean;
  /** `AddServerWizardView.swift`'s `scannedInfo` -- the real scan result,
   *  set once Upload's scan call succeeds. Its own `serverType`/`port` are
   *  folded onto `serverType`/`javaPort`/`bedrockPort` above (not a separate
   *  `importPort` field), so `NetworkStep.svelte` needs no import-specific
   *  branch to reuse unchanged for this path, matching this step's own plan
   *  text. */
  importScan: Schema['ServerImportScanResponseDTO'] | undefined;
  /** `AddServerWizardView.swift`'s `selectedWorldName` -- defaults to the
   *  scan's own `defaultWorldName` the moment a scan succeeds; only
   *  overridden by Review's world picker (shown when the scan found more
   *  than one world). */
  importActiveWorldName: string | undefined;
  /** `AddServerWizardView.swift`'s `importMaxPlayers`/`importEulaAccepted`
   *  -- Review's editable overrides, pre-populated from the scan. */
  importMaxPlayers: number;
  importEulaAccepted: boolean;
}

/** One entry in `WizardDraft.pendingAddOns` -- see that field's own doc
 *  comment for how each kind gets redeemed. */
export type PendingAddOn =
  | {
      readonly id: string;
      readonly kind: 'catalog';
      readonly projectId: string;
      readonly slug: string | undefined;
      readonly title: string;
      readonly description: string | undefined;
      readonly author: string | undefined;
      readonly iconURL: string | undefined;
      /** `undefined` means "latest compatible version," matching
       *  `CatalogInstallRequestDTO.versionId`'s own optional semantics. */
      readonly versionId: string | undefined;
    }
  | {
      readonly id: string;
      readonly kind: 'localFile';
      readonly fileName: string;
      readonly stagedUploadId: string;
    };

/** Simple Voice Chat's published files and Modrinth listing all carry one of
 * these stable name fragments. This is only a creation hint; the agent still
 * checks the real plugins/ and mods/ directories before provisioning voice. */
export function isSimpleVoiceChatName(value: string | undefined): boolean {
  const name = value?.toLowerCase() ?? '';
  return (
    name.includes('simple voice chat') ||
    name.includes('simple-voice-chat') ||
    name.includes('voicechat') ||
    name.includes('voice-chat')
  );
}

export function hasStagedSimpleVoiceChat(draft: WizardDraft): boolean {
  return draft.pendingAddOns.some((addOn) =>
    isSimpleVoiceChatName(
      addOn.kind === 'catalog' ? `${addOn.title} ${addOn.slug ?? ''}` : addOn.fileName,
    ),
  );
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
    pendingAddOns: [],
    importSourcePath: undefined,
    importIsZip: false,
    importScan: undefined,
    importActiveWorldName: undefined,
    importMaxPlayers: 20,
    importEulaAccepted: false,
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

// ---------------------------------------------------------------------------
// Confirm step (P12.18g) -- the real POST /v1/servers/create call and the
// staged fields that can only be redeemed once the server (and its default
// world slot) is real. The first-world profile is sent in the create request
// itself. `pollOperation`/`operationPath`
// duplicate worlds/model.ts's and components/model.ts's own copies -- this
// codebase's established per-domain convention, not shared to avoid
// cross-domain coupling (see either file's own doc comment).
// ---------------------------------------------------------------------------

export const operationPath = (id: string): string => `/v1/operations/${id}`;

const OPERATION_POLL_MS = 900;

export async function pollOperation(
  api: ScreenApi | undefined,
  operationId: string,
  onTick?: (operation: Schema['OperationDTO']) => void,
  delayMs = OPERATION_POLL_MS,
): Promise<Schema['OperationDTO'] | undefined> {
  if (!api) return undefined;
  for (;;) {
    const operation = await api.get<Schema['OperationDTO']>(operationPath(operationId));
    onTick?.(operation);
    if (
      operation.state === 'succeeded' ||
      operation.state === 'failed' ||
      operation.state === 'cancelled'
    ) {
      return operation;
    }
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }
}

/** `AddServerWizardView.swift`'s `canCreate`. */
export function canCreateServer(displayName: string): boolean {
  return displayName.trim().length > 0;
}

/**
 * `AddServerWizardView.swift`'s `beginCreate`, Fresh branch: assembles the
 * real `POST /v1/servers/create` body from the accumulated draft.
 *
 * `worldSourceMode: 'backupZip'` is deliberately **not** represented here --
 * `run_create_server`'s own `NewServerRequest` (`crates/msc-agent/src/routes/
 * servers.rs`) always provisions `WorldSource::Fresh` no matter what this
 * body contains, since `ServerCreateRequestDTO` carries no world-source
 * field at all (confirmed against the frozen contract: `worldName`/
 * `worldSeed` are the only world-shape fields it takes). A staged backup can
 * only be redeemed *after* the server -- and its one default world slot --
 * actually exists; see `redeemStagedWorldBackup` below, called only once
 * this request's own operation has succeeded.
 */
export function buildServerCreateRequest(
  draft: WizardDraft,
  displayName: string,
): Record<string, unknown> {
  const name = displayName.trim() || draft.serverName.trim();
  const body: Record<string, unknown> = {
    name,
    serverType: draft.serverType,
    enablePlayit: draft.enablePlayit,
    enableXboxBroadcast: draft.enableXboxBroadcast,
    difficulty: draft.worldDifficulty,
    gamemode: draft.worldGamemode,
  };
  if (draft.serverType === 'java' && hasStagedSimpleVoiceChat(draft)) {
    body.enableVoiceChat = true;
  }
  const worldName = draft.worldName.trim();
  if (worldName) body.worldName = worldName;
  const worldSeed = draft.worldSeed.trim();
  if (worldSeed) body.worldSeed = worldSeed;
  if (draft.worldSettings) {
    body.worldSettings = worldSettingsProfile(
      draft.worldSettings,
      draft.serverType,
      draft.worldSettings.capabilities,
    );
  }
  if (draft.serverType === 'java') {
    body.javaFlavor = draft.javaFlavor;
    if (draft.versionId) body.versionId = draft.versionId;
    body.port = draft.javaPort;
    body.enableCrossPlay = draft.enableCrossPlay;
    if (draft.enableCrossPlay) body.crossPlayBedrockPort = draft.crossPlayBedrockPort;
    if (draft.stagedModpack) body.stagedModpackUploadId = draft.stagedModpack.stagedUploadId;
  } else {
    body.bedrockVersion = draft.bedrockVersion.trim() || 'LATEST';
    body.maxPlayers = draft.bedrockMaxPlayers;
    body.port = draft.bedrockPort;
  }
  return body;
}

/**
 * Redeems `WizardDraft.stagedWorldBackup` against the just-created server:
 * `POST /v1/worlds/import` (the same call `ImportWorldZipSheet.svelte`
 * already makes) lands it as a *new* world slot alongside the server's own
 * default fresh-created one, then `POST /v1/worlds/activate` makes it the
 * active world -- matching the oracle's `worldSource: .backupZip(url)`
 * being handed straight into `createNewServer` as the server's one and only
 * world. Both routes resolve "the active server" server-side with no
 * `serverId` field to pass; `finish_created_server` (`servers.rs`) already
 * selected the new server active once its own operation succeeded, so this
 * is safe to call immediately after that.
 */
export async function redeemStagedWorldBackup(
  api: ScreenApi | undefined,
  staged: NonNullable<WizardDraft['stagedWorldBackup']>,
): Promise<void> {
  const name = staged.fileName.replace(/\.zip$/i, '').trim() || 'Imported World';
  const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.import, {
    name,
    stagedUploadId: staged.stagedUploadId,
  });
  const updated = result.updated;
  const newSlot = updated?.slots.find((slot) => slot.id !== updated.activeSlotId);
  if (!newSlot) return;
  const activated = await mutate<Schema['WorldActivateResultDTO']>(api, worldPaths.activate, {
    slotId: newSlot.id,
  });
  if (activated.operationId) await pollOperation(api, activated.operationId);
}

/**
 * Redeems one `WizardDraft.pendingAddOns` entry via the same
 * `POST /v1/components/install` route `ProjectDetailSheet.svelte`/
 * `ComponentsSection.svelte` already call for an existing server --
 * `AddOnsStep.svelte`'s own note explains why this can't fire until now.
 */
export async function redeemPendingAddOn(
  api: ScreenApi | undefined,
  addOn: PendingAddOn,
): Promise<void> {
  const body =
    addOn.kind === 'catalog'
      ? {
          projectId: addOn.projectId,
          slug: addOn.slug,
          title: addOn.title,
          versionId: addOn.versionId,
        }
      : { stagedUploadId: addOn.stagedUploadId };
  const result = await mutate<Schema['CatalogInstallResultDTO']>(api, addonPaths.install, body);
  if (result.operationId) await pollOperation(api, result.operationId);
}

/** A pending add-on's display name, for a warning line if its redemption fails. */
export function pendingAddOnLabel(addOn: PendingAddOn): string {
  return addOn.kind === 'catalog' ? addOn.title : addOn.fileName;
}

/**
 * Orchestrates the whole real create: the durable `POST /v1/servers/create`
 * operation, then -- only once it has actually succeeded -- the staged
 * world backup and every pending add-on, each independently best-effort so
 * one failure doesn't hide the others. Mirrors the oracle's own two-phase
 * shape (`createNewServer` then `applyStagedAddOn` per staged item) without
 * its single in-process call, since this port's staged items redeem over
 * HTTP instead.
 */
export async function createServerFromDraft(
  api: ScreenApi | undefined,
  draft: WizardDraft,
  displayName: string,
  onProgress?: (statusLine: string) => void,
): Promise<{ warnings: string[] }> {
  const result = await mutate<Schema['ServerCreateResultDTO']>(
    api,
    fleetMutationPaths.create,
    buildServerCreateRequest(draft, displayName),
  );
  if (!result.operationId) {
    if (!result.success) throw new Error(result.message);
    return { warnings: [] };
  }
  const operation = await pollOperation(api, result.operationId, (tick) => {
    if (tick.statusLine) onProgress?.(tick.statusLine);
  });
  if (operation?.state !== 'succeeded') {
    throw new Error(operation?.error?.message ?? 'Failed to create server.');
  }

  const warnings: string[] = [];
  if (draft.worldSourceMode === 'backupZip' && draft.stagedWorldBackup) {
    try {
      await redeemStagedWorldBackup(api, draft.stagedWorldBackup);
    } catch (error) {
      warnings.push(`World backup: ${errorMessage(error)}`);
    }
  }
  for (const addOn of draft.pendingAddOns) {
    try {
      await redeemPendingAddOn(api, addOn);
    } catch (error) {
      warnings.push(`${pendingAddOnLabel(addOn)}: ${errorMessage(error)}`);
    }
  }
  return { warnings };
}

// ---------------------------------------------------------------------------
// Import path (P12.18h) -- Upload's real POST /v1/servers/import scan call,
// Review's always-advanceable gate, and the final action: "importExisting"
// call Confirm makes through the same `pollOperation` durable-operation
// shape `createServerFromDraft` already established for Fresh.
// ---------------------------------------------------------------------------

export type ImportScan = Schema['ServerImportScanResponseDTO'];

/**
 * `AddServerWizardView.swift`'s `performScan` -- the real, synchronous
 * `POST /v1/servers/import` (`action: 'scan'`) call. Callers assign the
 * result onto `draft` themselves inside their own `.svelte` script, the same
 * discipline every other cross-step mutation in this file already follows
 * (`createServerFromDraft` etc. never touch `draft` directly either) --
 * Svelte's classic (non-runes) reactivity only instruments assignments
 * written directly in a component, not ones made inside an imported plain
 * function.
 */
export async function scanImportSource(
  api: ScreenApi | undefined,
  sourcePath: string,
  isZip: boolean,
): Promise<ImportScan> {
  return mutate<ImportScan>(api, fleetMutationPaths.import, {
    action: 'scan',
    sourcePath,
    importKind: isZip ? 'zip' : 'folder',
  });
}

/** `AddServerWizardView.swift`'s `canAdvance` case 2, Import branch
 *  (`scannedInfo != nil && !isScanning && scanError == nil`) -- `isScanning`/
 *  `scanError` are `UploadStep.svelte`'s own local UI state (mirroring
 *  `WorldStep.svelte`'s identical `staging`/`stageError` precedent), so this
 *  only needs to check the durable half. */
export function canAdvanceUpload(draft: WizardDraft): boolean {
  return draft.importScan !== undefined || draft.stagedModpack !== undefined;
}

/** `AddServerWizardView.swift`'s displayName prefill on reaching Confirm for
 *  a non-modpack import (`advanceStep`'s own `currentStep == 3` branch):
 *  the source path's file/folder name, underscores turned to spaces,
 *  extension stripped. */
export function importDisplayNameFromPath(sourcePath: string): string {
  const base = sourcePath.split(/[\\/]/).filter(Boolean).pop() ?? sourcePath;
  return base.replace(/\.[^./\\]+$/, '').replace(/_/g, ' ');
}

/**
 * `AddServerWizardView.swift`'s `beginCreate`, Import branch: assembles the
 * real `POST /v1/servers/import` (`action: 'importExisting'`) body.
 *
 * **Real gap found and left alone, not silently worked around:**
 * `ServerImportRequestDTO` (the frozen contract) declares `enablePlayit`,
 * and this path's own Network step (`NetworkStep.svelte`, reused unchanged
 * per this step's own plan text) collects `draft.enablePlayit` exactly like
 * Fresh does -- but `import_raw`'s own `RawImportOverrides`
 * (`crates/msc-agent/src/routes/servers.rs`) only carries `port`/
 * `maxPlayers`/`activeWorldName`/`eulaAccepted`; `body.enable_playit` has no
 * reader anywhere in the import route (`import`/`import_raw`/
 * `run_raw_import`, confirmed by reading all three directly), unlike the
 * Fresh create path's own `NewServerRequest.enable_playit`, which really
 * does reach `provisioning::create_server`. Sent anyway, since the contract
 * declares the field and a future backend fix should pick it up
 * automatically -- but choosing Tunnel (playit.gg) on this path's Network
 * step has no effect today. A real backend gap, not a client one; left for
 * a dedicated follow-up rather than special-casing `NetworkStep.svelte` per
 * path, a component built explicitly to need no such branch.
 */
export function buildImportRequest(
  draft: WizardDraft,
  displayName: string,
): Record<string, unknown> {
  if (!draft.importSourcePath || !draft.importScan) {
    throw new Error('No server has been scanned yet.');
  }
  const body: Record<string, unknown> = {
    action: 'importExisting',
    sourcePath: draft.importSourcePath,
    importKind: draft.importIsZip ? 'zip' : 'folder',
    displayName: displayName.trim(),
    serverType: draft.serverType,
    port: draft.serverType === 'bedrock' ? draft.bedrockPort : draft.javaPort,
    maxPlayers: draft.importMaxPlayers,
    acceptEula: draft.importEulaAccepted,
    enablePlayit: draft.enablePlayit,
  };
  const activeWorld = draft.importActiveWorldName ?? draft.importScan.defaultWorldName;
  if (activeWorld) body.activeWorldName = activeWorld;
  return body;
}

/**
 * `AddServerWizardView.swift`'s `beginCreate`, Import branch: the real
 * `POST /v1/servers/import` call -- a durable operation (202, `operationId`
 * always populated per `ServerImportResultDTO`), the same `pollOperation`
 * shape Fresh's create already uses. Unlike Fresh, there is nothing to
 * redeem afterward: the scan already read every world this import brings
 * in, and `activeWorldName` picks among them directly in the one request.
 */
export async function importServerFromDraft(
  api: ScreenApi | undefined,
  draft: WizardDraft,
  displayName: string,
  onProgress?: (statusLine: string) => void,
): Promise<{ warnings: string[] }> {
  const result = await mutate<Schema['ServerImportResultDTO']>(
    api,
    fleetMutationPaths.import,
    buildImportRequest(draft, displayName),
  );
  const operation = await pollOperation(api, result.operationId, (tick) => {
    if (tick.statusLine) onProgress?.(tick.statusLine);
  });
  if (operation?.state !== 'succeeded') {
    throw new Error(operation?.error?.message ?? 'Failed to import server.');
  }
  return { warnings: [] };
}
