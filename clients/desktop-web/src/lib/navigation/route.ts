import type { SectionDescriptor, SectionScope, RouteMatch } from './types';

const RESERVED_FAMILIES = {
  bedrock: 'bedrock',
  profiles: 'profiles',
} as const;

export const RESERVED_ROUTE_FAMILIES = Object.freeze(Object.keys(RESERVED_FAMILIES));

export function parseRoute(pathname: string): RouteMatch {
  const cleanPath = pathname.split(/[?#]/, 1)[0] || '/';
  const segments = cleanPath.split('/').filter(Boolean).map(decodeSegment);

  if (segments.some((segment) => segment === null)) {
    return { kind: 'invalid', pathname: cleanPath, tail: [] };
  }

  const decoded = segments as string[];
  if (decoded.length === 0) {
    return { kind: 'root', pathname: cleanPath, tail: [] };
  }

  if (decoded[0] !== 'hosts' || !decoded[1]) {
    return { kind: 'invalid', pathname: cleanPath, tail: [] };
  }

  const hostId = decoded[1];
  const hasServer = decoded[2] === 'servers';
  const sectionIndex = hasServer ? 4 : 2;
  const serverId = hasServer ? decoded[3] : undefined;
  const sectionSegment = decoded[sectionIndex];

  if (!sectionSegment || (hasServer && !serverId)) {
    return { kind: 'invalid', pathname: cleanPath, hostId, serverId, tail: [] };
  }

  const tail = decoded.slice(sectionIndex + 1);
  const reservedFamily = RESERVED_FAMILIES[sectionSegment as keyof typeof RESERVED_FAMILIES];
  if (reservedFamily) {
    return {
      kind: 'reserved',
      pathname: cleanPath,
      hostId,
      serverId,
      sectionSegment,
      tail,
      reservedFamily,
    };
  }

  return {
    kind: 'unknown',
    pathname: cleanPath,
    hostId,
    serverId,
    sectionSegment,
    tail,
  };
}

export function buildSectionPath(
  descriptor: Pick<SectionDescriptor, 'scope' | 'segment'>,
  hostId: string,
  serverId?: string,
  tail: readonly string[] = [],
): string {
  const parts = ['hosts', hostId];
  if (descriptor.scope === 'server') {
    if (!serverId) {
      throw new Error(`Server-scoped route '${descriptor.segment}' requires a serverId`);
    }
    parts.push('servers', serverId);
  }
  parts.push(descriptor.segment, ...tail);
  return `/${parts.map(encodeURIComponent).join('/')}`;
}

export function routeScope(match: RouteMatch): SectionScope | undefined {
  return match.serverId ? 'server' : 'host';
}

function decodeSegment(segment: string): string | null {
  try {
    const decoded = decodeURIComponent(segment);
    // Encoded slashes belong to an ID, not to the route tree. Splitting before
    // decoding keeps `/` in a host or server ID from changing its scope.
    return decoded || null;
  } catch {
    return null;
  }
}
