import type { Schema, ScreenApi } from '../shared/types';

// Real, frozen routes (Phase 6/7) -- see docs/msc2/worlds/phase6-api.md and
// crates/msc-agent/src/routes/{worlds,backups}.rs. P12.4k (2026-08-27)
// reverses P12.4's original call: Import ZIP / Replace World / Duplicate
// Slot -- `ServerEditorWorldTab.swift`'s three actions -- now live in this
// same Worlds tab instead of a World-shaped Server Editor sub-tab, so world
// behavior has exactly one home. `replaceActive` is
// `WorldReplaceActiveRequestDTO` (the live world, staged-upload-backed) --
// distinct from `/v1/worlds/replace`'s `WorldReplaceRequestDTO` (a saved-
// slot-to-saved-slot copy with no MSC 1 UI at all, still unused here).
export const worldPaths = {
  list: '/v1/worlds',
  create: '/v1/worlds/create',
  rename: '/v1/worlds/rename',
  delete: '/v1/worlds/delete',
  activate: '/v1/worlds/activate',
  saveCurrent: '/v1/worlds/update',
  repair: '/v1/worlds/repair',
  convert: '/v1/worlds/convert',
  convertFormats: '/v1/worlds/convert/formats',
  import: '/v1/worlds/import',
  replaceActive: '/v1/worlds/replace-active-world',
  duplicate: '/v1/worlds/duplicate',
  profile: (slotId: string): string => `/v1/worlds/${slotId}/profile`,
  thumbnail: (slotId: string): string => `/v1/worlds/${slotId}/thumbnail`,
} as const;

export const settingsPath = '/v1/settings';

export const backupPaths = {
  list: '/v1/backups',
  now: '/v1/backups/now',
  restore: '/v1/backups/restore',
  delete: '/v1/backups/delete',
  config: '/v1/backups/config',
} as const;

export const serversPath = '/v1/servers';
export const operationPath = (id: string): string => `/v1/operations/${id}`;

export type WorldServerType = 'java' | 'bedrock';

export type WorldProfileFieldMetadata = {
  capability: string;
  lifecycle: 'creation_only' | 'apply_on_activation' | 'live_safe' | 'restart_required' | string;
  valueState:
    'configured' | 'detected' | 'unknown' | 'unsupported' | 'achievement_disabled' | string;
  helpId?: string | null;
};

export type WorldSettingCapability = {
  capability: string;
  state: 'available' | 'unsupported' | 'unknown' | string;
  available: boolean;
  reason?: string | null;
  helpId?: string | null;
};

export type WorldSettingsCapabilities = {
  context: {
    serverType: WorldServerType | string;
    minecraftVersion?: string | null;
    javaFlavor?: string | null;
    loaderVersion?: string | null;
    javaRuntime?: {
      state: 'available' | 'unavailable' | 'unknown' | string;
      executablePath?: string | null;
      requiredMajor?: number | null;
      detectedMajor?: number | null;
      reason?: string | null;
    } | null;
    nativeCapabilities: string[];
  };
  fields: Record<string, WorldSettingCapability>;
  thirdParty: {
    available: boolean;
    label: string;
    message: string;
    handoff: string;
    helpId?: string | null;
  };
};

export type WorldProfile = {
  schemaVersion: number;
  identity: {
    name?: string | null;
    levelName?: string | null;
    seed?: string | null;
  };
  generation: {
    worldType?: string | null;
    flatPreset?: string | null;
    structures?: boolean | null;
    biomeSource?: string | null;
    generatorOptions?: string | null;
    bonusChest?: boolean | null;
    dataPacks: string[];
  };
  gameplay: {
    difficulty?: string | null;
    defaultGameMode?: string | null;
    hardcore?: boolean | null;
    commands?: boolean | null;
    gamerules: Record<string, string>;
    cheats?: boolean | null;
    experiments: Record<string, boolean>;
    coordinates?: boolean | null;
    startingMap?: boolean | null;
    supportedToggles: Record<string, boolean>;
  };
  safety: {
    state: string;
    reasons: string[];
  };
  fieldMetadata: Record<string, WorldProfileFieldMetadata>;
};

