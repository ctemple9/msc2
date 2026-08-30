import { describe, expect, it } from 'vitest';
import sheetSource from '../../src/lib/sections/server-editor/PlayitSetupSheet.svelte?raw';
import broadcastSource from '../../src/lib/sections/server-editor/BroadcastTab.svelte?raw';
import {
  PLAYIT_SETUP_STEPS,
  playitSetupError,
  playitSetupProgressForStatus,
  playitSetupStepsForMode,
} from '../../src/lib/sections/server-editor/model';

describe('native Playit setup sheet', () => {
  it('keeps the account flow agent-owned and credential-free after submission', () => {
    expect(sheetSource).toContain('serverEditorPaths.playitSetup');
    expect(sheetSource).toContain('serverEditorPaths.playitReset');
    expect(sheetSource).toContain('email');
    expect(sheetSource).toContain('password');
    expect(sheetSource).toContain('https://playit.gg/login');
    expect(sheetSource).toContain('rel="noopener noreferrer"');
    expect(sheetSource).toContain('two-factor');
    expect(sheetSource).toContain("email = ''");
    expect(sheetSource).toContain("password = ''");
    expect(sheetSource).not.toContain('api.playit.gg');
    expect(sheetSource).not.toContain('secretKey');
  });

  it('shows the frozen operation stages and narrows later voice setup', () => {
    expect(PLAYIT_SETUP_STEPS.map((step) => step.key)).toEqual([
      'signing_in',
      'claiming_or_reusing_agent',
      'waiting_for_agent',
      'creating_or_reusing_java_tunnel',
      'creating_or_reusing_bedrock_tunnel',
      'creating_or_reusing_voice_tunnel',
      'receiving_public_addresses',
    ]);
    expect(playitSetupStepsForMode(true).map((step) => step.key)).toEqual([
      'signing_in',
      'claiming_or_reusing_agent',
      'waiting_for_agent',
      'creating_or_reusing_voice_tunnel',
      'receiving_public_addresses',
    ]);
    expect(sheetSource).toContain("context === 'initiation'");
    expect(sheetSource).toContain('onComplete');
  });

  it('maps agent progress wording and provider failures to useful UI copy', () => {
    expect(playitSetupProgressForStatus('Reusing existing agent')).toBe(
      'claiming_or_reusing_agent',
    );
    expect(playitSetupProgressForStatus('Creating or reusing MSC Voice tunnel')).toBe(
      'creating_or_reusing_voice_tunnel',
    );
    expect(playitSetupProgressForStatus('Public addresses received')).toBe(
      'receiving_public_addresses',
    );
    expect(playitSetupError(new Error('two_factor_required'))).toContain('cannot complete 2FA');
    expect(playitSetupError(new Error('incorrect_credentials'))).toContain('email or password');
  });

  it('keeps the existing Playit start/stop gate and adds one deliberate entry point', () => {
    expect(broadcastSource).toContain('!playit?.playitEnabled');
    expect(broadcastSource).toContain('Add voice tunnel…');
    expect(broadcastSource).toContain('<PlayitSetupSheet');
    expect(broadcastSource).toContain('if (!isActive || playitBusy || !playit) return;');
    expect(broadcastSource).toContain('if (!isActive) return;');
    expect(broadcastSource).toContain('loadVersion !== playitLoadVersion');
    expect(broadcastSource).not.toContain('gradient');
    expect(broadcastSource).not.toContain('backdrop-filter');
  });
});
