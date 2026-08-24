import type { Schema } from '../shared/types';

export const demoPlayers: Schema['PlayerDTO'][] = [
  { id: 'player-1', name: 'cameron', displayName: 'Cameron', level: 42 },
];

export function playerSearch(
  players: readonly Schema['PlayerDTO'][],
  query: string,
): Schema['PlayerDTO'][] {
  const needle = query.trim().toLowerCase();
  return players.filter(
    (player) => !needle || `${player.name} ${player.displayName}`.toLowerCase().includes(needle),
  );
}

export const playerPaths = { players: '/v1/players' } as const;
