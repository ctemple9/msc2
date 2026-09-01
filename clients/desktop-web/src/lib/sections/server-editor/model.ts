import type { Schema, ScreenApi } from '../shared/types';

// Real, frozen routes this sheet is built against. `rename`/`delete`/`eula`
// take an explicit serverId (docs/msc2/api-contract/openapi.json), so they
// work on any server card regardless of which one is currently active.
// Everything else here -- RAM, broadcast, Playit, DuckDNS, resource packs --
// has no serverId parameter at all: crates/msc-agent/src/routes/networking.rs
// and routes/servers.rs's `/v1/config/ram` both resolve a single agent-wide
// `state.active_server()`, so a mutation always lands on whichever server is
// currently active, never the target this sheet was opened for if that
// differs. GeneralTab/BroadcastTab gate those specific blocks on `isActive`
// and offer the same "Set as Active" action ManageSheet's row menu already
// exposes, rather than silently mutating the wrong server or switching the
// active server behind the caller's back.
//
// The three java* routes below are a third category: no serverId, but also
// no active-server resolution -- `AppConfig.javaPath` is host-wide config,
// the same value regardless of which server (if any) is active. JavaTab
// reads/writes it unconditionally, with no `isActive` gate.
export const serverEditorPaths = {
  rename: '/v1/servers/rename',
  directory: '/v1/servers/directory',
  directorySize: (serverId: string): string =>
    `/v1/servers/size?serverId=${encodeURIComponent(serverId)}`,
  delete: '/v1/servers/delete',
  eula: '/v1/servers/eula',
  settings: '/v1/settings',
  geyser: '/v1/config/geyser',
  active: '/v1/active-server',
  status: '/v1/status',
  ram: '/v1/config/ram',
  broadcastStatus: '/v1/broadcast/status',
  broadcastAutostart: '/v1/broadcast/autostart',
  broadcastCredentials: '/v1/broadcast/credentials',
  broadcastAuthPromptDismiss: '/v1/broadcast/auth-prompt/dismiss',
  broadcastJarStatus: '/v1/broadcast/jar-status',
  broadcastDownloadJar: '/v1/broadcast/download-jar',
  broadcastStart: '/v1/broadcast/start',
  broadcastStop: '/v1/broadcast/stop',
  playit: '/v1/playit',
  playitSetup: '/v1/playit/setup',
  playitReset: '/v1/playit/reset',
  playitStart: '/v1/playit/start',
  playitStop: '/v1/playit/stop',
  duckdns: '/v1/duckdns',
  resourcePacks: '/v1/resourcepacks',
  resourcePacksToggle: '/v1/resourcepacks/toggle',
  javaConfig: '/v1/config/java-runtime',
  javaRuntimes: '/v1/java-runtimes',
  javaRuntimeInstall: '/v1/java-runtimes/install',
} as const;

export type PlayitSetupContext = 'settings' | 'initiation';

export type PlayitSetupProgressKey =
  | 'signing_in'
  | 'claiming_or_reusing_agent'
  | 'waiting_for_agent'
  | 'creating_or_reusing_java_tunnel'
  | 'creating_or_reusing_bedrock_tunnel'
  | 'creating_or_reusing_voice_tunnel'
  | 'receiving_public_addresses';

export type PlayitSetupStep = {
  key: PlayitSetupProgressKey;
  label: string;
};

/** The progress vocabulary is frozen in P12.20a. Keeping its display copy in
 * one place means the settings sheet and first-start sheet will describe the
 * same agent-owned operation when the latter is wired in by P12.20f. */
export const PLAYIT_SETUP_STEPS: readonly PlayitSetupStep[] = [
  { key: 'signing_in', label: 'Signing in to playit.gg' },
  { key: 'claiming_or_reusing_agent', label: 'Claiming or reusing the Playit agent' },
  { key: 'waiting_for_agent', label: 'Waiting for the agent to come online' },
  { key: 'creating_or_reusing_java_tunnel', label: 'Creating or reusing the Java tunnel' },
  { key: 'creating_or_reusing_bedrock_tunnel', label: 'Creating or reusing the Bedrock tunnel' },
  { key: 'creating_or_reusing_voice_tunnel', label: 'Creating or reusing the voice tunnel' },
  { key: 'receiving_public_addresses', label: 'Receiving public addresses' },
];