export type WorldSlotWithProfile = {
  slot: Schema['WorldSlotDTO'];
  profile: WorldProfile;
};

export type WorldProfileChange = {
  key: string;
  status: 'live' | 'pending_restart' | 'blocked' | string;
  reason?: string | null;
};

export type WorldProfileUpdateResult = {
  success: boolean;
  message: string;
  status: 'live' | 'pending_restart' | 'blocked' | string;
  slot: WorldSlotWithProfile;
  changes: WorldProfileChange[];
};

/** The display-friendly draft used by WorldSettingsForm in both creation
 * flows. Maps and arrays stay text-based in the form, then become the JSON
 * shapes the profile route accepts only when the user saves. */
export type WorldSettingsValues = {
  name: string;
  levelName: string;
  seed: string;
  worldType: string;
  flatPreset: string;
  structures: boolean | null;
  biomeSource: string;
  generatorOptions: string;
  bonusChest: boolean | null;
  dataPacks: string;
  difficulty: string;
  defaultGameMode: string;
  hardcore: boolean | null;
  commands: boolean | null;
  gamerules: string;
  cheats: boolean | null;
  experiments: string;
  coordinates: boolean | null;
  startingMap: boolean | null;
  supportedToggles: Record<string, boolean>;
  /** Runtime capability context is carried through the shared form only so
   *  profile writes can omit fields the agent did not advertise. It is not a
   *  persisted world-profile property. */
  capabilities?: WorldSettingsCapabilities;
};

export const WORLD_DIFFICULTY_OPTIONS: readonly { value: string; label: string }[] = [
  { value: 'peaceful', label: 'Peaceful' },
  { value: 'easy', label: 'Easy' },
  { value: 'normal', label: 'Normal' },
  { value: 'hard', label: 'Hard' },
];

export const WORLD_GAMEMODE_OPTIONS: readonly { value: string; label: string }[] = [
  { value: 'survival', label: 'Survival' },
  { value: 'creative', label: 'Creative' },
  { value: 'adventure', label: 'Adventure' },
  { value: 'spectator', label: 'Spectator' },
];

export const WORLD_TYPE_OPTIONS: readonly { value: string; label: string }[] = [
  { value: 'default', label: 'Default' },
  { value: 'flat', label: 'Flat' },
  { value: 'amplified', label: 'Amplified' },
  { value: 'large_biomes', label: 'Large biomes' },
  { value: 'single_biome_surface', label: 'Single biome' },
];

const JAVA_ONLY_PROFILE_FIELDS = new Set([
  'generation.flat-preset',
  'generation.biome-source',
  'generation.generator-options',
  'generation.data-packs',
  'gameplay.hardcore',
  'gameplay.commands',
]);

const BEDROCK_ONLY_PROFILE_FIELDS = new Set([
  'gameplay.cheats',
  'gameplay.experiments',
  'gameplay.coordinates',
  'gameplay.starting-map',
  'gameplay.supported-toggles',
]);

/** Creation-time server requests currently carry only the Essentials
 * projection. Keeping this list here makes the limitation visible in the
 * shared UI until the capability-aware create path lands. */
export const WIZARD_UNAVAILABLE_PROFILE_FIELDS = new Set([
  'identity.level-name',
  'generation.world-type',
  'generation.flat-preset',
  'generation.structures',
  'generation.biome-source',
  'generation.generator-options',
  'generation.bonus-chest',
  'generation.data-packs',
  'gameplay.hardcore',
  'gameplay.commands',
  'gameplay.gamerules',
  'gameplay.cheats',
  'gameplay.experiments',
  'gameplay.coordinates',
  'gameplay.starting-map',
  'gameplay.supported-toggles',
]);

