// Bedrock avatar lookup ported from MSC 1's BedrockSkinFetcher.swift.
// Resolution is client-side because these public endpoints return only skin
// identity data; no server credentials are involved.

const JOIN_CACHE_URL = 'https://api.geysermc.org/v2/utils/uuid/bedrock_or_java';
const XUID_URL = 'https://api.geysermc.org/v2/xbox/xuid';
const LOOKUP_TIMEOUT_MS = 8000;
const IMAGE_TIMEOUT_MS = 10000;
const FAILURE_TTL_MS = 120000;

const resolvedUUIDs = new Map<string, string>();
const failedLookups = new Map<string, number>();

export function dottedGamertag(gamertag: string): string {
  return gamertag.startsWith('.') ? gamertag : `.${gamertag}`;
}

export function xboxGamertagCandidates(gamertag: string): string[] {
  const candidates = [gamertag];
  if (gamertag.includes('_')) candidates.push(gamertag.replaceAll('_', ' '));
  return candidates;
}

export function floodgateUuidFromXuid(xuid: string): string | undefined {
  try {
    const value = BigInt(xuid);
    if (value < 0n || value > 0xffffffffffffffffn) return undefined;
    const hex = value.toString(16).padStart(16, '0');
    return `00000000-0000-0000-${hex.slice(0, 4)}-${hex.slice(4)}`;
  } catch {
    return undefined;
  }
}

export function bedrockBodyFallbackUrl(gamertag: string): string {
  return `https://api.mcheads.org/body/${encodeURIComponent(dottedGamertag(gamertag))}/160`;
}

async function getText(url: string, timeoutMs: number): Promise<string | undefined> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(url, { signal: controller.signal });
    if (!response.ok) return undefined;
    return await response.text();
  } catch {
    return undefined;
  } finally {
    clearTimeout(timeout);
  }
}

function normalizedUuid(value: string | undefined): string | undefined {
  const compact = value?.replaceAll('-', '').toLowerCase();
  if (!compact || !/^[0-9a-f]{32}$/.test(compact)) return undefined;
  return `${compact.slice(0, 8)}-${compact.slice(8, 12)}-${compact.slice(12, 16)}-${compact.slice(16, 20)}-${compact.slice(20)}`;
}

async function resolveViaJoinCache(dotted: string): Promise<string | undefined> {
  const url = `${JOIN_CACHE_URL}/${encodeURIComponent(dotted)}?prefix=.`;
  const text = await getText(url, LOOKUP_TIMEOUT_MS);
  if (!text) return undefined;
  try {
    const body = JSON.parse(text) as { id?: string };
    return normalizedUuid(body.id);
  } catch {
    return undefined;
  }
}

async function resolveViaXboxLive(gamertag: string): Promise<string | undefined> {
  const text = await getText(`${XUID_URL}/${encodeURIComponent(gamertag)}`, LOOKUP_TIMEOUT_MS);
  const xuid = text?.match(/"xuid"\s*:\s*"?([0-9]+)"?/)?.[1];
  return xuid ? floodgateUuidFromXuid(xuid) : undefined;
}

async function resolveFloodgateUuid(gamertag: string): Promise<string | undefined> {
  const dotted = dottedGamertag(gamertag);
  const cacheKey = dotted.toLowerCase();
  const cached = resolvedUUIDs.get(cacheKey);
  if (cached) return cached;

  const failedAt = failedLookups.get(cacheKey);
  if (failedAt !== undefined && Date.now() - failedAt < FAILURE_TTL_MS) return undefined;
  failedLookups.delete(cacheKey);

  const fromJoinCache = await resolveViaJoinCache(dotted);
  if (fromJoinCache) {
    resolvedUUIDs.set(cacheKey, fromJoinCache);
    return fromJoinCache;
  }

  const rawGamertag = dotted.slice(1);
  for (const candidate of xboxGamertagCandidates(rawGamertag)) {
    const fromXbox = await resolveViaXboxLive(candidate);
    if (fromXbox) {
      resolvedUUIDs.set(cacheKey, fromXbox);
      return fromXbox;
    }
  }

  failedLookups.set(cacheKey, Date.now());
  return undefined;
}

function imageLoads(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    const image = new Image();
    let settled = false;
    const timeout = setTimeout(() => finish(false), IMAGE_TIMEOUT_MS);

    function finish(loaded: boolean): void {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      image.onload = null;
      image.onerror = null;
      resolve(loaded);
    }

    image.onload = () => finish(true);
    image.onerror = () => finish(false);
    image.src = url;
  });
}

export async function fetchBedrockBodyUrl(gamertag: string): Promise<string | undefined> {
  const dotted = dottedGamertag(gamertag.trim());
  const uuid = await resolveFloodgateUuid(dotted);
  if (uuid) {
    const uuidUrl = `https://mc-heads.net/body/${uuid.replaceAll('-', '')}/160`;
    if (await imageLoads(uuidUrl)) return uuidUrl;
  }

  const fallback = bedrockBodyFallbackUrl(dotted);
  return (await imageLoads(fallback)) ? fallback : undefined;
}
