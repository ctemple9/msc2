import { parseRoute, routeScope } from './route';
import type { NavigationContext, RouteResolution, SectionDescriptor, SectionModule } from './types';

export class NavigationRegistry {
  private readonly descriptors = new Map<string, SectionDescriptor>();

  register(descriptor: SectionDescriptor): void {
    validateDescriptor(descriptor);
    if (this.descriptors.has(descriptor.id)) {
      throw new Error(`Section id '${descriptor.id}' is already registered`);
    }
    if (
      [...this.descriptors.values()].some(
        (existing) =>
          existing.segment === descriptor.segment && existing.scope === descriptor.scope,
      )
    ) {
      throw new Error(
        `Section route '${descriptor.scope}/${descriptor.segment}' is already registered`,
      );
    }
    this.descriptors.set(descriptor.id, descriptor);
  }

  registerAll(descriptors: readonly SectionDescriptor[]): void {
    for (const descriptor of descriptors) {
      this.register(descriptor);
    }
  }

  get(id: string): SectionDescriptor | undefined {
    return this.descriptors.get(id);
  }

  all(): readonly SectionDescriptor[] {
    return [...this.descriptors.values()];
  }

  visibleSections(context: NavigationContext): readonly SectionDescriptor[] {
    return this.all().filter((descriptor) => this.canAccess(descriptor, context));
  }

  resolve(pathname: string, context: NavigationContext): RouteResolution {
    const match = parseRoute(pathname);
    if (match.kind === 'root') {
      return { kind: 'root', match };
    }
    if (match.kind === 'invalid') {
      return { kind: 'invalid', match, reason: 'invalid-path' };
    }
    if (match.kind === 'reserved') {
      return { kind: 'reserved', match, reason: 'reserved-family' };
    }

    const descriptor = this.findForMatch(match);
    if (!descriptor) {
      return { kind: 'unknown', match, reason: 'missing-section' };
    }
    if (!this.canAccess(descriptor, context)) {
      return {
        kind: 'forbidden',
        match,
        descriptor,
        reason: this.hasPermissions(descriptor, context) ? 'capability' : 'permission',
      };
    }
    return { kind: 'section', match, descriptor };
  }

  async load(pathname: string, context: NavigationContext): Promise<SectionModule> {
    const resolution = this.resolve(pathname, context);
    if (resolution.kind !== 'section') {
      throw new Error(`Route '${pathname}' cannot load (${resolution.kind})`);
    }
    return resolution.descriptor.load();
  }

  private findForMatch(match: ReturnType<typeof parseRoute>): SectionDescriptor | undefined {
    if (!match.sectionSegment) {
      return undefined;
    }
    const scope = routeScope(match);
    return [...this.descriptors.values()].find(
      (descriptor) => descriptor.segment === match.sectionSegment && descriptor.scope === scope,
    );
  }

  private canAccess(descriptor: SectionDescriptor, context: NavigationContext): boolean {
    return (
      this.hasPermissions(descriptor, context) &&
      (descriptor.isAvailable?.(context.capabilities) ?? true) &&
      (descriptor.scope === 'host' || Boolean(context.serverId))
    );
  }

  private hasPermissions(descriptor: SectionDescriptor, context: NavigationContext): boolean {
    return (descriptor.requiredPermissions ?? []).every((permission) =>
      context.permissions.includes(permission),
    );
  }
}

function validateDescriptor(descriptor: SectionDescriptor): void {
  if (!descriptor.id || !descriptor.label || !descriptor.segment || !descriptor.load) {
    throw new Error('A section descriptor needs an id, label, segment, and lazy loader');
  }
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(descriptor.segment)) {
    throw new Error(`Section route '${descriptor.segment}' is not a stable URL segment`);
  }
}
