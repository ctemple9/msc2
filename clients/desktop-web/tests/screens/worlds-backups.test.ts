import { describe, expect, it } from 'vitest';
import {
  backupPaths,
  backupsForSlot,
  compatibleTargetServers,
  currentLevelName,
  demoBackups,
  demoSlots,
  defaultWorldSettingsValues,
  diffWorldSettings,
  formatBackupDay,
  formatDisplayName,
  groupBackupsByDay,
  legacyBackupReason,
  legacyImportName,
  legacyOrUnmatchedBackups,
  operationPath,
  placeholderHue,
  pollOperation,
  serversPath,
  settingsPath,
  slotThumbnailUrl,
  targetFormats,
  profileToWorldSettings,
  worldSettingsChanges,
  worldPaths,
} from '../../src/lib/sections/worlds/model';
import type { Schema, ScreenApi } from '../../src/lib/sections/shared/types';
import type { WorldProfile } from '../../src/lib/sections/worlds/model';

describe('routes -- DetailsWorldsTabView.swift is the real oracle', () => {
  it('exposes the real, frozen world-slot routes this tab actually calls', () => {
    expect(worldPaths).toMatchObject({
      list: '/v1/worlds',
      create: '/v1/worlds/create',
      rename: '/v1/worlds/rename',
      delete: '/v1/worlds/delete',
      activate: '/v1/worlds/activate',
      saveCurrent: '/v1/worlds/update',
      repair: '/v1/worlds/repair',
      convert: '/v1/worlds/convert',
      convertFormats: '/v1/worlds/convert/formats',
      import: '/v1/worlds/import',
      replaceActive: '/v1/worlds/replace-active-world',
      duplicate: '/v1/worlds/duplicate',
    });
    expect(worldPaths.profile('slot-1')).toBe('/v1/worlds/slot-1/profile');
    expect(worldPaths.thumbnail('slot-1')).toBe('/v1/worlds/slot-1/thumbnail');
    expect(settingsPath).toBe('/v1/settings');
  });

  it('exposes the real backup routes', () => {
    expect(backupPaths).toMatchObject({
      list: '/v1/backups',
      now: '/v1/backups/now',
      restore: '/v1/backups/restore',
      delete: '/v1/backups/delete',
      config: '/v1/backups/config',
    });
    expect(serversPath).toBe('/v1/servers');
    expect(operationPath('op-1')).toBe('/v1/operations/op-1');
  });

  it('keeps demo fixtures wired to each other (a backup belongs to a demo slot)', () => {
    expect(demoBackups[0].slotId).toBe(demoSlots[0].id);
  });
});

describe('world profiles -- settings stay with the slot', () => {
  const profile: WorldProfile = {
    schemaVersion: 1,
    identity: { name: 'Overworld', levelName: 'world', seed: '12345' },
    generation: {
      worldType: 'default',
      flatPreset: null,
      structures: true,
      biomeSource: null,
      generatorOptions: null,
      bonusChest: false,
      dataPacks: ['vanilla'],
    },
    gameplay: {
      difficulty: 'normal',
      defaultGameMode: 'survival',
      hardcore: false,
      commands: true,
      gamerules: { keepInventory: 'true' },
      cheats: null,
      experiments: {},
      coordinates: null,
      startingMap: null,
      supportedToggles: {},
    },
    safety: { state: 'safe', reasons: [] },
    fieldMetadata: {},
  };

  it('round-trips profile values through the shared form model', () => {
    const values = profileToWorldSettings(profile, demoSlots[0]);
    expect(values).toMatchObject({
      name: 'Overworld',
      seed: '12345',
      difficulty: 'normal',
      defaultGameMode: 'survival',
      gamerules: 'keepInventory=true',
    });
    expect(worldSettingsChanges(values, 'java')).toMatchObject({
      'identity.name': 'Overworld',
      'identity.seed': '12345',
      'generation.data-packs': ['vanilla'],
      'gameplay.gamerules': { keepInventory: 'true' },
    });
  });

  it('filters the other edition from a sparse update', () => {
    const bedrock = { ...defaultWorldSettingsValues('bedrock'), cheats: true };
    const changes = worldSettingsChanges(bedrock, 'bedrock');
    expect(changes).toHaveProperty('gameplay.cheats', true);
    expect(changes).not.toHaveProperty('generation.flat-preset');
    expect(changes).not.toHaveProperty('gameplay.hardcore');
  });

  it('only submits changed supported values when editing a slot', () => {
    const before = profileToWorldSettings(profile);
    const after = { ...before, difficulty: 'hard' };
    expect(diffWorldSettings(before, after, 'java')).toEqual({ 'gameplay.difficulty': 'hard' });
  });
});

