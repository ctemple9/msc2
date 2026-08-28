import { describe, expect, it } from 'vitest';
import { wizardStepLabels } from '../../src/lib/sections/fleet/wizard/model';

describe('add server wizard step labels', () => {
  it('walks the Fresh path sequence', () => {
    expect(wizardStepLabels('fresh')).toEqual([
      'Choose path',
      'Configure',
      'Network',
      'World',
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
