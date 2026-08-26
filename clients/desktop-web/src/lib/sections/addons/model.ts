import type { Schema } from '../shared/types';

// AddonItemDTO.bucket is the resolver's own update-resolution bucket
// (crates/msc-agent/src/routes/components.rs's addon_bucket_name), not a
// mod/plugin/component category -- a single /v1/addons response is always
// all-mods or all-plugins for the active server, never mixed. Kept the demo
// fixtures aligned with the real enum (updateAvailable/noCompatibleVersion/
// upToDate/unlinked) rather than the invented category strings this file
// shipped with previously, since ComponentsSection now derives its
// "Update available" affordance directly from this field.
export const demoAddons: Schema['AddonItemDTO'][] = [
  {
    jarStem: 'lithium',
    displayName: 'Lithium',
    bucket: 'updateAvailable',
    currentVersion: '0.13.0',
    availableVersion: '0.13.1',
    isEnabled: true,
    projectId: 'lithium',
  },
  {
    jarStem: 'geyser',
    displayName: 'Geyser',
    bucket: 'upToDate',
    currentVersion: '2.5.0',
    isEnabled: true,
  },
];

export const addonPaths = {
  list: '/v1/addons',
  search: '/v1/catalog/search?q=',
  install: '/v1/components/install',
  update: '/v1/components/update',
  remove: '/v1/components/remove',
  export: '/v1/components/client-export',
  inspectPack: '/v1/modpacks/inspect',
  importPack: '/v1/modpacks/import',
  manualFile: (operationId: string): string =>
    `/v1/modpacks/${encodeURIComponent(operationId)}/manual-file`,
} as const;

/** "Update available" is the only bucket with a real update to offer;
 *  noCompatibleVersion means the resolver found one but it doesn't match
 *  this server's loader/game version, so no update button either. */
export function addonStatusLabel(addon: Schema['AddonItemDTO']): string | undefined {
  if (addon.bucket === 'updateAvailable') return 'Update available';
  if (addon.bucket === 'noCompatibleVersion') return 'No compatible update';
  return undefined;
}
