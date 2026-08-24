export interface TransportCredentialAdapter {
  headersFor(hostId: string): Promise<Readonly<Record<string, string>>>;
  requestCredentials?: RequestCredentials;
}

/** Browser sessions are deliberately represented by fetch's cookie jar. */
export function cookieCredentialAdapter(): TransportCredentialAdapter {
  return {
    headersFor: async () => ({}),
    requestCredentials: 'include',
  };
}

/** The shell supplies a token on demand; the transport never persists it. */
export function bearerCredentialAdapter(
  tokenFor: (hostId: string) => string | Promise<string>,
): TransportCredentialAdapter {
  return {
    headersFor: async (hostId) => ({ Authorization: `Bearer ${await tokenFor(hostId)}` }),
  };
}
