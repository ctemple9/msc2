import { describe, expect, it } from 'vitest';
import createWorldSource from '../../src/lib/sections/worlds/CreateWorldSheet.svelte?raw';
import formSource from '../../src/lib/sections/worlds/WorldSettingsForm.svelte?raw';
import settingsSource from '../../src/lib/sections/settings/SettingsSection.svelte?raw';
import worldsSource from '../../src/lib/sections/worlds/WorldsSection.svelte?raw';
import {
  defaultWorldSettingsValues,
  diffWorldSettings,
  worldSettingsChanges,
} from '../../src/lib/sections/worlds/model';

describe('world-settings release gate', () => {
  it('uses one form for the fresh-server and Worlds-tab creation paths', () => {
    expect(formSource).toContain('Essentials');
    expect(createWorldSource).toContain('WorldSettingsForm');
    expect(worldsSource).toContain('WorldSettingsForm');
    expect(worldsSource).toContain('worldPaths.profile');
    expect(formSource).toContain('These settings are saved with this world');
  });

  it('keeps server-wide force-gamemode separate from the active world profile', () => {
    expect(settingsSource).toContain('force-gamemode');
    expect(settingsSource).toContain('Applies to every world and can override saved defaults.');
    expect(settingsSource).toContain('forceGamemodeConfirmation');
    expect(settingsSource).toContain('confirmForceGamemode');
    expect(settingsSource).toContain("'server_force_gamemode'");
    expect(settingsSource).not.toContain('Edit active world settings in Worlds');
    expect(formSource).toContain('serverSettingsHref');
    expect(formSource).toContain('The agent will require an acknowledgement');
  });

  it('does not send fields that the selected edition or runtime cannot support', () => {
    const java = { ...defaultWorldSettingsValues('java'), flatPreset: 'minecraft:plains' };
    const bedrock = { ...defaultWorldSettingsValues('bedrock'), cheats: true };
    const unsupportedJava = {
      context: { serverType: 'java', minecraftVersion: '1.19.4', nativeCapabilities: [] },
      fields: {
        'generation.flat-preset': {
          capability: 'world.java',
          state: 'unsupported',
          available: false,
          reason: 'Requires Minecraft 1.20 or newer',
        },
      },
      thirdParty: {
        available: true,
        label: 'Provided by this server/mod',
        message: 'Use the server configuration path.',
        handoff: 'server_settings',
      },
    };

    expect(worldSettingsChanges(java, 'java', unsupportedJava)).not.toHaveProperty(
      'generation.flat-preset',
    );
    expect(worldSettingsChanges(bedrock, 'bedrock')).toHaveProperty('gameplay.cheats', true);
    expect(worldSettingsChanges(bedrock, 'bedrock')).not.toHaveProperty('gameplay.hardcore');
    expect(formSource).toContain('Some properties are unknown to this MSC version');
    expect(formSource).toContain('capabilities.thirdParty');
  });

  it('keeps two slot profiles distinct while reporting restart timing', () => {
    const survival = defaultWorldSettingsValues('java');
    const creative = { ...survival, difficulty: 'hard', defaultGameMode: 'creative' };

    expect(diffWorldSettings(survival, creative, 'java')).toEqual({
      'gameplay.difficulty': 'hard',
      'gameplay.default-game-mode': 'creative',
    });
    expect(worldsSource).toContain('pending_restart');
    expect(formSource).toContain('Saved now; applies after the server restarts.');
  });

  it('keeps migration actions on the existing slot and backup routes', () => {
    for (const route of [
      'worldPaths.duplicate',
      'worldPaths.import',
      'backupPaths.restore',
      'worldPaths.activate',
    ]) {
      expect(worldsSource).toContain(route);
    }
    expect(formSource).toContain('Create a new world to change it.');
  });
});
