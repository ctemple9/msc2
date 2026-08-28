/** The two entry points `AddServerWizardView.swift`'s step 1 offers. */
export type WizardPath = 'importExisting' | 'fresh';

/**
 * Step-chip labels for the wizard's step counter, mirroring
 * `AddServerWizardView.swift`'s `stepLabel(_:)` -- Fresh and Import each walk
 * a different five-step sequence, sharing only step 1 (Choose path) and the
 * final step (Confirm). The oracle also inserts a sixth Add-ons step for
 * Fresh/Java flavors with a plugin or mod ecosystem (`hasAddOnsStep`); that
 * depends on the Configure step's flavor picker, which P12.18b builds, so
 * it isn't wired into this list yet -- P12.18f adds it there.
 */
export function wizardStepLabels(path: WizardPath): readonly string[] {
  return path === 'fresh'
    ? ['Choose path', 'Configure', 'Network', 'World', 'Confirm']
    : ['Choose path', 'Upload', 'Review', 'Network', 'Confirm'];
}
