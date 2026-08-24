import { describe, expect, it } from 'vitest';
import { bundleIdentity, bundleLabel } from './bundle-identity';

describe('shared bundle identity', () => {
  it('has a stable identity for desktop and browser delivery', () => {
    expect(bundleIdentity).toEqual({ id: 'msc2-shared-client', version: '0.1.0' });
    expect(bundleLabel()).toBe('msc2-shared-client v0.1.0');
  });
});
