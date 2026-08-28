import { describe, expect, it } from 'vitest';
import { clearClientPreferences, HostStore } from '../../src/lib/hosts';

const host = (id: string) => ({ id, label: id.toUpperCase(), baseUrl: `http://${id}.test` });

describe('client reset state', () => {
  it('resets every in-memory host record and selected-host state together', () => {
    const store = new HostStore();
    store.addHost(host('alpha'));
    store.addHost(host('beta'));
    store.selectHost('beta');

    store.reset();

    expect(store.listHosts()).toEqual([]);
    expect(store.selectedHost).toBeNull();
    expect(() => store.getState('alpha')).toThrow('Unknown host');
  });

  it('clears only MSC-owned client storage keys', () => {
    if (typeof localStorage === 'undefined' || typeof localStorage.setItem !== 'function') return;
    localStorage.setItem('msc.accent', 'blue');
    localStorage.setItem('msc2.bannerColor.alpha.server', '#666679');
    localStorage.setItem('msc_onboarding_tour_complete', 'true');
    localStorage.setItem('other-app.preference', 'keep');

    clearClientPreferences();

    expect(localStorage.getItem('msc.accent')).toBeNull();
    expect(localStorage.getItem('msc2.bannerColor.alpha.server')).toBeNull();
    expect(localStorage.getItem('msc_onboarding_tour_complete')).toBeNull();
    expect(localStorage.getItem('other-app.preference')).toBe('keep');
  });
});
