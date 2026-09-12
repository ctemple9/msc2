export const bundleIdentity = Object.freeze({
  id: 'msc2-shared-client',
  version: '0.1.6',
});

export function bundleLabel(): string {
  return `${bundleIdentity.id} v${bundleIdentity.version}`;
}