export const CREATION_ONLY_PROFILE_FIELDS = new Set([
  'identity.seed',
  'generation.world-type',
  'generation.flat-preset',
  'generation.structures',
  'generation.biome-source',
  'generation.generator-options',
  'generation.bonus-chest',
  'gameplay.hardcore',
  'gameplay.commands',
  'gameplay.starting-map',
]);

export function defaultWorldSettingsValues(_serverType: WorldServerType): WorldSettingsValues {
  return {
    name: '',
    levelName: '',
    seed: '',
    worldType: 'default',
    flatPreset: '',
    structures: true,
    biomeSource: '',
    generatorOptions: '',
    bonusChest: false,
    dataPacks: '',
    difficulty: 'normal',
    defaultGameMode: 'survival',
    hardcore: false,
    commands: false,
    gamerules: '',
    cheats: false,
    experiments: '',
    coordinates: true,
    startingMap: false,
    supportedToggles: {},
  };
}

function mapToText(values: Record<string, string | boolean>): string {
  return Object.entries(values)
    .map(([key, value]) => `${key}=${String(value)}`)
    .join('\n');
}

function textToStringMap(value: string): Record<string, string> {
  return Object.fromEntries(
    value
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0 && line.includes('='))
      .map((line) => {
        const separator = line.indexOf('=');
        return [line.slice(0, separator).trim(), line.slice(separator + 1).trim()];
      })
      .filter(([key]) => key.length > 0),
  );
}

function textToBooleanMap(value: string): Record<string, boolean> {
  return Object.fromEntries(
    Object.entries(textToStringMap(value))
      .filter(([, raw]) => raw === 'true' || raw === 'false')
      .map(([key, raw]) => [key, raw === 'true']),
  );
}

