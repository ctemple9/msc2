import type { Schema } from '../shared/types';

export type ConsoleLine = Schema['ConsoleLineDTO'];

/**
 * Stable identity for a console line while the agent's tail has no numeric id.
 * The dock uses this to keep a locally-cleared line hidden when the next poll
 * returns the agent's still-retained history.
 */
export function consoleLineKey(line: ConsoleLine): string {
  return [line.ts, line.source, line.level ?? '', line.text].join('\u0000');
}

function consoleLineTimestamp(ts: string): number | undefined {
  const numeric = Number(ts);
  if (Number.isFinite(numeric)) return numeric;
  const parsed = Date.parse(ts);
  return Number.isNaN(parsed) ? undefined : parsed;
}

/**
 * Applies the dock's local clear boundary to a freshly fetched tail. The
 * timestamp handles lines that arrived between the last poll and Clear; the
 * key set handles lines already rendered, including local command echoes.
 */
export function consoleLinesAfterClear(
  lines: readonly ConsoleLine[],
  clearedAt: number | undefined,
  clearedLineKeys: ReadonlySet<string>,
): ConsoleLine[] {
  if (clearedAt === undefined && clearedLineKeys.size === 0) return [...lines];
  return lines.filter((line) => {
    if (clearedLineKeys.has(consoleLineKey(line))) return false;
    const timestamp = consoleLineTimestamp(line.ts);
    return timestamp === undefined || clearedAt === undefined || timestamp > clearedAt;
  });
}

export const demoConsole: ConsoleLine[] = [
  {
    ts: '2026-08-24T12:00:00Z',
    source: 'server',
    text: '[Server thread/INFO]: Server ready',
    level: 'info',
  },
  { ts: '2026-08-24T12:00:04Z', source: 'server', text: 'No players online', level: 'info' },
];

export function filterLines(
  lines: readonly ConsoleLine[],
  search: string,
  level: string,
): ConsoleLine[] {
  const needle = search.trim().toLowerCase();
  return lines.filter(
    (line) =>
      (!needle || line.text.toLowerCase().includes(needle)) && (!level || line.level === level),
  );
}

export function rememberCommand(history: readonly string[], command: string, max = 30): string[] {
  const value = command.trim();
  if (!value) return [...history];
  return [value, ...history.filter((item) => item !== value)].slice(0, max);
}

export const livePaths = {
  command: '/v1/command',
  tail: '/v1/console/tail?n=200',
  performance: '/v1/performance',
  operations: '/v1/operations',
  // Same route players-online/model.ts's playerPaths.players calls (P12.10b:
  // feeds the command palette's player chips and autocomplete). Real for
  // Bedrock; a Java active server gets back an empty roster with
  // note:"not_bedrock" (crates/msc-agent/src/routes/players.rs) -- no
  // equivalent live-online-roster route exists for Java yet, so a Java
  // palette/autocomplete honestly falls back to plain text entry for
  // player arguments, same as MSC 1 does with zero online players.
  players: '/v1/players',
} as const;

// --- Docked console (P12.10): MSC 1 ConsoleManager/ConsoleLineParser ported to the
// agent's actual ConsoleLineDTO {ts, source, level?, text}. The agent only ever sends
// source in {"stdout","stderr","bedrock","system"} and never sets `level` (see
// crates/msc-agent/src/routes/lifecycle.rs) -- MSC 1's richer per-line `tag` has no
// contract equivalent, so Server/Plugins is inferred the same way MSC 1 does: from a
// bracketed token in the raw text, not from a field the agent doesn't send.

export type ConsoleChipId =
  'all' | 'server' | 'plugins' | 'warnings' | 'controller' | 'commands' | 'custom';

export const CONSOLE_CHIPS: readonly { id: ConsoleChipId; label: string }[] = [
  { id: 'all', label: 'All' },
  { id: 'server', label: 'Server' },
  { id: 'plugins', label: 'Plugins' },
  { id: 'warnings', label: 'Warnings' },
  { id: 'controller', label: 'Controller' },
  { id: 'commands', label: 'Commands' },
  { id: 'custom', label: 'Custom' },
];

