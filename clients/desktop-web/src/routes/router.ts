import { NavigationRegistry } from '../lib/navigation/registry';
import type { SectionDescriptor } from '../lib/navigation/types';

export function createClientRouter(
  descriptors: readonly SectionDescriptor[] = [],
): NavigationRegistry {
  const registry = new NavigationRegistry();
  registry.registerAll(descriptors);
  return registry;
}