describe('world slot thumbnails', () => {
  it('only points at the real thumbnail route once the slot actually has one', () => {
    const withThumbnail: Schema['WorldSlotDTO'] = { ...demoSlots[0], hasThumbnail: true };
    const withoutThumbnail: Schema['WorldSlotDTO'] = { ...demoSlots[0], hasThumbnail: false };
    expect(slotThumbnailUrl(withThumbnail)).toBe('/v1/worlds/slot-1/thumbnail');
    expect(slotThumbnailUrl(withoutThumbnail)).toBeUndefined();
  });

  it('derives a stable placeholder hue from the slot name (matches ActiveWorldCard.svelte)', () => {
    expect(placeholderHue('Overworld')).toBe(placeholderHue('Overworld'));
    expect(placeholderHue('Overworld')).not.toBe(placeholderHue('Nether Base'));
  });
});

describe('backups grouped and filtered per slot', () => {
  const backups: Schema['BackupItemDTO'][] = [
    {
      id: 'a.zip',
      displayName: 'a',
      isAutomatic: false,
      triggerReason: 'manual',
      modificationDate: '2026-08-24T10:00:00.000Z',
      slotId: 'slot-1',
      slotName: 'Overworld',
    },
    {
      id: 'b.zip',
      displayName: 'b',
      isAutomatic: true,
      triggerReason: 'auto',
      modificationDate: '2026-08-24T18:00:00.000Z',
      slotId: 'slot-1',
      slotName: 'Overworld',
    },
    {
      id: 'c.zip',
      displayName: 'c',
      isAutomatic: false,
      triggerReason: 'manual',
      modificationDate: '2026-08-20T10:00:00.000Z',
      slotId: 'slot-2',
      slotName: 'Before the Nether trip',
    },
    {
      id: 'legacy.zip',
      displayName: 'legacy',
      isAutomatic: false,
      triggerReason: 'manual',
      modificationDate: '2026-08-10T10:00:00.000Z',
      slotId: 'missing-slot',
      slotName: 'Deleted World',
    },
    {
      id: 'no-metadata.zip',
      displayName: 'no-metadata',
      isAutomatic: false,
      triggerReason: 'manual',
    },
  ];

  it('filters backups down to one slot', () => {
    expect(backupsForSlot(backups, 'slot-1')).toHaveLength(2);
    expect(backupsForSlot(backups, undefined)).toHaveLength(0);
  });

  it('groups same-slot backups by calendar day, most recent day first', () => {
    const groups = groupBackupsByDay(backupsForSlot(backups, 'slot-1'));
    expect(groups).toHaveLength(1);
    expect(groups[0].items).toHaveLength(2);
  });

  it('formats a grouped day label as a full weekday and date, no year (recent by construction)', () => {
    const groups = groupBackupsByDay(backupsForSlot(backups, 'slot-2'));
    expect(formatBackupDay(groups[0].day)).toBe('Thursday, August 20');
    expect(formatBackupDay('Unknown date')).toBe('Unknown date');
  });

  it('finds legacy/unmatched backups -- missing a known slot, or no slot metadata at all', () => {
    const legacy = legacyOrUnmatchedBackups(backups, demoSlots);
    expect(legacy.map((item) => item.id).sort()).toEqual(['legacy.zip', 'no-metadata.zip']);
  });

  it('explains why each legacy backup is unmatched', () => {
    expect(legacyBackupReason(backups[3])).toBe('Missing slot: Deleted World');
    expect(legacyBackupReason(backups[4])).toBe('Legacy backup (no slot metadata)');
  });

  it('names a legacy import after its recorded slot, else its own display name, else a flat fallback', () => {
    expect(legacyImportName(backups[3])).toBe('Deleted World');
    expect(legacyImportName({ ...backups[4], displayName: 'old-world.zip' })).toBe(
      'Imported old-world.zip',
    );
    expect(legacyImportName({ ...backups[4], displayName: '' })).toBe('Imported Backup');
  });
});