export type ConsoleOrigin = 'server' | 'controller';
export type ConsoleLevel = 'info' | 'warn' | 'error';
export type ConsoleCategory = 'server' | 'plugins' | 'controller' | 'commands';

const CORE_SERVER_TAGS = new Set(['bootstrap', 'minecraftserver', 'paper', 'server', 'main']);

/** MSC 1 ConsoleLineParser.inferLevel -- the agent never sets `level`, so this reads it
 *  from the declared field when present and otherwise infers it from the raw text. */
export function inferLevel(line: ConsoleLine): ConsoleLevel {
  const declared = line.level?.trim().toLowerCase();
  if (declared === 'error' || declared === 'warn' || declared === 'info') return declared;
  const upper = line.text.toUpperCase();
  if (upper.includes(' ERROR') || upper.includes(' SEVERE')) return 'error';
  if (upper.includes(' WARN')) return 'warn';
  if (upper.includes('EXCEPTION') || upper.includes('JAVA.LANG.')) return 'error';
  return 'info';
}

/** MSC 1's Warnings tab also sweeps in bare stack-trace continuation lines that have
 *  no level token of their own. */
export function isStackTraceLine(text: string): boolean {
  const lower = text.toLowerCase();
  return (
    lower.includes(' exception') ||
    lower.includes('java.lang.') ||
    lower.trimStart().startsWith('at ')
  );
}

/** The origin the agent actually reports (game process vs. agent/app) -- MSC 1's
 *  ConsoleSource. Sent commands are echoed locally with source "command" and count
 *  as controller-origin, same as MSC 1 tagging "You → cmd" lines source=.controller. */
export function originOf(line: ConsoleLine): ConsoleOrigin {
  return line.source === 'system' || line.source === 'command' ? 'controller' : 'server';
}

/** First bracketed token that isn't a timestamp or a bare level word -- MSC 1's
 *  plugin/core tag heuristic (extractBracketTokens + isCoreServerTag), reduced to what
 *  the raw text alone can tell us. */
function bracketTag(text: string): string | undefined {
  for (const match of text.matchAll(/\[([^[\]]{1,32})\]/g)) {
    const token = match[1].trim();
    if (!token) continue;
    if (/^\d{1,2}:\d{2}(:\d{2})?$/.test(token)) continue;
    const lower = token.toLowerCase();
    if (
      lower === 'info' ||
      lower === 'warn' ||
      lower === 'error' ||
      lower === 'severe' ||
      lower === 'debug'
    ) {
      continue;
    }
    return token;
  }
  return undefined;
}

function isCoreServerTag(tag: string): boolean {
  const lower = tag.toLowerCase();
  if (CORE_SERVER_TAGS.has(lower)) return true;
  return (
    lower.includes('server thread') ||
    lower.endsWith('/info') ||
    lower.endsWith('/warn') ||
    lower.endsWith('/error')
  );
}

export function categoryOf(line: ConsoleLine): ConsoleCategory {
  if (line.source === 'command') return 'commands';
  if (line.source === 'system') return 'controller';
  const tag = bracketTag(line.text);
  if (!tag || isCoreServerTag(tag)) return 'server';
  return 'plugins';
}

export function matchesChip(line: ConsoleLine, chip: ConsoleChipId): boolean {
  switch (chip) {
    case 'all':
    case 'custom':
      return true;
    case 'server':
      return categoryOf(line) === 'server';
    case 'plugins':
      return categoryOf(line) === 'plugins';
    case 'controller': {
      const category = categoryOf(line);
      return category === 'controller' || category === 'commands';
    }
    case 'commands':
      return categoryOf(line) === 'commands';
    case 'warnings':
      return inferLevel(line) !== 'info' || isStackTraceLine(line.text);
    default:
      return true;
  }
}

