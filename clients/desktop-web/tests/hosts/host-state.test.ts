import { describe, expect, it } from 'vitest';
import { HostStore, type CredentialAdapter } from '../../src/lib/hosts';

const host = (id: string) => ({ id, label: id.toUpperCase(), baseUrl: `http://${id}.test` });

describe('host-scoped client state', () => {
  it('keeps caches and active servers isolated when switching hosts', () => {
    const store = new HostStore();
    store.addHost(host('alpha'));
    store.addHost(host('beta'));
    store.setServers('alpha', [
      { id: 'alpha-server', name: 'Alpha', directory: '/alpha', serverType: 'paper' },
    ]);
    store.setServers('beta', [
      { id: 'beta-server', name: 'Beta', directory: '/beta', serverType: 'vanilla' },
    ]);
    store.appendConsole('alpha', {
      source: 'server',
      text: 'alpha only',
      ts: '2026-08-24T00:00:00Z',
    });
    store.selectHost('beta');

    expect(store.getSelectedState()?.cache.activeServerId).toBe('beta-server');
    expect(store.getState('beta').cache.consoleLines).toEqual([]);
    expect(store.getState('alpha').cache.consoleLines[0].text).toBe('alpha only');
  });

  it('requires host identity in destructive confirmations', () => {
    const store = new HostStore();
    store.addHost(host('alpha'));
    store.addHost(host('beta'));
    const confirmation = store.makeDestructiveConfirmation('alpha', 'delete-server', 'a1');

    expect(() => store.assertDestructiveConfirmation(confirmation, 'beta', 'a1')).toThrow(
      'different host',
    );
    expect(() => store.assertDestructiveConfirmation(confirmation, 'alpha', 'a1')).not.toThrow();
  });

  it('injects credentials without storing them in host state', async () => {
    const adapter: CredentialAdapter = {
      headersFor: async (record) => ({ Authorization: `Bearer secret-for-${record.id}` }),
    };
    const store = new HostStore({ credentialAdapter: adapter });
    store.addHost(host('alpha'));

    expect(await store.credentialHeaders('alpha')).toEqual({
      Authorization: 'Bearer secret-for-alpha',
    });
    expect(JSON.stringify(store.getState('alpha'))).not.toContain('secret-for-alpha');
  });
});
