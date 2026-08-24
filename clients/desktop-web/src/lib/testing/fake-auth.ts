import type { components } from '../api/generated';

export type PermissionCategory =
  'none' | NonNullable<components['schemas']['CapabilitiesDTO']['permissions']>[number];

export type AuthScheme = 'bearer' | 'cookie';

export interface CredentialDefinition {
  id: string;
  permissions: PermissionCategory[];
  scheme: AuthScheme;
  secret: string;
}

export interface AuthDecision {
  credentialId: string;
  permissions: PermissionCategory[];
}

/**
 * In-memory auth is deliberately explicit so contract tests cannot accidentally
 * depend on a browser session, a keychain, or a developer token in the shell.
 */
export class FakeAuth {
  private readonly credentials = new Map<string, CredentialDefinition>();

  addCredential(definition: CredentialDefinition): void {
    this.credentials.set(definition.id, {
      ...definition,
      permissions: [...definition.permissions],
    });
  }

  headersFor(id: string): Record<string, string> {
    const credential = this.credentials.get(id);
    if (!credential) {
      throw new Error(`unknown fixture credential: ${id}`);
    }

    return credential.scheme === 'bearer'
      ? { Authorization: `Bearer ${credential.secret}` }
      : { Cookie: `msc_session=${credential.secret}` };
  }

  authorize(
    headers: Record<string, string>,
    requiredPermission: PermissionCategory,
  ): AuthDecision | null {
    const bearer = headers.Authorization?.match(/^Bearer (.+)$/)?.[1];
    const cookie = headers.Cookie?.match(/(?:^|; )msc_session=([^;]+)/)?.[1];
    const credential = [...this.credentials.values()].find(
      (candidate) => candidate.secret === (bearer ?? cookie),
    );

    if (
      !credential ||
      (requiredPermission !== 'none' && !credential.permissions.includes(requiredPermission))
    ) {
      return null;
    }

    return {
      credentialId: credential.id,
      permissions: [...credential.permissions],
    };
  }
}
