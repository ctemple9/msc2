export interface ServerSelectionCandidate {
  readonly id: string;
}

export function selectAvailableServerId(
  servers: readonly ServerSelectionCandidate[],
  activeServerId?: string,
  currentServerId?: string,
): string {
  if (activeServerId && servers.some((server) => server.id === activeServerId)) {
    return activeServerId;
  }
  if (currentServerId && servers.some((server) => server.id === currentServerId)) {
    return currentServerId;
  }
  return servers[0]?.id ?? '';
}
