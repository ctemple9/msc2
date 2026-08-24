export const bundleIdentity = Object.freeze({
  id: 'msc2-shared-client',
  version: '0.1.0',
});

export function bundleLabel(): string {
  return `${bundleIdentity.id} v${bundleIdentity.version}`;
}
