import type { components } from '../api/generated';
import type { Capabilities, CapabilityPredicate } from './types';

type BedrockCapabilityAdvertisement =
  components['schemas']['CapabilitiesDTO']['serverTypes']['bedrock'];
type BedrockRuntimeState = components['schemas']['BedrockRuntimeStateDTO'];

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

export function hasUsableBedrockRuntime(capabilities: Capabilities | null | undefined): boolean {
  const bedrock: BedrockCapabilityAdvertisement | undefined = capabilities?.serverTypes.bedrock;
  if (!bedrock?.supported) {
    return false;
  }

  const runtime: BedrockRuntimeState | undefined = bedrock.runtime;
  return runtime?.state === undefined || runtime.state === 'available';
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
