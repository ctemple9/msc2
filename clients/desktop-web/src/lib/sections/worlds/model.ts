import type { Schema } from '../shared/types';

export const demoWorlds: Schema['WorldSlotDTO'][] = [
  {
    id: 'world-1',
    name: 'Overworld',
    createdAt: '2026-08-20T12:00:00Z',
    isActive: true,
    hasThumbnail: false,
    worldSeed: '—',
    zipSizeBytes: 1024 ** 3,
  },
  {
    id: 'world-2',
    name: 'Before the Nether trip',
    createdAt: '2026-08-18T12:00:00Z',
    isActive: false,
    hasThumbnail: true,
    zipSizeBytes: 480 * 1024 ** 2,
  },
];

export const worldPaths = {
  list: '/v1/worlds',
  create: '/v1/worlds/create',
  rename: '/v1/worlds/rename',
  duplicate: '/v1/worlds/duplicate',
  delete: '/v1/worlds/delete',
  import: '/v1/worlds/import',
  export: '/v1/worlds/export',
  activate: '/v1/worlds/activate',
  replaceActive: '/v1/worlds/replace-active-world',
  convert: '/v1/worlds/convert',
} as const;
