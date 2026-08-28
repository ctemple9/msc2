import { describe, expect, it } from 'vitest';
import {
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
