import type { Capabilities, CapabilityPredicate } from './types';

export function hasPermission(permission: string): (permissions: readonly string[]) => boolean {
  return (permissions) => permissions.includes(permission);
}

export function hasCapability(path: readonly string[], expected = true): CapabilityPredicate {
  return (capabilities) => readCapability(capabilities, path) === expected;
}

export function capabilityValue<T>(
  path: readonly string[],
  predicate: (value: T) => boolean,
): CapabilityPredicate {
  return (capabilities) => predicate(readCapability(capabilities, path) as T);
}

function readCapability(
  capabilities: Capabilities | null | undefined,
  path: readonly string[],
): unknown {
  let value: unknown = capabilities;
  for (const key of path) {
    if (!value || typeof value !== 'object') {
      return undefined;
    }
    value = (value as Record<string, unknown>)[key];
  }
  return value;
}
