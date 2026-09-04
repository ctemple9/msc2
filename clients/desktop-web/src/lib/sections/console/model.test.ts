import { describe, expect, it } from 'vitest';
import {
  MINECRAFT_COMMANDS,
  EMPTY_CUSTOM_FILTER,
  buildCommand,
  commandSuggestions,
  commandSyntaxHint,
  commandsFor,
  hasRequiredArgs,
  visibleConsoleLines,
} from './model';

describe('command palette registry', () => {
  it('filters commands by edition using the per-command flags', () => {
    const java = commandsFor('java');
    const bedrock = commandsFor('bedrock');
    expect(java.some((def) => def.name === 'ban')).toBe(true);
    expect(bedrock.some((def) => def.name === 'ban')).toBe(false);
    expect(bedrock.some((def) => def.name === 'allowlist')).toBe(true);
    expect(java.some((def) => def.name === 'allowlist')).toBe(false);
    expect(java.some((def) => def.name === 'tp')).toBe(true);
    expect(bedrock.some((def) => def.name === 'tp')).toBe(true);
  });

  it('treats an unknown/undefined server type as Java, same as the oracle default', () => {
    expect(commandsFor(undefined).some((def) => def.name === 'ban')).toBe(true);
  });

  it('reports required args and builds the syntax hint from argument labels', () => {
    const tp = MINECRAFT_COMMANDS.find((def) => def.name === 'tp')!;
    const stop = MINECRAFT_COMMANDS.find((def) => def.name === 'stop')!;
    expect(hasRequiredArgs(tp)).toBe(true);
    expect(hasRequiredArgs(stop)).toBe(false);
    expect(commandSyntaxHint(tp)).toBe('/tp <target player> <destination player>');
    expect(commandSyntaxHint(stop)).toBe('/stop');
  });

  it('builds a command string from filled args, dropping blanks', () => {
    const tp = MINECRAFT_COMMANDS.find((def) => def.name === 'tp')!;
    expect(buildCommand(tp, ['Steve', 'Alex'])).toBe('/tp Steve Alex');
    expect(buildCommand(tp, ['Steve', ''])).toBe('/tp Steve');
    expect(buildCommand(tp, ['', ''])).toBe('/tp');
  });

  it('suggests command names by prefix while typing the first token', () => {
    expect(commandSuggestions('t', 'java', [])).toEqual([
      '/tp',
      '/teleport',
      '/tell',
      '/time',
      '/title',
    ]);
    expect(commandSuggestions('/ti', 'java', [])).toEqual(['/time', '/title']);
    expect(commandSuggestions('xyz', 'java', [])).toEqual([]);
  });

  it('suggests online players for a player-typed argument slot', () => {
    const players = [{ name: 'Steve' }, { name: 'Stella' }, { name: 'Alex' }];
    expect(commandSuggestions('/tp ', 'java', players)).toEqual([
      '/tp Steve',
      '/tp Stella',
      '/tp Alex',
    ]);
    expect(commandSuggestions('/tp st', 'java', players)).toEqual(['/tp Steve', '/tp Stella']);
  });

  it('suggests the fixed option list for a keyword-typed argument slot', () => {
    expect(commandSuggestions('/gamemode ', 'java', [])).toEqual([
      '/gamemode survival',
      '/gamemode creative',
      '/gamemode adventure',
      '/gamemode spectator',
    ]);
    expect(commandSuggestions('/gamemode c', 'java', [])).toEqual(['/gamemode creative']);
  });

  it('drops Java-only commands from suggestions on a Bedrock server', () => {
    expect(commandSuggestions('ba', 'bedrock', [])).toEqual([]);
    expect(commandSuggestions('ba', 'java', [])).toEqual(['/ban', '/ban-ip', '/banlist']);
  });

  it('suggests nothing past the last argument slot or for a free-text slot', () => {
    expect(commandSuggestions('/stop ', 'java', [])).toEqual([]);
    expect(commandSuggestions('/say hello ', 'java', [])).toEqual([]);
  });

  it('hides Paper metrics and ANSI-wrapped broadcast noise without hiding normal output', () => {
    const lines = [
      { ts: '1', source: 'stdout', text: 'A player joined the game' },
      {
        ts: '2',
        source: 'stdout',
        text: 'TPS from last 1m, 5m, 15m: 19.9, 19.9, 19.9',
      },
      {
        ts: '3',
        source: 'stdout',
        text: '\uFFFD[m\uFFFD[36;1mINFO\uFFFD[m\uFFFD[39m[Primary Session] Updated session!\uFFFD[m',
      },
      { ts: '4', source: 'stdout', text: 'There are 0 of a max of 20 players online:' },
      { ts: '5', source: 'stdout', text: 'Server stopped unexpectedly', auto: true },
    ];

    expect(
      visibleConsoleLines(lines, 'all', EMPTY_CUSTOM_FILTER, '', true).map((line) => line.text),
    ).toEqual(['A player joined the game']);
  });
});
