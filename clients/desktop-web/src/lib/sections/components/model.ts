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

/** ProjectDetailSheet's minimum input -- satisfied directly by a
 *  CatalogItemDTO search hit (every optional field present), or built from
 *  an already-installed AddonItemDTO (which only ever has projectId/
 *  displayName/iconURL -- the rest loads from the P12.7c project-detail
 *  fetch itself, same as the sheet's own loading-state fallbacks). */
export interface ProjectDetailItem {
  projectId: string;
  title: string;
  slug?: string;
  author?: string;
  downloads?: number;
  description?: string;
  iconURL?: string;
  projectType?: string;
}

// ModrinthProjectDetailView's two extra fetches (ModrinthBrowserView.swift:
// 748-753) -- both new P12.7c routes, keyed by Modrinth project id/slug.
export const catalogDetailPaths = {
  project: (projectId: string): string => `/v1/catalog/projects/${encodeURIComponent(projectId)}`,
  versions: (projectId: string): string =>
    `/v1/catalog/projects/${encodeURIComponent(projectId)}/versions`,
} as const;

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

// JavaServerFlavor.modrinth_loader_facets (identity.rs:216-227) -- the same
// facets the backend already applies to GET /v1/catalog/search, needed again
// client-side here because the project detail page fetches every version
// unfiltered (ModrinthProjectDetailView.swift:753) and filters locally.
const LOADER_FACETS: Record<string, string[]> = {
  paper: ['paper', 'spigot', 'bukkit'],
  purpur: ['paper', 'spigot', 'bukkit'],
  pufferfish: ['paper', 'spigot', 'bukkit'],
  spigot: ['paper', 'spigot', 'bukkit'],
  fabric: ['fabric'],
  quilt: ['quilt', 'fabric'],
  neoforge: ['neoforge'],
  forge: ['forge'],
  vanilla: [],
};

export function modrinthLoaderFacets(javaFlavor: string | undefined): string[] {
  if (!javaFlavor) return [];
  return LOADER_FACETS[javaFlavor] ?? [];
}

/** ModrinthProjectDetailView.expandedLoaders (line 359-364): NeoForge servers
 *  also run Forge mods, Quilt servers also run Fabric mods, for the purpose of
 *  deciding which versions to show -- not what the search facets ask for. */
