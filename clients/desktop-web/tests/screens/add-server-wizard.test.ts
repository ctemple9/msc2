import { describe, expect, it } from 'vitest';
import {
  buildServerCreateRequest,
  canCreateServer,
  defaultWizardDraft,
  hasAddOnsStep,
  javaAddOnKind,
  wizardStepLabels,
} from '../../src/lib/sections/fleet/wizard/model';

describe('add server wizard step labels', () => {
  it('walks the Fresh path sequence without an Add-ons step', () => {
    expect(wizardStepLabels('fresh')).toEqual([
      'Choose path',
      'Configure',
      'Network',
      'World',
      'Confirm',
    ]);
  });

  it('inserts Add-ons between World and Confirm when the flavor accepts add-ons', () => {
    expect(wizardStepLabels('fresh', true)).toEqual([
      'Choose path',
      'Configure',
      'Network',
      'World',
      'Add-ons',
      'Confirm',
    ]);
  });

  it('walks the Import Existing path sequence', () => {
    expect(wizardStepLabels('importExisting')).toEqual([
      'Choose path',
      'Upload',
      'Review',
      'Network',
      'Confirm',
    ]);
  });
});

describe('add server wizard add-ons eligibility', () => {
  it('mirrors JavaServerFlavor.swift addOnKind: vanilla has none, standard is plugin, modded is mod', () => {
    expect(javaAddOnKind('vanilla')).toBeUndefined();
    expect(javaAddOnKind('paper')).toBe('plugin');
    expect(javaAddOnKind('purpur')).toBe('plugin');
    expect(javaAddOnKind('fabric')).toBe('mod');
    expect(javaAddOnKind('neoforge')).toBe('mod');
    expect(javaAddOnKind('forge')).toBe('mod');
  });

  it('mirrors hasAddOnsStep: Java flavors with an add-on kind get the step, Bedrock and Vanilla do not', () => {
    const javaPaper = {
      ...defaultWizardDraft(),
      serverType: 'java' as const,
      javaFlavor: 'paper' as const,
    };
    expect(hasAddOnsStep(javaPaper)).toBe(true);

    const javaVanilla = {
      ...defaultWizardDraft(),
      serverType: 'java' as const,
      javaFlavor: 'vanilla' as const,
    };
    expect(hasAddOnsStep(javaVanilla)).toBe(false);

    const bedrock = { ...defaultWizardDraft(), serverType: 'bedrock' as const };
    expect(hasAddOnsStep(bedrock)).toBe(false);
  });
});

describe('add server wizard confirm step (P12.18g)', () => {
  it('mirrors AddServerWizardView.swift canCreate: a non-blank display name only', () => {
    expect(canCreateServer('')).toBe(false);
    expect(canCreateServer('   ')).toBe(false);
    expect(canCreateServer('My Server')).toBe(true);
  });

  it('builds the real POST /v1/servers/create body for a Java flavor with a pinned version and crossplay', () => {
    const draft = {
      ...defaultWizardDraft(),
      serverName: 'Draft Name',
      javaFlavor: 'fabric' as const,
      javaCategory: 'modded' as const,
      versionId: 'version-7',
      enableCrossPlay: true,
      crossPlayBedrockPort: 19133,
      worldName: 'My World',
      worldSeed: '12345',
    };
    expect(buildServerCreateRequest(draft, 'Display Name')).toEqual({
      name: 'Display Name',
      serverType: 'java',
      enablePlayit: false,
      enableXboxBroadcast: false,
      difficulty: 'normal',
      gamemode: 'survival',
      worldName: 'My World',
      worldSeed: '12345',
      javaFlavor: 'fabric',
      versionId: 'version-7',
      port: 25565,
      enableCrossPlay: true,
      crossPlayBedrockPort: 19133,
    });
  });

  it('falls back to the draft server name when the display name is blank, and omits unset optionals', () => {
    const draft = { ...defaultWizardDraft(), serverName: 'Fallback Name' };
    const body = buildServerCreateRequest(draft, '   ');
    expect(body.name).toBe('Fallback Name');
    expect(body).not.toHaveProperty('worldName');
    expect(body).not.toHaveProperty('worldSeed');
    expect(body).not.toHaveProperty('versionId');
    expect(body).not.toHaveProperty('crossPlayBedrockPort');
  });

  it('builds the Bedrock branch with bedrockVersion/maxPlayers/port instead of javaFlavor', () => {
    const draft = {
      ...defaultWizardDraft(),
      serverType: 'bedrock' as const,
      bedrockVersion: '  1.21.0  ',
      bedrockMaxPlayers: 25,
      bedrockPort: 19140,
    };
    const body = buildServerCreateRequest(draft, 'Bedrock Server');
    expect(body).toMatchObject({
      serverType: 'bedrock',
      bedrockVersion: '1.21.0',
      maxPlayers: 25,
      port: 19140,
    });
    expect(body).not.toHaveProperty('javaFlavor');
  });

  it('redeems a staged modpack as stagedModpackUploadId only for the Java branch', () => {
    const draft = {
      ...defaultWizardDraft(),
      stagedModpack: {
        fileName: 'pack.mrpack',
        stagedUploadId: 'upload-42',
        inspection: {
          success: true,
          message: 'ok',
          format: 'mrpack' as const,
          fileCount: 3,
          clientOnlyFileCount: 0,
          manualFiles: [],
        },
      },
    };
    expect(buildServerCreateRequest(draft, 'Packed Server').stagedModpackUploadId).toBe(
      'upload-42',
    );
  });
});
