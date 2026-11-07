import type { Schema } from '../shared/types';
import type { ContentEntry, ContentSnapshot } from './content-model';

export interface PlayerContentItem {
  path: string;
  player?: ContentEntry;
  server?: ContentEntry;
  upstream?: ContentEntry;
  requirement?: ContentSnapshot['requirements'][number];
}
export const primaryFolders = ['mods', 'resourcepacks', 'shaderpacks', 'datapacks'];
export const categoryNames: Record<string, string> = {
  mods: 'Mods',
  resourcepacks: 'Resource packs',
  shaderpacks: 'Shaders',
  datapacks: 'Datapacks',
};
export function contentItems(snapshot: ContentSnapshot | undefined): PlayerContentItem[] {
  if (!snapshot) return [];
  const collection = snapshot.collection;
  const requirements = new Map(snapshot.requirements.map((item) => [item.path, item]));
  return [
    ...new Set([...Object.keys(collection.currentFiles), ...Object.keys(collection.playerFiles)]),
  ]
    .sort((a, b) => a.localeCompare(b))
    .map((path) => ({
      path,
      server: collection.currentFiles[path],
      player: collection.playerFiles[path],
      upstream: collection.upstreamPlayerFiles[path],
      requirement: requirements.get(path),
    }));
}
export function primary(item: PlayerContentItem): boolean {
  return primaryFolders.includes(item.path.split('/')[0]);
}
export function needsAttention(item: PlayerContentItem): boolean {
  return (
    (!!item.requirement &&
      !['matched', 'serverOnly', 'serverDisabled', 'optionalMissing'].includes(
        item.requirement.state,
      )) ||
    !!item.player?.unresolvedReasons.length ||
    !!item.server?.unresolvedReasons.length
  );
}
export function itemState(item: PlayerContentItem): string {
  if (!item.player)
    return item.server?.audience === 'server' ? 'Server only' : 'Not in player content';
  return `${item.player.optional ? 'Optional' : 'Required'} · ${item.player.enabled === false ? 'Inactive' : 'Selected'}`;
}
export function supportingFolder(item: PlayerContentItem): string {
  const parts = item.path.split('/');
  return parts.slice(0, parts.length > 2 ? 2 : 1).join('/');
}
export function itemMatches(item: PlayerContentItem, query: string, filter: string): boolean {
  if (!item.path.toLowerCase().includes(query.trim().toLowerCase())) return false;
  if (filter === 'attention') return needsAttention(item);
  if (filter === 'required') return !!item.player && !item.player.optional;
  if (filter === 'optional') return !!item.player?.optional;
  if (filter === 'inactive') return item.player?.enabled === false;
  return true;
}
export const changeNames: Record<string, string> = {
  copyServerToPlayers: 'Include server file for players',
  setServerAudience: 'Set audience',
  setPlayerOptions: 'Update player options',
  removePlayerFile: 'Remove from player content',
  restorePlayerUpstream: 'Restore original pack file',
  addPlayerFile: 'Add player file',
};

export function actionAllowed(item: PlayerContentItem, action: string): boolean {
  if (action === 'serverOnly') return !!item.server;
  if (action === 'restore') return !!item.upstream;
  if (['inactive', 'active', 'remove'].includes(action)) return !!item.player;
  return ['required', 'optional'].includes(action) && !!(item.player || item.server);
}

export function playerContentChanges(
  items: PlayerContentItem[],
  action: string,
): Partial<Schema['ContentChangeDTO']>[] {
  if (!items.length || !items.every((item) => actionAllowed(item, action)))
    throw new Error('Choose an action that applies to every selected file.');
  const changes: Partial<Schema['ContentChangeDTO']>[] = [];
  for (const item of items) {
    const player = item.player;
    const server = item.server;
    if (action === 'required' || action === 'optional') {
      const optional = action === 'optional';
      if (server)
        changes.push({
          action: 'setServerAudience',
          path: item.path,
          sha512: server.sha512,
          audience: 'both',
          optional,
        });
      if (player)
        changes.push({
          action: 'setPlayerOptions',
          path: item.path,
          sha512: player.sha512,
          optional,
        });
      else if (server) {
        changes.push({
          action: 'copyServerToPlayers',
          path: item.path,
          sha512: server.sha512,
          optional,
        });
        if (optional && /^(shaderpacks|resourcepacks)\//.test(item.path))
          changes.push({
            action: 'setPlayerOptions',
            path: item.path,
            sha512: server.sha512,
            enabled: false,
          });
      }
    } else if (action === 'serverOnly' && server)
      changes.push({
        action: 'setServerAudience',
        path: item.path,
        sha512: server.sha512,
        audience: 'server',
      });
    else if (action === 'restore' && item.upstream)
      changes.push({
        action: 'restorePlayerUpstream',
        path: item.path,
        sha512: item.upstream.sha512,
      });
    else if (player)
      changes.push({
        action: action === 'remove' ? 'removePlayerFile' : 'setPlayerOptions',
        path: item.path,
        sha512: player.sha512,
        ...(action === 'remove' ? {} : { enabled: action === 'active' }),
      });
  }
  if (changes.length > 256) {
    throw new Error(
      'This selection is too large for one review. Select fewer files using the page checkboxes, then review the next group after saving.',
    );
  }
  return changes;
}
