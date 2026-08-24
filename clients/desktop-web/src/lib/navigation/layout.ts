import type { LayoutMode, NavigationContext, SectionDescriptor } from './types';
import type { NavigationRegistry } from './registry';

export const DEFAULT_NARROW_BREAKPOINT = 760;

export function layoutForWidth(
  width: number,
  narrowBreakpoint = DEFAULT_NARROW_BREAKPOINT,
): LayoutMode {
  return width < narrowBreakpoint ? 'narrow' : 'wide';
}

export function sectionsForLayout(
  registry: NavigationRegistry,
  context: NavigationContext,
  width: number,
): { mode: LayoutMode; sections: readonly SectionDescriptor[] } {
  return {
    mode: layoutForWidth(width),
    sections: registry.visibleSections(context),
  };
}