export function playitSetupStepsForMode(voiceOnly: boolean): readonly PlayitSetupStep[] {
  if (!voiceOnly) return PLAYIT_SETUP_STEPS;
  return PLAYIT_SETUP_STEPS.filter(
    ({ key }) =>
      key === 'signing_in' ||
      key === 'claiming_or_reusing_agent' ||
      key === 'waiting_for_agent' ||
      key === 'creating_or_reusing_voice_tunnel' ||
      key === 'receiving_public_addresses',
  );
}

/** The agent sends human-readable status lines, while the contract also
 * freezes the state vocabulary. This tolerant matcher lets older/newer agent
 * wording update the right visible step without making the client a second
 * Playit implementation. */
export function playitSetupProgressForStatus(
  statusLine: string | null | undefined,
  fallback: PlayitSetupProgressKey = 'signing_in',
): PlayitSetupProgressKey {
  const text = (statusLine ?? '').toLowerCase().replace(/[-_]/g, ' ');
  if (text.includes('sign') || text.includes('log in')) return 'signing_in';
  if (text.includes('claim') || (text.includes('reus') && text.includes('agent')))
    return 'claiming_or_reusing_agent';
  if (text.includes('wait') && text.includes('agent')) return 'waiting_for_agent';
  if (text.includes('java') && text.includes('tunnel')) return 'creating_or_reusing_java_tunnel';
  if (text.includes('bedrock') && text.includes('tunnel'))
    return 'creating_or_reusing_bedrock_tunnel';
  if ((text.includes('voice') || text.includes('24454')) && text.includes('tunnel'))
    return 'creating_or_reusing_voice_tunnel';
  if (text.includes('address')) return 'receiving_public_addresses';
  return fallback;
}

/** Stable provider codes are the useful part of an API error. Convert them to
 * direct next steps while preserving an already-human agent message. */
export function playitSetupError(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error);
  const text = raw.toLowerCase().replace(/[-_]/g, ' ');
  if (text.includes('two factor') || text.includes('2fa')) {
    return "This Playit account requires two-factor authentication. MSC's native setup cannot complete 2FA yet; no changes were made. Try an account without 2FA or try again later.";
  }
  if (text.includes('incorrect credential') || text.includes('invalid credential')) {
    return 'Playit did not accept that email or password. Check both fields and try again.';
  }
  if (text.includes('account banned') || text.includes('banned account')) {
    return 'This Playit account is banned. Use a different account or contact Playit support.';
  }
  if (text.includes('rate limit') || text.includes('too many')) {
    return 'Playit is temporarily rate-limiting sign-in. Wait a moment, then try again.';
  }
  if (text.includes('agent not found')) {
    return 'The saved Playit agent was not found. Reset local Playit setup, then sign in again.';
  }
  if (text.includes('setup unavailable')) {
    return 'Native Playit setup is not available on this agent yet. Update the agent and try again.';
  }
  if (text.includes('setup in progress')) {
    return 'Another Playit setup is already running on this host. Wait for it to finish or cancel it.';
  }
  return raw || 'Playit setup did not complete. Check the agent and try again.';
}

export type PlayitSetupAccepted = {
  result: string;
  operationId: string;
  message?: string | null;
};

export type PlayitResetResult = {
  result: string;
  message?: string | null;
  operationId?: string | null;
};

export const operationPath = (id: string): string => `/v1/operations/${id}`;

const OPERATION_POLL_MS = 900;

export async function pollOperation(
  api: ScreenApi | undefined,
  operationId: string,
  onTick?: (operation: Schema['OperationDTO']) => void,
  delayMs = OPERATION_POLL_MS,
): Promise<Schema['OperationDTO'] | undefined> {
  if (!api) return undefined;
  for (;;) {
    const operation = await api.get<Schema['OperationDTO']>(operationPath(operationId));
    onTick?.(operation);
    if (
      operation.state === 'succeeded' ||
      operation.state === 'failed' ||
      operation.state === 'cancelled'
    ) {
      return operation;
    }
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }
}
