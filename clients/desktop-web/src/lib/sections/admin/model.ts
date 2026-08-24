export const administrationPaths = {
  settings: ['/v1/settings', '/v1/config/ram', '/v1/config/java-runtime', '/v1/config/geyser'],
  health: ['/v1/health', '/v1/health/problems', '/v1/health/repair'],
  connectivity: [
    '/v1/connectivity',
    '/v1/playit',
    '/v1/duckdns',
    '/v1/resourcepacks',
    '/v1/broadcast/status',
  ],
  access: ['/v1/users', '/v1/users/update', '/v1/users/revoke'],
} as const;

export function isSchemaDriven(fields: readonly { key: string }[]): boolean {
  return fields.length === 0 || fields.every((field) => field.key.trim().length > 0);
}