function textToList(value: string): string[] {
  return value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function optionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function profileToWorldSettings(
  profile: WorldProfile,
  slot?: Schema['WorldSlotDTO'],
): WorldSettingsValues {
  return {
    name: profile.identity.name ?? slot?.name ?? '',
    levelName: profile.identity.levelName ?? '',
    seed: profile.identity.seed ?? slot?.worldSeed ?? '',
    worldType: profile.generation.worldType ?? '',
    flatPreset: profile.generation.flatPreset ?? '',
    structures: profile.generation.structures ?? null,
    biomeSource: profile.generation.biomeSource ?? '',
    generatorOptions: profile.generation.generatorOptions ?? '',
    bonusChest: profile.generation.bonusChest ?? null,
    dataPacks: profile.generation.dataPacks.join('\n'),
    difficulty: profile.gameplay.difficulty ?? '',
    defaultGameMode: profile.gameplay.defaultGameMode ?? '',
    hardcore: profile.gameplay.hardcore ?? null,
    commands: profile.gameplay.commands ?? null,
    gamerules: mapToText(profile.gameplay.gamerules),
    cheats: profile.gameplay.cheats ?? null,
    experiments: mapToText(profile.gameplay.experiments),
    coordinates: profile.gameplay.coordinates ?? null,
    startingMap: profile.gameplay.startingMap ?? null,
    supportedToggles: { ...profile.gameplay.supportedToggles },
  };
}

export function worldSettingsChanges(
  values: WorldSettingsValues,
  serverType?: WorldServerType,
  capabilities?: WorldSettingsCapabilities,
): Record<string, unknown> {
  const changes: Record<string, unknown> = {
    'identity.name': optionalText(values.name),
    'identity.level-name': optionalText(values.levelName),
    'identity.seed': optionalText(values.seed),
    'generation.world-type': optionalText(values.worldType),
    'generation.flat-preset': optionalText(values.flatPreset),
    'generation.structures': values.structures,
    'generation.biome-source': optionalText(values.biomeSource),
    'generation.generator-options': optionalText(values.generatorOptions),
    'generation.bonus-chest': values.bonusChest,
    'generation.data-packs': textToList(values.dataPacks),
    'gameplay.difficulty': optionalText(values.difficulty),
    'gameplay.default-game-mode': optionalText(values.defaultGameMode),
    'gameplay.hardcore': values.hardcore,
    'gameplay.commands': values.commands,
    'gameplay.gamerules': textToStringMap(values.gamerules),
    'gameplay.cheats': values.cheats,
    'gameplay.experiments': textToBooleanMap(values.experiments),
    'gameplay.coordinates': values.coordinates,
    'gameplay.starting-map': values.startingMap,
    'gameplay.supported-toggles': { ...values.supportedToggles },
  };

  // The profile schema is shared by Java and Bedrock, but the agent must not
  // receive keys belonging to the other edition. Leaving those keys in a
  // sparse update would turn an otherwise valid save into an unsupported
  // field error on the backend.
  if (serverType === 'java') {
    for (const key of BEDROCK_ONLY_PROFILE_FIELDS) delete changes[key];
  } else if (serverType === 'bedrock') {
    for (const key of JAVA_ONLY_PROFILE_FIELDS) delete changes[key];
  }

  const advertised = capabilities ?? values.capabilities;
  if (advertised) {
    for (const key of Object.keys(changes)) {
      if (!advertised.fields[key]?.available) delete changes[key];
    }
  }
  return changes;
}

export function diffWorldSettings(
  before: WorldSettingsValues,
  after: WorldSettingsValues,
  serverType?: WorldServerType,
  capabilities?: WorldSettingsCapabilities,
): Record<string, unknown> {
  const previous = worldSettingsChanges(before, serverType, capabilities);
  const next = worldSettingsChanges(after, serverType, capabilities);
  return Object.fromEntries(
    Object.entries(next).filter(
      ([key, value]) => JSON.stringify(value) !== JSON.stringify(previous[key]),
    ),
  );
}

export function profileFieldIsUnavailable(
  key: string,
  serverType: WorldServerType,
  mode: 'wizard' | 'create' | 'edit',
  metadata: Record<string, WorldProfileFieldMetadata>,
  capabilities?: WorldSettingsCapabilities,
): boolean {
  if (mode === 'wizard' && WIZARD_UNAVAILABLE_PROFILE_FIELDS.has(key)) return true;
  if (capabilities && !capabilities.fields[key]?.available) return true;
  if (metadata[key]?.valueState === 'unsupported') return true;
  if (serverType === 'java' && BEDROCK_ONLY_PROFILE_FIELDS.has(key)) return true;
  if (serverType === 'bedrock' && JAVA_ONLY_PROFILE_FIELDS.has(key)) return true;
  return false;
}

export function profileFieldUnavailableReason(
  key: string,
  serverType: WorldServerType,
  mode: 'wizard' | 'create' | 'edit',
  metadata: Record<string, WorldProfileFieldMetadata>,
  capabilities?: WorldSettingsCapabilities,
): string | undefined {
  if (mode === 'wizard' && WIZARD_UNAVAILABLE_PROFILE_FIELDS.has(key)) {
    return 'Available after the server is created.';
  }
  if (capabilities && !capabilities.fields[key]?.available) {
    return capabilities.fields[key]?.reason ?? 'This setting was not advertised by the selected runtime.';
  }
  if (metadata[key]?.valueState === 'unsupported') return 'Unavailable for this server type.';
  if (serverType === 'java' && BEDROCK_ONLY_PROFILE_FIELDS.has(key)) {
    return 'Unavailable for Java servers.';
  }
  if (serverType === 'bedrock' && JAVA_ONLY_PROFILE_FIELDS.has(key)) {
    return 'Unavailable for Bedrock servers.';
  }
  return undefined;
}

export function profileFieldIsReadOnly(
  key: string,
  mode: 'wizard' | 'create' | 'edit',
  metadata: Record<string, WorldProfileFieldMetadata>,
): boolean {
  return (
    mode === 'edit' &&
    (metadata[key]?.lifecycle === 'creation_only' || CREATION_ONLY_PROFILE_FIELDS.has(key))
  );
}

export const demoSlots: Schema['WorldSlotDTO'][] = [
  {
    id: 'slot-1',
    name: 'Overworld',
    isActive: true,
    createdAt: '2026-08-20T12:00:00Z',
    zipSizeBytes: 1024 ** 3,
    worldSeed: '8412552538448335604',
    hasThumbnail: false,
  },
  {
    id: 'slot-2',
    name: 'Before the Nether trip',
    isActive: false,
    createdAt: '2026-08-18T09:00:00Z',
    zipSizeBytes: 480 * 1024 ** 2,
    hasThumbnail: false,
  },
];

export const demoBackups: Schema['BackupItemDTO'][] = [
  {
    id: 'world-2026-08-24-103000.zip',
    displayName: 'Overworld backup',
    isAutomatic: false,
    triggerReason: 'manual',
    modificationDate: '2026-08-24T10:30:00Z',
    fileSize: 768 * 1024 ** 2,
    slotId: 'slot-1',
    slotName: 'Overworld',
  },
];

/** `GET /v1/worlds/{slotId}/thumbnail` only has bytes to serve once the slot
 *  actually carries one; everything else falls back to the same deterministic
 *  gradient placeholder ActiveWorldCard.svelte already uses on Overview. */
export function slotThumbnailUrl(slot: Schema['WorldSlotDTO']): string | undefined {
  return slot.hasThumbnail ? worldPaths.thumbnail(slot.id) : undefined;
}

export function placeholderHue(seed: string): number {
  let h = 0;
  for (let i = 0; i < seed.length; i += 1) h = (h * 31 + seed.charCodeAt(i)) % 360;
  return h;
}

export function backupsForSlot(
  backups: readonly Schema['BackupItemDTO'][],
  slotId: string | undefined,
): Schema['BackupItemDTO'][] {
  if (!slotId) return [];
  return backups.filter((backup) => backup.slotId === slotId);
}

export type BackupDay = { day: string; items: Schema['BackupItemDTO'][] };

/** Groups by calendar day, most recent first -- same shape as
 *  players-online/model.ts's groupSessionEventsByDay (no MSC 1
 *  Today/Yesterday special-casing; P12.3's SessionLogCard already
 *  simplified that away and this screen follows the same precedent). */
export function groupBackupsByDay(items: readonly Schema['BackupItemDTO'][]): BackupDay[] {
  const byDay = new Map<string, Schema['BackupItemDTO'][]>();
  for (const item of items) {
    const day = item.modificationDate
      ? new Date(item.modificationDate).toDateString()
      : 'Unknown date';
    const bucket = byDay.get(day);
    if (bucket) bucket.push(item);
    else byDay.set(day, [item]);
  }
  return [...byDay.entries()]
    .sort((a, b) => {
      if (a[0] === 'Unknown date') return 1;
      if (b[0] === 'Unknown date') return -1;
      return new Date(b[0]).getTime() - new Date(a[0]).getTime();
    })
    .map(([day, dayItems]) => ({ day, items: dayItems }));
}

export function formatBackupDay(day: string): string {
  if (day === 'Unknown date') return day;
  return new Date(day).toLocaleDateString(undefined, {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
  });
}

export function legacyOrUnmatchedBackups(
  backups: readonly Schema['BackupItemDTO'][],
  slots: readonly Schema['WorldSlotDTO'][],
): Schema['BackupItemDTO'][] {
  const known = new Set(slots.map((slot) => slot.id));
  return backups.filter((backup) => !backup.slotId || !known.has(backup.slotId));
}

export function legacyBackupReason(item: Schema['BackupItemDTO']): string {
  if (item.slotId) {
    const name = item.slotName?.trim();
    return name ? `Missing slot: ${name}` : `Missing slot ID: ${item.slotId}`;
  }
  return 'Legacy backup (no slot metadata)';
}

/** MSC 1's WorldConversionWizardView.compatibleTargetServers: the opposite
 *  edition, never the source server itself. */
export function compatibleTargetServers(
  servers: readonly Schema['ServerDTO'][],
  sourceServer: Schema['ServerDTO'] | undefined,
): Schema['ServerDTO'][] {
  if (!sourceServer) return [];
  const sourceIsBedrock = sourceServer.serverType === 'bedrock';
  return servers.filter(
    (server) =>
      server.id !== sourceServer.id && (server.serverType === 'bedrock') !== sourceIsBedrock,
  );
}

const OPERATION_POLL_MS = 900;

/** Every world/backup mutation that touches real files (activate, convert,
 *  back-up-now, restore) is operation-backed (docs/msc2/worlds/phase6-api.md)
 *  -- the route returns immediately with an operationId, and the real
 *  outcome lands on GET /v1/operations/{id}. Polls until a terminal state. */
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

/** Ports AppViewModel+WorldSlots.swift's importLegacyBackupAsNewSlot naming:
 *  the backup's own recorded slot name first, else "Imported {displayName}",
 *  else a flat fallback. */
export function legacyImportName(backup: Schema['BackupItemDTO']): string {
  const slotName = backup.slotName?.trim();
  if (slotName) return slotName;
  const displayName = backup.displayName.trim();
  return displayName ? `Imported ${displayName}` : 'Imported Backup';
}

/** AppViewModel+WorldManagement.swift's replaceWorld reads the server's
 *  current server.properties `level-name` and passes it straight back --
 *  Replace World never renames the live world, it only swaps its content.
 *  GET /v1/settings only surfaces `level-name` as an editable field for
 *  Bedrock (routes/settings.rs::bedrock_sections); Java's settings response
 *  never exposes it (it isn't a client-editable field there), so this falls
 *  back to Minecraft's own default world-folder name for Java. */
export function currentLevelName(settings: Schema['SettingsResponseDTO'] | undefined): string {
  const field = settings?.sections
    .flatMap((section) => section.fields)
    .find((candidate) => candidate.key === 'level-name');
  return field?.value.trim() || 'world';
}

/** ChunkerManager.displayName(forFormat:) -- "JAVA_1_21_0" -> "Java 1.21",
 *  "BEDROCK_R21_80" -> "Bedrock 1.21.80". Falls back to the raw string for
 *  anything Chunker reports that doesn't match either shape. */
export function formatDisplayName(format: string): string {
  if (format.startsWith('JAVA_')) {
    return `Java ${format.slice(5).replace(/_/g, '.')}`;
  }
  if (format.startsWith('BEDROCK_')) {
    const raw = format.slice(8);
    if (raw.startsWith('R')) {
      const [minor, patch] = raw.slice(1).split('_');
      if (minor !== undefined && /^\d+$/.test(minor)) {
        return patch !== undefined && /^\d+$/.test(patch)
          ? `Bedrock 1.${minor}.${patch}`
          : `Bedrock 1.${minor}`;
      }
    }
    return `Bedrock ${raw.replace(/_/g, '.')}`;
  }
  return format;
}

/** WorldConversionWizardView.targetFormats: only the opposite edition's
 *  formats, oldest-to-newest as Chunker itself reports them (the same order
 *  MSC 1 trusts when it defaults its picker to the last/newest entry). */
export function targetFormats(
  formats: readonly string[],
  sourceServer: Schema['ServerDTO'] | undefined,
): string[] {
  const prefix = sourceServer?.serverType === 'bedrock' ? 'JAVA_' : 'BEDROCK_';
  return formats.filter((format) => format.startsWith(prefix));
}
