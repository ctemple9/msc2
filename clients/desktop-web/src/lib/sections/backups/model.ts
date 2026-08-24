import type { Schema } from '../shared/types';

export const demoBackups: Schema['BackupItemDTO'][] = [
  {
    id: 'backup-1',
    displayName: 'Before Nether trip',
    isAutomatic: false,
    triggerReason: 'manual',
    modificationDate: '2026-08-24T10:30:00Z',
    fileSize: 768 * 1024 ** 2,
    slotName: 'Overworld',
    slotId: 'world-1',
  },
];

export const backupPaths = {
  list: '/v1/backups',
  now: '/v1/backups/now',
  restore: '/v1/backups/restore',
  delete: '/v1/backups/delete',
  config: '/v1/backups/config',
} as const;
