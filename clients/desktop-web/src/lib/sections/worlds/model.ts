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
