import { BrowserSessionAuth } from './browser';

interface BrowserHandoffLocation {
  readonly hash: string;
  readonly origin: string;
  readonly pathname: string;
  readonly search: string;
}

interface BrowserHandoffHistory {
  replaceState(data: unknown, unused: string, url?: string | URL | null): void;
}

interface BrowserHandoffSession {
  exchangePairingCode(pairingCode: string): Promise<void>;
}

/**
 * Exchanges a desktop-created browser pairing before normal client startup.
 * The fragment is local to the browser, then removed before any app request.
 */
export async function redeemBrowserHandoff(
  location: BrowserHandoffLocation,
  history: BrowserHandoffHistory,
  session: BrowserHandoffSession = new BrowserSessionAuth(location.origin),
): Promise<boolean> {
  const pairingCode = new URLSearchParams(location.hash.slice(1)).get('browser-pairing');
  if (!pairingCode?.startsWith('pair_')) return false;

  history.replaceState({}, '', `${location.pathname}${location.search}`);
  await session.exchangePairingCode(pairingCode);
  return true;
}