export function expandedLoaders(javaFlavor: string | undefined, loaders: string[]): Set<string> {
  const set = new Set(loaders);
  if (javaFlavor === 'neoforge') set.add('forge');
  if (javaFlavor === 'quilt') set.add('fabric');
  return set;
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

export function formatCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

export function isStableVersion(version: Schema['CatalogVersionDTO']): boolean {
  return version.versionType === 'release';
}

/** versionRow's conflictCount (ModrinthBrowserView.swift:527) -- a version
 *  can declare another mod/plugin incompatible with it. */
export function conflictCount(version: Schema['CatalogVersionDTO']): number {
  return version.dependencies.filter((dependency) => dependency.dependencyType === 'incompatible')
    .length;
}

/** isCompatible (line 732-735): does this version's declared game-versions
 *  list include the active server's Minecraft version. */
export function isVersionCompatible(
  version: Schema['CatalogVersionDTO'],
  serverMinecraftVersion: string | undefined,
): boolean {
  return !!serverMinecraftVersion && version.gameVersions.includes(serverMinecraftVersion);
}

/** collapsedVersions (ModrinthProjectDetailView.swift:339-355): cross-platform
 *  projects like Geyser publish one Modrinth version per loader sharing the
 *  same build number -- keep one entry per version number, preferring
 *  whichever loader variant matches this server. */
export function collapseVersions(
  versions: Schema['CatalogVersionDTO'][],
  serverLoaders: Set<string>,
): Schema['CatalogVersionDTO'][] {
  const best = new Map<string, Schema['CatalogVersionDTO']>();
  const order: string[] = [];
  for (const version of versions) {
    const key = version.versionNumber;
    const matches = serverLoaders.size > 0 && version.loaders.some((l) => serverLoaders.has(l));
    const existing = best.get(key);
    if (existing) {
      const existingMatches =
        serverLoaders.size > 0 && existing.loaders.some((l) => serverLoaders.has(l));
      if (matches && !existingMatches) best.set(key, version);
    } else {
      best.set(key, version);
      order.push(key);
    }
  }
  return order.map((key) => best.get(key) as Schema['CatalogVersionDTO']);
}

/** visibleVersions (line 366-381): stable-only filter, then a loader filter
 *  that never hides every option -- falls back to the unfiltered stable list
 *  rather than showing nothing when the loader filter would remove everything. */
export function filterVisibleVersions(
  versions: Schema['CatalogVersionDTO'][],
  options: { stableOnly: boolean; loaders: Set<string> },
): Schema['CatalogVersionDTO'][] {
  const stable = options.stableOnly ? versions.filter(isStableVersion) : versions;
  if (options.loaders.size > 0) {
    const filtered = stable.filter(
      (version) =>
        version.loaders.length === 0 || version.loaders.some((l) => options.loaders.has(l)),
    );
    if (filtered.length > 0) return filtered;
  }
  return stable;
}

/** sanitizedBodyMarkdown (ModrinthBrowserView.swift:681-730): Modrinth project
 *  bodies are GitHub-Flavored Markdown with raw HTML mixed in. Reduces that to
 *  a small safe subset -- plain text, **bold**, and [text](url) links -- that
 *  parseInlineMarkdown then turns into real DOM nodes. Untrusted third-party
 *  text never gets passed to `{@html}` at any point in this pipeline. */
export function sanitizeModrinthBody(raw: string): string {
  let s = raw;
  for (const tag of ['iframe', 'script', 'style', 'video', 'table']) {
    s = s.replace(new RegExp(`<${tag}[\\s\\S]*?</${tag}>`, 'gi'), '');
  }
  s = s.replace(/<br\s*\/?>/gi, '\n');
  s = s.replace(/<img[^>]*>/gi, '');
  s = s.replace(/!\[[^\]]*]\([^)]*\)/g, '');
  s = s.replace(/<a[^>]*href=["']([^"']+)["'][^>]*>([\s\S]*?)<\/a>/gi, '[$2]($1)');
  s = s.replace(/\[\s*]\([^)]*\)/g, '');
  s = s.replace(/<[^>]+>/g, '');
  const entities: Record<string, string> = {
    '&amp;': '&',
    '&lt;': '<',
    '&gt;': '>',
    '&quot;': '"',
    '&#39;': "'",
    '&nbsp;': ' ',
    '&mdash;': '—',
    '&ndash;': '–',
  };
  for (const [entity, char] of Object.entries(entities)) {
    s = s.split(entity).join(char);
  }
  s = s
    .split('\n')
    .map((line) => {
      const trimmed = line.trim();
      if (trimmed === '---' || trimmed === '***' || trimmed === '___') return '';
      const header = trimmed.match(/^#{1,6}\s+(.*)$/);
      if (header) return header[1].trim() ? `**${header[1].trim()}**` : '';
      return line.replace(/^\s*>\s?/, '');
    })
    .join('\n');
  s = s.replace(/\n{3,}/g, '\n\n');
  return s.trim();
}

export type InlineSegment =
  | { type: 'text'; text: string }
  | { type: 'bold'; text: string }
  | { type: 'link'; text: string; href: string };

const INLINE_TOKEN = /\*\*(.+?)\*\*|\[([^\]]+)]\((https?:\/\/[^\s)]+)\)/g;

/** Splits one line of `sanitizeModrinthBody`'s output into text/bold/link
 *  segments for template rendering -- the only markdown this subset needs. */
export function parseInlineMarkdown(line: string): InlineSegment[] {
  const segments: InlineSegment[] = [];
  let lastIndex = 0;
  for (const match of line.matchAll(INLINE_TOKEN)) {
    const index = match.index ?? 0;
    if (index > lastIndex) segments.push({ type: 'text', text: line.slice(lastIndex, index) });
    if (match[1] !== undefined) {
      segments.push({ type: 'bold', text: match[1] });
    } else {
      segments.push({ type: 'link', text: match[2], href: match[3] });
    }
    lastIndex = index + match[0].length;
  }
  if (lastIndex < line.length) segments.push({ type: 'text', text: line.slice(lastIndex) });
  return segments;
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
