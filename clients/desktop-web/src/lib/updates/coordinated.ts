export type UpdatePlatform = 'macos' | 'windows' | 'linux';
export type ReleaseArtifactKind = 'desktop' | 'agent' | 'sidecar';

export interface ReleaseArtifact {
  readonly kind: ReleaseArtifactKind;
  readonly fileName: string;
  readonly sha256: string;
}

/** The signed native manifest's public shape; the shell verifies its signature. */
export interface CoordinatedRelease {
  readonly releaseId: string;
  readonly platform: UpdatePlatform;
  readonly apiMajor: number;
  readonly desktopApiMinor: number;
  readonly agentApiMinorFloor: number;
  readonly agentApiMinorCeiling: number;
  readonly artifacts: readonly ReleaseArtifact[];
}

export type UpdateDisposition =
  | { readonly kind: 'ready-to-stage' }
  | { readonly kind: 'package-manager'; readonly message: string }
  | { readonly kind: 'refused'; readonly message: string };

export interface InstallConfirmation {
  readonly releaseId: string;
  readonly explicitlyApproved: boolean;
}

const EXPECTED_API_MAJOR = 1;

/**
 * Validates only policy visible to the shared client. Native Rust repeats this
 * check while verifying the signature and hashes, so the webview cannot make a
 * release installable by bypassing this presentation boundary.
 */
export function assessCoordinatedRelease(release: CoordinatedRelease): UpdateDisposition {
  if (release.platform === 'linux') {
    return {
      kind: 'package-manager',
      message: `MSC ${release.releaseId} is available. Install it through this distribution's package manager.`,
    };
  }
  if (release.apiMajor !== EXPECTED_API_MAJOR) {
    return { kind: 'refused', message: 'This release requires a different API major version.' };
  }
  if (
    release.desktopApiMinor < release.agentApiMinorFloor ||
    release.desktopApiMinor > release.agentApiMinorCeiling
  ) {
    return {
      kind: 'refused',
      message: 'This release falls outside its advertised API compatibility window.',
    };
  }
  const kinds = new Set(release.artifacts.map((artifact) => artifact.kind));
  if (!kinds.has('desktop') || !kinds.has('agent')) {
    return {
      kind: 'refused',
      message: 'A coordinated release must include both desktop and agent.',
    };
  }
  if (release.platform === 'macos' && !kinds.has('sidecar')) {
    return {
      kind: 'refused',
      message: 'The macOS release is missing its compatible Bedrock sidecar.',
    };
  }
  if (release.platform === 'windows' && kinds.has('sidecar')) {
    return {
      kind: 'refused',
      message: 'Windows native Bedrock must not be packaged as a sidecar.',
    };
  }
  return { kind: 'ready-to-stage' };
}

/** A staged release cannot run until the person confirms that exact release ID. */
export function canInstall(
  release: CoordinatedRelease,
  confirmation: InstallConfirmation,
): boolean {
  return (
    assessCoordinatedRelease(release).kind === 'ready-to-stage' &&
    confirmation.explicitlyApproved &&
    confirmation.releaseId === release.releaseId
  );
}
