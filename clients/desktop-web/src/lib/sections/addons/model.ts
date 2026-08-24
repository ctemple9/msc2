import type { Schema } from '../shared/types';

export const demoAddons: Schema['AddonItemDTO'][] = [
  {
    jarStem: 'lithium',
    displayName: 'Lithium',
    bucket: 'mod',
    currentVersion: '0.13.0',
    availableVersion: '0.13.1',
    isEnabled: true,
    projectId: 'lithium',
  },
  {
    jarStem: 'geyser',
    displayName: 'Geyser',
    bucket: 'component',
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
} as const;
