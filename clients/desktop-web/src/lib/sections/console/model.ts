import type { Schema } from '../shared/types';

export type ConsoleLine = Schema['ConsoleLineDTO'];

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
