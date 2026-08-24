import { describe, expect, it } from 'vitest';
import {
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
  it('uses the shared console, command, performance, and operation contract paths', () => {
    expect(livePaths).toMatchObject({
      command: '/v1/command',
      tail: '/v1/console/tail?n=200',
      performance: '/v1/performance',
    });
  });
});
