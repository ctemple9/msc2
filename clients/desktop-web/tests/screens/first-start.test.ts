import { describe, expect, it } from 'vitest';
import confirmSource from '../../src/lib/sections/fleet/wizard/ConfirmStep.svelte?raw';
import sheetSource from '../../src/lib/sections/server-editor/FirstStartSheet.svelte?raw';

describe('first-start initiation flow', () => {
  it('connects the wizard to the two-pass first-start sheet', () => {
    expect(confirmSource).toContain('FirstStartSheet');
    expect(confirmSource).toContain('Start first run');
    expect(confirmSource).toContain("draft.worldSourceMode === 'fresh'");
  });

  it('keeps server readiness and transport completion in the sheet flow', () => {
    expect(sheetSource).toContain('Pass 1 of 2');
    expect(sheetSource).toContain('Pass 2 of 2');
    expect(sheetSource).toContain('serverEditorPaths.playitStart');
    expect(sheetSource).toContain('serverEditorPaths.broadcastStart');
    expect(sheetSource).toContain('600_000');
    expect(sheetSource).toContain('Future starts are manual');
    expect(sheetSource).toContain('context="initiation"');
  });

  it('keeps the first-start surface flat and state-labelled', () => {
    expect(sheetSource).toContain('<StatusDot');
    expect(sheetSource).not.toContain('gradient');
    expect(sheetSource).not.toContain('backdrop-filter');
  });
});
