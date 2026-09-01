import type { components } from '../../api/generated';

export type Schema = components['schemas'];

/** The screen layer talks through this small adapter so browser and Tauri keep one workflow. */
export interface ScreenApi {
  get<T>(path: string): Promise<T>;
  post<T>(path: string, body?: unknown): Promise<T>;
  /** Fetches binary resources through the authenticated host transport. */
  getBytes?(path: string): Promise<Uint8Array>;
  /** Builds a host-aware URL for resources rendered directly by the browser. */
  resourceUrl?(path: string): string;
  upload?(
    purpose: Schema['StagedUploadBeginRequestDTO']['purpose'],
    bytes: Uint8Array,
    /** curseforge-manual-file only: which pending operation/file this upload resumes. */
    options?: { operationId?: string; fileId?: string },
  ): Promise<Schema['StagedUploadCompleteResultDTO']>;
  download?(id: string): Promise<Uint8Array>;
}

export type ScreenProps = {
  api?: ScreenApi;
  hostId?: string;
  serverId?: string;
  permissions?: readonly string[];
};

export function can(permissions: readonly string[] | undefined, permission: string): boolean {
  return !permissions || permissions.includes(permission) || permissions.includes('admin');
}

export function bytesLabel(bytes: number | undefined): string {
  if (bytes === undefined) return '—';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

export function dateLabel(value: string | undefined): string {
  if (!value) return 'Unknown time';
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : 'The agent did not complete that request.';
}

/** Creates a browser image URL from bytes returned by the authenticated API. */
export function imageObjectUrl(bytes: Uint8Array, mimeType = 'image/jpeg'): string {
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  return URL.createObjectURL(new Blob([buffer], { type: mimeType }));
}

export async function call<T>(api: ScreenApi | undefined, fallback: T, path: string): Promise<T> {
  if (!api) return fallback;
  try {
    return await api.get<T>(path);
  } catch {
    return fallback;
  }
}

export async function mutate<T>(
  api: ScreenApi | undefined,
  path: string,
  body?: unknown,
): Promise<T> {
  if (!api) throw new Error('Connect to an agent before changing server state.');
  return api.post<T>(path, body);
}

export function operationLabel(operation: Schema['OperationDTO']): string {
  return `${operation.type} · ${operation.state}`;
}