describe('conversion formats -- P12.4a exposes GET /v1/worlds/convert/formats for real', () => {
  const java: Schema['ServerDTO'] = {
    id: 'java-1',
    name: 'Java',
    directory: '/s/java',
    serverType: 'java',
  };
  const bedrock: Schema['ServerDTO'] = {
    id: 'bedrock-1',
    name: 'Bedrock',
    directory: '/s/bedrock',
    serverType: 'bedrock',
  };

  it('formats Chunker format strings the same way ChunkerManager.displayName(forFormat:) does', () => {
    expect(formatDisplayName('JAVA_1_21_0')).toBe('Java 1.21.0');
    expect(formatDisplayName('BEDROCK_R21_80')).toBe('Bedrock 1.21.80');
    expect(formatDisplayName('BEDROCK_R12')).toBe('Bedrock 1.12');
    expect(formatDisplayName('WEIRD_FORMAT')).toBe('WEIRD_FORMAT');
  });

  it('only offers the opposite edition of the raw list Chunker reports', () => {
    const formats = ['JAVA_1_20_0', 'JAVA_1_21_0', 'BEDROCK_R12', 'BEDROCK_R21_80'];
    expect(targetFormats(formats, java)).toEqual(['BEDROCK_R12', 'BEDROCK_R21_80']);
    expect(targetFormats(formats, bedrock)).toEqual(['JAVA_1_20_0', 'JAVA_1_21_0']);
    expect(targetFormats(formats, undefined)).toEqual(['BEDROCK_R12', 'BEDROCK_R21_80']);
  });
});

describe('conversion target servers -- WorldConversionWizardView.compatibleTargetServers', () => {
  const java: Schema['ServerDTO'] = {
    id: 'java-1',
    name: 'Java',
    directory: '/s/java',
    serverType: 'java',
  };
  const bedrock: Schema['ServerDTO'] = {
    id: 'bedrock-1',
    name: 'Bedrock',
    directory: '/s/bedrock',
    serverType: 'bedrock',
  };
  const otherJava: Schema['ServerDTO'] = {
    id: 'java-2',
    name: 'Java 2',
    directory: '/s/java2',
    serverType: 'java',
  };

  it('only offers the opposite edition, never the source server itself', () => {
    expect(compatibleTargetServers([java, bedrock, otherJava], java)).toEqual([bedrock]);
    expect(compatibleTargetServers([java, bedrock, otherJava], bedrock)).toEqual([java, otherJava]);
    expect(compatibleTargetServers([java, bedrock], undefined)).toEqual([]);
  });
});

describe('current level name -- P12.4k Replace World reads it back unchanged, like the oracle', () => {
  const section = (fields: Schema['SettingFieldDTO'][]): Schema['SettingsSectionDTO'] => ({
    id: 'bedrock',
    title: 'Bedrock',
    icon: 'cube',
    fields,
  });

  it('reads level-name off the settings response when a section exposes it (Bedrock)', () => {
    const settings: Schema['SettingsResponseDTO'] = {
      serverType: 'bedrock',
      serverName: 'Bedrock',
      serverRunning: false,
      editable: true,
      sections: [
        section([
          { key: 'level-name', label: 'Level Name', type: 'string', value: 'Bedrock level' },
        ]),
      ],
    };
    expect(currentLevelName(settings)).toBe('Bedrock level');
  });

  it("falls back to Minecraft's own default when no section exposes it (Java) or settings are unavailable", () => {
    const javaSettings: Schema['SettingsResponseDTO'] = {
      serverType: 'java',
      serverName: 'Java',
      serverRunning: false,
      editable: true,
      sections: [section([{ key: 'max-players', label: 'Max Players', type: 'int', value: '20' }])],
    };
    expect(currentLevelName(javaSettings)).toBe('world');
    expect(currentLevelName(undefined)).toBe('world');
  });

  it('falls back when the field is present but blank', () => {
    const settings: Schema['SettingsResponseDTO'] = {
      serverType: 'bedrock',
      serverName: 'Bedrock',
      serverRunning: false,
      editable: true,
      sections: [
        section([{ key: 'level-name', label: 'Level Name', type: 'string', value: '  ' }]),
      ],
    };
    expect(currentLevelName(settings)).toBe('world');
  });
});

describe('operation polling -- activate/convert/backup-now/restore are all operation-backed', () => {
  it('polls GET /v1/operations/{id} until a terminal state', async () => {
    const seen: string[] = [];
    let calls = 0;
    const api: ScreenApi = {
      get: async <T>(): Promise<T> => {
        calls += 1;
        seen.push('poll');
        const operation: Schema['OperationDTO'] =
          calls < 2
            ? { id: 'op-1', type: 'world-activate', state: 'running' }
            : { id: 'op-1', type: 'world-activate', state: 'succeeded' };
        return operation as unknown as T;
      },
      post: async () => {
        throw new Error('not used');
      },
    };
    const result = await pollOperation(api, 'op-1', (operation) => seen.push(operation.state), 0);
    expect(result?.state).toBe('succeeded');
    expect(calls).toBe(2);
    expect(seen).toEqual(['poll', 'running', 'poll', 'succeeded']);
  });

  it('returns undefined without a connected agent instead of polling forever', async () => {
    expect(await pollOperation(undefined, 'op-1')).toBeUndefined();
  });
});
