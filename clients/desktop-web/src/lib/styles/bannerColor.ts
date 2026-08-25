// Per-server identity accent — MSC 1's `bannerColor`. The agent contract has no
// banner-color field yet (ServerDTO carries no such property), so this is held
// as client-local state per host+server until a real field exists. Mirrors MSC 1's
// clampedAwayFromWhite/clampedAwayFromBlack guard rails (ContentView.swift) so the
// wash stays visible on the dark chrome shell regardless of input.
const STORAGE_PREFIX = 'msc2.bannerColor.';

// MSC 1's own clampedAwayFromBlack() replacement color — the "no color set" default.
export const DEFAULT_BANNER_COLOR = '#666679';

function parseHex(hex: string): [number, number, number] | null {
  const match = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!match) return null;
  const value = parseInt(match[1], 16);
  return [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
}

function toHex([r, g, b]: readonly [number, number, number]): string {
  return `#${[r, g, b]
    .map((c) =>
      Math.max(0, Math.min(255, Math.round(c)))
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`;
}

export function clampBannerColor(hex: string): string {
  const rgb = parseHex(hex);
  if (!rgb) return DEFAULT_BANNER_COLOR;
  const [r, g, b] = rgb;
  if (r > 245 && g > 245 && b > 245) return '#ebebeb';
  const luminance = 0.2126 * (r / 255) + 0.7152 * (g / 255) + 0.0722 * (b / 255);
  if (luminance < 0.05) return DEFAULT_BANNER_COLOR;
  return toHex(rgb);
}

export function bannerColorAccent(hex: string, alpha: number): string {
  const rgb = parseHex(hex) ?? parseHex(DEFAULT_BANNER_COLOR)!;
  return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${alpha})`;
}

function storageKey(hostId: string, serverId: string): string {
  return `${STORAGE_PREFIX}${hostId}.${serverId}`;
}

export function bannerColorFor(hostId: string, serverId: string): string {
  if (typeof localStorage === 'undefined') return DEFAULT_BANNER_COLOR;
  const stored = localStorage.getItem(storageKey(hostId, serverId));
  return stored ? clampBannerColor(stored) : DEFAULT_BANNER_COLOR;
}

export function setBannerColorFor(hostId: string, serverId: string, hex: string): void {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(storageKey(hostId, serverId), clampBannerColor(hex));
}
