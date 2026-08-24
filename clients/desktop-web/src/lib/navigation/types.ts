import type { components } from '../api/generated';

export type Capabilities = components['schemas']['CapabilitiesDTO'];
export type LayoutMode = 'narrow' | 'wide';
export type SectionScope = 'host' | 'server';

export type SectionModule = {
  // The shell treats the loaded value as a Svelte component without coupling
  // the registry to a particular rendering surface.
  default: unknown;
};

export type SectionLoader = () => Promise<SectionModule>;
export type CapabilityPredicate = (capabilities: Capabilities | null | undefined) => boolean;

export type NavigationContext = {
  hostId: string;
  serverId?: string;
  permissions: readonly string[];
  capabilities?: Capabilities | null;
};

export type SectionDescriptor = {
  id: string;
  label: string;
  segment: string;
  // A later feature group may claim a reserved URL family without changing the
  // shell or treating the family name as a closed client-side enum.
  routeFamily?: string;
  scope: SectionScope;
  requiredPermissions?: readonly string[];
  isAvailable?: CapabilityPredicate;
  load: SectionLoader;
};

export type RouteMatch = {
  kind: 'root' | 'section' | 'unknown' | 'reserved' | 'invalid';
  pathname: string;
  hostId?: string;
  serverId?: string;
  sectionSegment?: string;
  tail: readonly string[];
  reservedFamily?: 'bedrock' | 'profiles';
};

export type RouteResolution =
  | {
      kind: 'section';
      match: RouteMatch;
      descriptor: SectionDescriptor;
    }
  | {
      kind: 'unknown' | 'reserved' | 'invalid' | 'root' | 'forbidden';
      match: RouteMatch;
      descriptor?: SectionDescriptor;
      reason?: 'missing-section' | 'reserved-family' | 'invalid-path' | 'permission' | 'capability';
    };
