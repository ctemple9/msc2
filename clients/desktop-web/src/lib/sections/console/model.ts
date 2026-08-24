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
