import { describe, expect, it } from 'vitest';
import {
  consoleLineKey,
  consoleLinesAfterClear,
  demoConsole,
  filterLines,
  livePaths,
  rememberCommand,
} from '../../src/lib/sections/console/model';

describe('live screens', () => {
  it('filters locally without changing the bounded source history', () => {
    expect(filterLines(demoConsole, 'ready', '')).toHaveLength(1);
    expect(demoConsole).toHaveLength(2);
  });
  it('deduplicates command history and keeps the newest command first', () => {
    expect(rememberCommand(['list', 'save-all'], 'list')).toEqual(['list', 'save-all']);
  });
  it('keeps an old polled tail cleared while allowing newer output through', () => {
    const oldLine = { ts: '1000', source: 'server', text: 'old' };
    const newLine = { ts: '2000', source: 'server', text: 'new' };
    expect(
      consoleLinesAfterClear([oldLine, newLine], 1500, new Set([consoleLineKey(oldLine)])),
    ).toEqual([newLine]);
  });
  it('uses the shared console, command, performance, and operation contract paths', () => {
    expect(livePaths).toMatchObject({
      command: '/v1/command',
      tail: '/v1/console/tail?n=200',
      performance: '/v1/performance',
    });
  });
});
