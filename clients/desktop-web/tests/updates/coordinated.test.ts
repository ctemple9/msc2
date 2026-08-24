import { describe, expect, it } from 'vitest';
import {
  assessCoordinatedRelease,
  canInstall,
  type CoordinatedRelease,
} from '../../src/lib/updates/coordinated';

const macRelease: CoordinatedRelease = {
  releaseId: '2026.8.24',
  platform: 'macos',
  apiMajor: 1,
  desktopApiMinor: 4,
  agentApiMinorFloor: 2,
  agentApiMinorCeiling: 5,
  artifacts: [
    { kind: 'desktop', fileName: 'MSC-2.pkg', sha256: 'a'.repeat(64) },
    { kind: 'agent', fileName: 'msc-agent.tar.zst', sha256: 'b'.repeat(64) },
    { kind: 'sidecar', fileName: 'msc-sidecar.zip', sha256: 'c'.repeat(64) },
  ],
};

describe('coordinated release policy', () => {
  it('stages only an API-compatible macOS release set', () => {
    expect(assessCoordinatedRelease(macRelease)).toEqual({ kind: 'ready-to-stage' });
  });

  it('requires explicit approval for the exact staged release', () => {
    expect(
      canInstall(macRelease, { releaseId: macRelease.releaseId, explicitlyApproved: false }),
    ).toBe(false);
    expect(
      canInstall(macRelease, { releaseId: 'different-release', explicitlyApproved: true }),
    ).toBe(false);
    expect(
      canInstall(macRelease, { releaseId: macRelease.releaseId, explicitlyApproved: true }),
    ).toBe(true);
  });

  it('refuses incomplete or incompatible sets instead of silently degrading them', () => {
    expect(
      assessCoordinatedRelease({ ...macRelease, artifacts: macRelease.artifacts.slice(0, 2) }),
    ).toMatchObject({
      kind: 'refused',
      message: expect.stringContaining('sidecar'),
    });
    expect(assessCoordinatedRelease({ ...macRelease, desktopApiMinor: 6 })).toMatchObject({
      kind: 'refused',
      message: expect.stringContaining('compatibility'),
    });
  });

  it('gives Linux an actionable package-manager notice instead of a self-updater', () => {
    expect(
      assessCoordinatedRelease({ ...macRelease, platform: 'linux', artifacts: [] }),
    ).toMatchObject({
      kind: 'package-manager',
      message: expect.stringContaining('package manager'),
    });
  });
});