/** The reduced form of MSC 1's Custom filters popover (Sources + Levels groups) --
 *  Tags and Hide Auto have no equivalent since the agent sends neither a tag nor an
 *  auto-attribution flag. An empty set means "no restriction," matching MSC 1. */
export interface CustomFilter {
  origins: ReadonlySet<ConsoleOrigin>;
  levels: ReadonlySet<ConsoleLevel>;
}

export const EMPTY_CUSTOM_FILTER: CustomFilter = { origins: new Set(), levels: new Set() };

export function customFilterActive(filter: CustomFilter): boolean {
  return filter.origins.size > 0 || filter.levels.size > 0;
}

export function matchesCustomFilter(line: ConsoleLine, filter: CustomFilter): boolean {
  if (filter.origins.size > 0 && !filter.origins.has(originOf(line))) return false;
  if (filter.levels.size > 0 && !filter.levels.has(inferLevel(line))) return false;
  return true;
}

export function matchesSearch(line: ConsoleLine, search: string): boolean {
  const needle = search.trim().toLowerCase();
  return !needle || line.text.toLowerCase().includes(needle);
}

export function visibleConsoleLines(
  lines: readonly ConsoleLine[],
  chip: ConsoleChipId,
  custom: CustomFilter,
  search: string,
): ConsoleLine[] {
  return lines.filter(
    (line) =>
      matchesChip(line, chip) &&
      (chip !== 'custom' || matchesCustomFilter(line, custom)) &&
      matchesSearch(line, search),
  );
}

/** MSC 1 ConsoleView.logColor: controller-origin lines (incl. echoed commands) are
 *  always muted regardless of level; otherwise color follows level. */
export function consoleLineTone(line: ConsoleLine): 'error' | 'warn' | 'muted' | 'default' {
  if (originOf(line) === 'controller') return 'muted';
  const level = inferLevel(line);
  return level === 'info' ? 'default' : level;
}

/** A locally-echoed sent command -- the agent has no server-side record of commands
 *  issued through `/v1/command` (it only forwards them to the process's stdin), so the
 *  dock appends this itself, the same way MSC 1's ConsoleManager appends a "You → cmd"
 *  entry client-side. */
export function commandEchoLine(command: string): ConsoleLine {
  return { ts: Date.now().toString(), source: 'command', text: `› ${command}` };
}

export function formatConsoleTimestamp(ts: string): string {
  const millis = Number(ts);
  const date = Number.isFinite(millis) ? new Date(millis) : new Date(ts);
  return Number.isNaN(date.getTime()) ? ts : date.toLocaleTimeString();
}

// --- Command Palette (P12.10a): MinecraftCommandRegistry.swift ported
// verbatim -- a static list, no backend involved (that file's own header
// comment). Categories keep MSC 1's five groupings for sectioning the
// command list, but drop its per-category icon+color (antiAIslop.md #11 --
// color is a shared, scarce resource, not a per-row rail); grouping uses a
// plain overline header instead, the same vocabulary every other rebuilt
// screen already uses. MSC 1's `@AppStorage` favorites/star system is a
// separate, smaller affordance this step's own plan text never named --
// left deferred, not fabricated.

export type CommandArgKind = 'player' | 'keyword' | 'coordinates' | 'integer' | 'freeText';

export interface CommandArgSlot {
  readonly kind: CommandArgKind;
  readonly label: string;
  readonly options?: readonly string[];
}

function player(label = 'player'): CommandArgSlot {
  return { kind: 'player', label };
}
function keyword(options: readonly string[], label: string): CommandArgSlot {
  return { kind: 'keyword', label, options };
}
function coordinates(label = 'x y z'): CommandArgSlot {
  return { kind: 'coordinates', label };
}
function integer(label: string): CommandArgSlot {
  return { kind: 'integer', label };
}
function freeText(label: string): CommandArgSlot {
  return { kind: 'freeText', label };
}

export type CommandCategory = 'Players' | 'World' | 'Server Admin' | 'Game Rules' | 'Creative';

