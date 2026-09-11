export type ConnectionErrorCategory =
  | 'network'
  | 'ssh'
  | 'msc-agent'
  | 'authentication'
  | 'minecraft';

const labels: Record<ConnectionErrorCategory, string> = {
  network: 'Network',
  ssh: 'SSH',
  'msc-agent': 'MSC agent',
  authentication: 'Authentication',
  minecraft: 'Minecraft',
};

/** Keeps connection failures identifiable without exposing native diagnostics raw. */
export function formatConnectionFailure(
  error: unknown,
  fallback: ConnectionErrorCategory = 'network',
): string {
  const raw = error instanceof Error ? error.message : String(error);
  const candidate =
    error && typeof error === 'object'
      ? (error as { status?: unknown; error?: { code?: unknown; message?: unknown } })
      : {};
  const apiStatus = typeof candidate.status === 'number' ? candidate.status : undefined;
  const apiCode = typeof candidate.error?.code === 'string' ? candidate.error.code : '';
  const apiMessage = typeof candidate.error?.message === 'string' ? candidate.error.message : raw;
  const message = apiStatus
    ? `${labels[categoryForAgentResponse(apiStatus, apiCode)]}: ${apiMessage}`
    : raw.replace(/^Error:\s*/, '').trim();
  if (hasKnownCategory(message)) return message;
  return `${labels[fallback]}: ${message || 'The connection failed.'}`;
}

/** Maps an agent refusal into the same vocabulary as native connection errors. */
export function categoryForAgentResponse(status: number, code = ''): ConnectionErrorCategory {
  if (status === 401 || status === 403 || code.includes('auth')) return 'authentication';
  if (status === 426 || code.includes('version')) return 'msc-agent';
  if (code.startsWith('server_') || code === 'capability_unavailable') return 'minecraft';
  if (status >= 500) return 'msc-agent';
  return 'network';
}

function hasKnownCategory(message: string): boolean {
  return Object.values(labels).some((label) => message.startsWith(`${label}:`));
}
