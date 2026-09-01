import { describe, expect, it } from 'vitest';
import generalSource from '../../src/lib/sections/server-editor/GeneralTab.svelte?raw';
import javaSource from '../../src/lib/sections/server-editor/JavaTab.svelte?raw';
import sheetSource from '../../src/lib/sections/server-editor/ServerEditorSheet.svelte?raw';

describe('server editor memory settings', () => {
  it('automatically saves both RAM values as decimal-capable numbers', () => {
    expect(generalSource).toContain('function parseRamDraft(value: string): number | undefined');
    expect(generalSource).toContain('Number.isFinite(parsed)');
    expect(generalSource).toContain('{ minRamGB, maxRamGB }');
    expect(generalSource).toContain('step={0.1}');
    expect(generalSource).toContain('onchange={handleMinRamChange}');
    expect(generalSource).toContain('onchange={handleMaxRamChange}');
    expect(generalSource).not.toContain('bind:value={minRamDraft}');
    expect(generalSource).not.toContain('bind:value={maxRamDraft}');
  });

  it('removes the manual save action and hardware guidance from the memory block', () => {
    expect(generalSource).not.toContain('memory-footer');
    expect(generalSource).not.toContain('recommendedMaxGB');
    expect(generalSource).not.toContain('physicalRAMGB');
    expect(generalSource).not.toContain("ramSaving ? 'Saving…' : 'Save'");
  });

  it('shows the server folder size as a quiet read-only storage row', () => {
    expect(generalSource).toContain('Server Folder Size');
    expect(generalSource).toContain('bytesLabel(directorySize)');
    expect(generalSource).toContain('serverEditorPaths.directorySize(server.id)');
    expect(generalSource).toContain("directorySizeLoading ? 'Loading…'");
  });

  it('exposes local Java or Bedrock ports without claiming to edit external mappings', () => {
    expect(generalSource).toContain('msc2-type-overline">Ports');
    expect(generalSource).toContain("isJava ? 'Java Port' : 'Bedrock Port'");
    expect(generalSource).toContain('Bedrock / Geyser Port');
    expect(generalSource).toContain("'server-port': String(gamePort)");
    expect(generalSource).toContain('serverEditorPaths.geyser');
    expect(generalSource).toContain('Router forwarding and Playit mappings are separate.');
    expect(generalSource).toContain('max={65535}');
  });

  it('keeps every editor tab scrollable without visible scrollbar chrome', () => {
    expect(sheetSource).toContain('overflow-y: auto');
    expect(sheetSource).toContain('scrollbar-width: none');
    expect(sheetSource).toContain('.tab-panel::-webkit-scrollbar');
    expect(javaSource).toContain('.list::-webkit-scrollbar');
  });
});