export const COMMAND_CATEGORIES: readonly CommandCategory[] = [
  'Players',
  'World',
  'Server Admin',
  'Game Rules',
  'Creative',
];

export interface MinecraftCommandDef {
  readonly name: string;
  readonly description: string;
  readonly category: CommandCategory;
  readonly argumentSlots: readonly CommandArgSlot[];
  readonly supportsJava: boolean;
  readonly supportsBedrock: boolean;
}

export const MINECRAFT_COMMANDS: readonly MinecraftCommandDef[] = [
  // Players
  {
    name: 'tp',
    description: 'Teleport a player to another player or to coordinates',
    category: 'Players',
    argumentSlots: [player('target player'), player('destination player')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'teleport',
    description: 'Alias for tp — teleport to player or coordinates',
    category: 'Players',
    argumentSlots: [player('target player'), coordinates('destination x y z')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'give',
    description: 'Give a player one or more items',
    category: 'Players',
    argumentSlots: [player(), freeText('item id (e.g. diamond_sword)'), integer('count')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'kick',
    description: 'Remove a player from the server',
    category: 'Players',
    argumentSlots: [player(), freeText('reason (optional)')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'ban',
    description: 'Permanently ban a player by name',
    category: 'Players',
    argumentSlots: [player(), freeText('reason (optional)')],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'ban-ip',
    description: 'Ban a player by IP address',
    category: 'Players',
    argumentSlots: [freeText('ip address or player name')],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'pardon',
    description: 'Unban a previously banned player',
    category: 'Players',
    argumentSlots: [player()],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'op',
    description: 'Grant operator (admin) status to a player',
    category: 'Players',
    argumentSlots: [player()],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'deop',
    description: 'Revoke operator status from a player',
    category: 'Players',
    argumentSlots: [player()],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'msg',
    description: 'Send a private message to a player',
    category: 'Players',
    argumentSlots: [player(), freeText('message')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'tell',
    description: 'Alias for msg — send a private message',
    category: 'Players',
    argumentSlots: [player(), freeText('message')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'kill',
    description: 'Kill a player or entity',
    category: 'Players',
    argumentSlots: [player()],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'gamemode',
    description: "Change a player's game mode",
    category: 'Players',
    argumentSlots: [
      keyword(['survival', 'creative', 'adventure', 'spectator'], 'mode'),
      player('player (optional)'),
    ],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'effect',
    description: 'Apply a status effect to a player',
    category: 'Players',
    argumentSlots: [
      player(),
      freeText('effect id (e.g. speed, jump_boost)'),
      integer('duration (seconds)'),
      integer('amplifier (0 = level I)'),
    ],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'xp',
    description: 'Give experience points or levels to a player',
    category: 'Players',
    argumentSlots: [freeText('amount (use L suffix for levels, e.g. 5L)'), player()],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'experience',
    description: 'Add or set experience points or levels',
    category: 'Players',
    argumentSlots: [
      keyword(['add', 'set', 'query'], 'action'),
      player(),
      integer('amount'),
      keyword(['points', 'levels'], 'type'),
    ],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'clear',
    description: "Clear a player's inventory or a specific item",
    category: 'Players',
    argumentSlots: [player(), freeText('item (optional)')],
    supportsJava: true,
    supportsBedrock: true,
  },

  // World
  {
    name: 'time',
    description: 'Set, add, or query the world time',
    category: 'World',
    argumentSlots: [
      keyword(['set', 'add', 'query'], 'action'),
      freeText('value or day/night/noon/midnight'),
    ],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'weather',
    description: 'Change the current weather',
    category: 'World',
    argumentSlots: [keyword(['clear', 'rain', 'thunder'], 'type')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'difficulty',
    description: 'Set the game difficulty',
    category: 'World',
    argumentSlots: [keyword(['peaceful', 'easy', 'normal', 'hard'], 'level')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'gamerule',
    description: 'Set or query a game rule',
    category: 'World',
    argumentSlots: [freeText('rule name'), freeText('value (omit to query)')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'setworldspawn',
    description: 'Set the world spawn point',
    category: 'World',
    argumentSlots: [coordinates()],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'spawnpoint',
    description: "Set a player's personal spawn point",
    category: 'World',
    argumentSlots: [player(), coordinates()],
    supportsJava: true,
    supportsBedrock: true,
  },

  // Server Admin
  {
    name: 'list',
    description: 'List all currently online players',
    category: 'Server Admin',
    argumentSlots: [],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'seed',
    description: 'Display the current world seed',
    category: 'Server Admin',
    argumentSlots: [],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'say',
    description: 'Broadcast a message to all players',
    category: 'Server Admin',
    argumentSlots: [freeText('message')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'title',
    description: 'Display a title on screen for a player',
    category: 'Server Admin',
    argumentSlots: [
      player(),
      keyword(['title', 'subtitle', 'actionbar', 'clear', 'reset'], 'type'),
      freeText('text (omit for clear/reset)'),
    ],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'save-all',
    description: 'Force save all loaded chunks to disk',
    category: 'Server Admin',
    argumentSlots: [],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'save-off',
    description: 'Disable automatic chunk saving',
    category: 'Server Admin',
    argumentSlots: [],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'save-on',
    description: 'Re-enable automatic chunk saving',
    category: 'Server Admin',
    argumentSlots: [],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'reload',
    description: 'Reload server configuration and plugins',
    category: 'Server Admin',
    argumentSlots: [],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'stop',
    description: 'Gracefully stop the server',
    category: 'Server Admin',
    argumentSlots: [],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'whitelist',
    description: 'Manage the server whitelist',
    category: 'Server Admin',
    argumentSlots: [
      keyword(['on', 'off', 'add', 'remove', 'list', 'reload'], 'action'),
      player('player (for add/remove)'),
    ],
    supportsJava: true,
    supportsBedrock: false,
  },
  {
    name: 'allowlist',
    description: 'Manage the BDS allowlist (Bedrock equivalent of whitelist)',
    category: 'Server Admin',
    argumentSlots: [
      keyword(['on', 'off', 'add', 'remove', 'list', 'reload'], 'action'),
      player('player (for add/remove)'),
    ],
    supportsJava: false,
    supportsBedrock: true,
  },
  {
    name: 'banlist',
    description: 'Display the current ban list',
    category: 'Server Admin',
    argumentSlots: [keyword(['players', 'ips'], 'type (optional)')],
    supportsJava: true,
    supportsBedrock: false,
  },

  // Game Rules
  {
    name: 'enchant',
    description: "Enchant the item in a player's hand",
    category: 'Game Rules',
    argumentSlots: [player(), freeText('enchantment id'), integer('level')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'attribute',
    description: 'Query or modify an entity attribute',
    category: 'Game Rules',
    argumentSlots: [player(), freeText('attribute name')],
    supportsJava: true,
    supportsBedrock: false,
  },

  // Creative / Building
  {
    name: 'setblock',
    description: 'Place a specific block at given coordinates',
    category: 'Creative',
    argumentSlots: [coordinates(), freeText('block id')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'fill',
    description: 'Fill a region with a block type',
    category: 'Creative',
    argumentSlots: [coordinates('from x y z'), coordinates('to x y z'), freeText('block id')],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'clone',
    description: 'Copy a region of blocks to another location',
    category: 'Creative',
    argumentSlots: [
      coordinates('from x y z'),
      coordinates('to x y z'),
      coordinates('destination x y z'),
    ],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'summon',
    description: 'Spawn an entity at given coordinates',
    category: 'Creative',
    argumentSlots: [freeText('entity id'), coordinates()],
    supportsJava: true,
    supportsBedrock: true,
  },
  {
    name: 'particle',
    description: 'Create a particle effect at coordinates',
    category: 'Creative',
    argumentSlots: [freeText('particle name'), coordinates()],
    supportsJava: true,
    supportsBedrock: false,
  },
];

export function hasRequiredArgs(def: MinecraftCommandDef): boolean {
  return def.argumentSlots.length > 0;
}

export function commandSyntaxHint(def: MinecraftCommandDef): string {
  const args = def.argumentSlots.map((slot) => `<${slot.label}>`).join(' ');
  return args ? `/${def.name} ${args}` : `/${def.name}`;
}

export function commandsFor(serverType: string | undefined): readonly MinecraftCommandDef[] {
  return MINECRAFT_COMMANDS.filter((def) =>
    serverType === 'bedrock' ? def.supportsBedrock : def.supportsJava,
  );
}

export function buildCommand(def: MinecraftCommandDef, values: readonly string[]): string {
  const filled = values.map((value) => value.trim()).filter((value) => value.length > 0);
  return filled.length ? `/${def.name} ${filled.join(' ')}` : `/${def.name}`;
}

export interface CommandPlayerName {
  readonly name: string;
}

/** MinecraftCommandRegistry.suggestions: token 0 completes a command name;
 *  token 1+ completes that command's current argument slot -- players for a
 *  player slot, the fixed option list for a keyword slot, nothing for the
 *  rest. Up to 6 results, same as the oracle.
 *
 *  One deliberate fix, not a straight port: the oracle tokenizes with
 *  `split(omittingEmptySubsequences: false)`, which keeps the empty string
 *  a trailing space produces as if it were a real argument token. That
 *  makes the *first* space after a command name (or after finishing an
 *  argument) count as one already-filled slot too many -- e.g. "/tp "
 *  points suggestions at the second argument ("destination player"),
 *  doubled-spaced, instead of the first. This tokenizes on non-empty
 *  tokens instead, so a trailing space always means "start the next slot"
 *  at the correct index -- the behavior the feature clearly intends,
 *  without the split artifact. */
export function commandSuggestions(
  input: string,
  serverType: string | undefined,
  onlinePlayers: readonly CommandPlayerName[],
): string[] {
  if (!input) return [];

  const endsWithSpace = input.endsWith(' ');
  const rawTokens = input.split(' ').filter((token) => token.length > 0);
  if (rawTokens.length === 0) return [];
  const available = commandsFor(serverType);

  if (rawTokens.length === 1 && !endsWithSpace) {
    const raw = rawTokens[0];
    const prefix = (raw.startsWith('/') ? raw.slice(1) : raw).toLowerCase();
    if (!prefix) return [];
    return available
      .filter((def) => def.name.startsWith(prefix))
      .slice(0, 6)
      .map((def) => `/${def.name}`);
  }

  const rawCommand = rawTokens[0];
  const commandName = rawCommand.startsWith('/') ? rawCommand.slice(1) : rawCommand;
  const def = available.find((item) => item.name === commandName);
  if (!def || def.argumentSlots.length === 0) return [];

  const argTokens = rawTokens.slice(1);
  const slotIndex = endsWithSpace ? argTokens.length : Math.max(0, argTokens.length - 1);
  if (slotIndex >= def.argumentSlots.length) return [];
  const slot = def.argumentSlots[slotIndex];

  const partial = (endsWithSpace ? '' : (argTokens[argTokens.length - 1] ?? '')).toLowerCase();
  const baseTokens = endsWithSpace ? rawTokens : rawTokens.slice(0, -1);
  const base = baseTokens.join(' ');

  if (slot.kind === 'player') {
    return onlinePlayers
      .filter((entry) => !partial || entry.name.toLowerCase().startsWith(partial))
      .slice(0, 6)
      .map((entry) => `${base} ${entry.name}`);
  }
  if (slot.kind === 'keyword' && slot.options) {
    return slot.options
      .filter((option) => !partial || option.toLowerCase().startsWith(partial))
      .slice(0, 6)
      .map((option) => `${base} ${option}`);
  }
  return [];
}
