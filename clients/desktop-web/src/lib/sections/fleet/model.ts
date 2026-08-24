import type { Schema } from '../shared/types';

export type FleetServer = Schema['ServerDTO'];
export type FleetStatus = Schema['RemoteAPIStatus'];

export const demoServers: FleetServer[] = [
  {
    id: 'survival',
    name: 'Survival',
    directory: 'servers/survival',
    serverType: 'paper',
    javaFlavor: 'Java 21',
    gamePort: 25565,
  },
  {
    id: 'creative',
    name: 'Creative',
    directory: 'servers/creative',
    serverType: 'vanilla',
    javaFlavor: 'Java 21',
    gamePort: 25566,
  },
];

export const demoStatus: FleetStatus = {
  activeServerId: 'survival',
  running: false,
  serverType: 'paper',
};

export const fleetMutationPaths = {
  create: '/v1/servers/create',
  import: '/v1/servers/import',
  rename: '/v1/servers/rename',
  delete: '/v1/servers/delete',
  eula: '/v1/servers/eula',
  active: '/v1/active-server',
  start: '/v1/start',
  stop: '/v1/stop',
  runtimes: '/v1/java-runtimes',
  installRuntime: '/v1/java-runtimes/install',
  versions: '/v1/versions',
  templates: '/v1/templates',
} as const;

export function selectedServer(
  servers: readonly FleetServer[],
  activeId: string | undefined,
): FleetServer | undefined {
  return servers.find((server) => server.id === activeId) ?? servers[0];
}
