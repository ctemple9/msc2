import type { Schema, ScreenApi } from '../shared/types';

// Real, frozen routes (docs/msc2/api-contract/openapi.json) -- backed by
// crates/msc-agent/src/routes/files.rs (P12.9), a straight port of MSC 1's
// `wireSkinAndFileProviders` (AppViewModel+APIWiringContent.swift:170-275).
export const filePaths = {
  browse: (path: string): string =>
    path ? `/v1/files?path=${encodeURIComponent(path)}` : '/v1/files',
  read: (path: string): string => `/v1/files/read?path=${encodeURIComponent(path)}`,
} as const;

export async function browseDirectory(
  api: ScreenApi | undefined,
  path: string,
): Promise<Schema['ServerFilesResponseDTO'] | undefined> {
  if (!api) return undefined;
  return api.get<Schema['ServerFilesResponseDTO']>(filePaths.browse(path));
}

export async function readFile(
  api: ScreenApi | undefined,
  path: string,
): Promise<Schema['ServerFileReadResponseDTO'] | undefined> {
  if (!api) return undefined;
  return api.get<Schema['ServerFileReadResponseDTO']>(filePaths.read(path));
}

export interface Breadcrumb {
  label: string;
  path: string;
}

/** ServerFilesTabView's own `breadcrumbs`: "Server Root" first, then one
 *  crumb per path segment. `path` is the relative path the agent already
 *  echoes back in `ServerFilesResponseDTO.path` -- no client-side
 *  navigation-stack bookkeeping needed (unlike the oracle's `navigationStack`,
 *  which existed only because `FileManager` calls need a real `URL`; this
 *  client just re-requests the agent with a new relative path string). */
export function breadcrumbsFor(path: string): Breadcrumb[] {
  const crumbs: Breadcrumb[] = [{ label: 'Server Root', path: '' }];
  if (!path) return crumbs;
  const segments = path.split('/').filter(Boolean);
  let built = '';
  for (const segment of segments) {
    built = built ? `${built}/${segment}` : segment;
    crumbs.push({ label: segment, path: built });
  }
  return crumbs;
}

/** MSC 1's `.formatted(.relative(presentation: .named))` -- no existing
 *  client-side helper for this (dateLabel in shared/types.ts is absolute). */
export function relativeTime(iso: string | undefined): string | undefined {
  if (!iso) return undefined;
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return undefined;
  const seconds = Math.round((Date.now() - then) / 1000);
  if (seconds < 5) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days}d ago`;
  return new Date(iso).toLocaleDateString();
}

/** `note` is only ever set on an otherwise-200 response (`invalid_path`,
 *  `directory_not_found`) -- see files.rs's own doc comment for why those
 *  aren't error statuses. Turned into the message this screen shows in
 *  place of a listing. */
export function browseNoticeFor(note: string | undefined): string | undefined {
  switch (note) {
    case 'invalid_path':
      return 'That path is outside the server directory.';
    case 'directory_not_found':
      return "That folder doesn't exist.";
    default:
      return undefined;
  }
}

/** `message` on a failed `ServerFileReadResponseDTO`/`ErrorDTO` body, turned
 *  into the text this screen shows in the preview sheet in place of content. */
export function readErrorMessage(code: string | undefined): string {
  switch (code) {
    case 'not_previewable':
      return "This file type can't be previewed.";
    case 'directory_not_file':
      return "That's a folder, not a file.";
    case 'file_not_found':
      return 'That file no longer exists.';
    case 'read_failed':
      return 'The file could not be read.';
    default:
      return 'Could not open this file.';
  }
}
