import { describe, expect, it } from 'vitest';
import confirmSource from '../../src/lib/sections/fleet/wizard/ConfirmStep.svelte?raw';
import sheetSource from '../../src/lib/sections/server-editor/FirstStartSheet.svelte?raw';
import authSource from '../../src/lib/sections/server-editor/BroadcastAuthSheet.svelte?raw';

describe('first-start initiation flow', () => {
  it('leaves initiation on the main server control', () => {
    expect(confirmSource).not.toContain('FirstStartSheet');
    expect(confirmSource).not.toContain('Start first run');
    expect(confirmSource).toContain("draft.worldSourceMode === 'fresh'");
  });

  it('keeps server readiness, EULA, console, and transport completion in the sheet flow', () => {
    expect(sheetSource).toContain("Initiate ${serverName || 'server'}");
    expect(sheetSource).toContain('I accept the Minecraft server EULA.');
    expect(sheetSource).toContain("disabled={serverType === 'java' && !eulaAccepted}");
    expect(sheetSource).toContain('livePaths.tail');
    expect(sheetSource).toContain('First-start console');
    expect(sheetSource).toContain('Pass 1 of 2');
    expect(sheetSource).toContain('Pass 2 of 2');
    expect(sheetSource).toContain('serverEditorPaths.playitStart');
    expect(sheetSource).toContain('serverEditorPaths.broadcastStart');
    expect(sheetSource).toContain(
      'Xbox Broadcast helper is ready. Continue to start it with your server.',
    );
    expect(sheetSource).toContain('600_000');
    expect(sheetSource).toContain('Future starts are manual');
    expect(sheetSource).toContain('context="initiation"');
    expect(sheetSource).toContain('BroadcastAuthSheet');
    expect(sheetSource).toContain('showBroadcastAuth');
  });

  it('presents the Microsoft device-code prompt from the managed helper', () => {
    expect(authSource).toContain('broadcastAuthPromptDismiss');
    expect(authSource).toContain('Open Microsoft sign-in');
    expect(authSource).toContain('Device code');
    expect(authSource).not.toContain('password');
  });

  it('keeps the first-start surface flat and state-labelled', () => {
    expect(sheetSource).toContain('<StatusDot');
    expect(sheetSource).not.toContain('gradient');
    expect(sheetSource).not.toContain('backdrop-filter');
  });
});
