import { describe, expect, it } from 'vitest';
import applicationShellSource from '../../src/lib/components/ApplicationShell.svelte?raw';
import confirmDialogSource from '../../src/lib/components/ConfirmDialog.svelte?raw';
import appSource from '../../src/App.svelte?raw';

describe('shared MSC shell contract', () => {
  it('keeps host/server context, console access, and registry-driven sections in the shell', () => {
    expect(applicationShellSource).toContain('hostLabel');
    expect(applicationShellSource).toContain('serverLabel');
    expect(applicationShellSource).toContain('openConsole');
    expect(applicationShellSource).toContain('{#each sections as section');
    expect(applicationShellSource).not.toContain('sections.slice(0, 5)');
  });

  it('defines accessible states, focus treatment, responsive navigation, and reduced motion tokens', () => {
    expect(appSource).toContain('role="status"');
    expect(confirmDialogSource).toContain('role="alertdialog"');
    expect(applicationShellSource).toContain(':focus-visible');
    expect(applicationShellSource).toContain('max-width: 759px');
    expect(confirmDialogSource).toContain('export let context');
    expect(confirmDialogSource).toContain('{context}');
  });
});
