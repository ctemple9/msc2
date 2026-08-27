import { describe, expect, it } from 'vitest';
import { selectAvailableServerId } from '../../src/lib/navigation/serverSelection';
import appSource from '../../src/App.svelte?raw';

describe('browser host context restoration', () => {
  const servers = [{ id: 'campak' }, { id: 'creative' }];

  it('uses the active server when it exists', () => {
    expect(selectAvailableServerId(servers, 'creative', 'campak')).toBe('creative');
  });

  it('keeps an existing selection and otherwise falls back to the first real server', () => {
    expect(selectAvailableServerId(servers, 'missing', 'creative')).toBe('creative');
    expect(selectAvailableServerId(servers, 'missing', 'survival')).toBe('campak');
    expect(selectAvailableServerId([], 'missing', 'survival')).toBe('');
  });

  it('resolves the initial route from the context assigned in the same async turn', () => {
    expect(appSource).toContain('const context = currentNavigationContext();');
    expect(appSource).toContain('router.resolve(window.location.pathname, context)');
    expect(appSource).not.toContain('Acknowledge shell');
  });
});
