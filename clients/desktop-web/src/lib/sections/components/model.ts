import type { Schema, ScreenApi } from '../shared/types';

export { addonPaths, addonStatusLabel, demoAddons } from '../addons/model';

// Real, frozen routes this tab calls (docs/msc2/api-contract/openapi.json).
// /v1/components/version and /v1/versions are shared by both the Server
// JAR/Loader row and the Bedrock Runtime card -- VersionsResponseDTO already
// carries isBedrock/runtime, so one version-management flow covers both
// editions rather than needing a separate Bedrock-only route.
export const componentPaths = {
  status: '/v1/components',
  version: '/v1/components/version',
  versions: '/v1/versions',
} as const;

export const broadcastPaths = {
  status: '/v1/broadcast/status',
  autostart: '/v1/broadcast/autostart',
  jarStatus: '/v1/broadcast/jar-status',
  downloadJar: '/v1/broadcast/download-jar',
} as const;

export const serversPath = '/v1/servers';
export const healthPath = '/v1/health';
export const operationPath = (id: string): string => `/v1/operations/${id}`;

// JavaServerFlavor.swift's own classification (category/addOnKind), applied
// to ServerDTO.javaFlavor. addOnKind is undefined only for vanilla, which has
// no plugin/mod API (datapacks only) -- the add-ons browser stays hidden.
const MODDED_FLAVORS = new Set(['fabric', 'neoforge', 'forge', 'quilt']);
const FLAVOR_DISPLAY_NAMES: Record<string, string> = {
  paper: 'Paper',
  purpur: 'Purpur',
  pufferfish: 'Pufferfish',
  vanilla: 'Vanilla',
  fabric: 'Fabric',
  neoforge: 'NeoForge',
  spigot: 'Spigot',
  forge: 'Forge',
  quilt: 'Quilt',
};

export function isModdedFlavor(javaFlavor: string | undefined): boolean {
  return !!javaFlavor && MODDED_FLAVORS.has(javaFlavor);
}

export function addOnKind(javaFlavor: string | undefined): 'mod' | 'plugin' | undefined {
  if (!javaFlavor || javaFlavor === 'vanilla') return undefined;
  return isModdedFlavor(javaFlavor) ? 'mod' : 'plugin';
}

export function flavorDisplayName(javaFlavor: string | undefined): string {
  if (!javaFlavor) return 'Server JAR';
  return FLAVOR_DISPLAY_NAMES[javaFlavor] ?? javaFlavor;
}

/** DetailsComponentsTabView.swift's showCrossplay: Java, not modded, not vanilla. */
export function supportsCrossplay(server: Schema['ServerDTO'] | undefined): boolean {
  if (!server || server.serverType === 'bedrock') return false;
  return addOnKind(server.javaFlavor) === 'plugin';
}

export type Tone = 'ok' | 'warn' | 'error';

/** ComponentStatusDTO already carries the agent's own resolved verdict
 *  (isUpToDate/updatable/note) -- unlike MSC 1's client-side ComponentStatus
 *  .derive, there's no version-string comparison to reproduce here. */
export function componentTone(component: Schema['ComponentStatusDTO']): Tone {
  if (component.installedVersion === undefined && component.installedLabel === undefined) {
    return 'error';
  }
  if (component.note) return 'warn';
  return component.isUpToDate ? 'ok' : 'warn';
}

export function componentStatusLabel(component: Schema['ComponentStatusDTO']): string {
  if (component.installedVersion === undefined && component.installedLabel === undefined) {
    return 'Missing';
  }
  if (component.note) return component.note;
  return component.isUpToDate ? 'Up to date' : 'Update available';
}

export const demoComponentsStatus: Schema['ComponentsStatusDTO'] = {
  components: [
    {
      name: 'Paper',
      installedLabel: '26.2 (build 117)',
      installedVersion: '26.2',
      installedBuild: 117,
      isUpToDate: true,
      updatable: true,
    },
  ],
  restartRequiredToApply: false,
};

export const demoVersions: Schema['VersionsResponseDTO'] = {
  supportsVersions: true,
  flavorName: 'Paper',
  isBedrock: false,
  currentVersion: '26.2 (build 117)',
  versions: [],
};

export const demoBroadcastStatus: Schema['BroadcastStatusDTO'] = {
  xboxBroadcastRunning: false,
  bedrockBroadcastRunning: false,
};

export const demoBroadcastAutostart: Schema['BroadcastAutoStartDTO'] = { enabled: false };

export const demoJarStatus: Schema['BroadcastJarStatusDTO'] = {
  installed: false,
  downloading: false,
};

const OPERATION_POLL_MS = 900;

/** Same shape as worlds/model.ts's pollOperation -- every version-change,
 *  add-on install/update, and broadcast-JAR download here is operation-backed. */
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
