import { describe, expect, it, vi } from 'vitest';
import { redeemBrowserHandoff } from '../../src/lib/auth/browser-handoff';

describe('browser handoff', () => {
  it('exchanges a desktop-created pairing from the fragment and removes it from history', async () => {
    const exchangePairingCode = vi.fn(async () => undefined);
    const replaceState = vi.fn();

    await expect(
      redeemBrowserHandoff(
        {
          origin: 'http://127.0.0.1:48001',
          pathname: '/server/local-agent/survival/home',
          search: '?from=desktop',
          hash: '#browser-pairing=pair_one%2Ftime',
        },
        { replaceState },
        { exchangePairingCode },
      ),
    ).resolves.toBe(true);

    expect(exchangePairingCode).toHaveBeenCalledWith('pair_one/time');
    expect(replaceState).toHaveBeenCalledWith(
      {},
      '',
      '/server/local-agent/survival/home?from=desktop',
    );
  });

  it('leaves ordinary browser links alone', async () => {
    const exchangePairingCode = vi.fn(async () => undefined);
    const replaceState = vi.fn();

    await expect(
      redeemBrowserHandoff(
        { origin: 'http://127.0.0.1:48001', pathname: '/', search: '', hash: '#details' },
        { replaceState },
        { exchangePairingCode },
      ),
    ).resolves.toBe(false);

    expect(exchangePairingCode).not.toHaveBeenCalled();
    expect(replaceState).not.toHaveBeenCalled();
  });
});
