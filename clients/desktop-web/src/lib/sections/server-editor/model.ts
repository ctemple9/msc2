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
export const serverEditorPaths = {
  rename: '/v1/servers/rename',
  delete: '/v1/servers/delete',
  eula: '/v1/servers/eula',
  active: '/v1/active-server',
  status: '/v1/status',
  ram: '/v1/config/ram',
  broadcastStatus: '/v1/broadcast/status',
  broadcastAutostart: '/v1/broadcast/autostart',
  broadcastCredentials: '/v1/broadcast/credentials',
  broadcastJarStatus: '/v1/broadcast/jar-status',
  broadcastDownloadJar: '/v1/broadcast/download-jar',
  broadcastStart: '/v1/broadcast/start',
  broadcastStop: '/v1/broadcast/stop',
  playit: '/v1/playit',
  playitStart: '/v1/playit/start',
  playitStop: '/v1/playit/stop',
  duckdns: '/v1/duckdns',
  resourcePacks: '/v1/resourcepacks',
  resourcePacksToggle: '/v1/resourcepacks/toggle',
} as const;

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
