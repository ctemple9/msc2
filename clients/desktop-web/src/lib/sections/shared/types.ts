import type { components } from '../../api/generated';
import type { FileChunkSource, FileUploadProgress } from '../../platform/types';

export type Schema = components['schemas'];

/** The four user-facing inventory states shown by Components. */
export type ComponentState = 'installed' | 'missing' | 'unresolved' | 'disabled';

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
    /** Modpack recovery uploads are bound to one operation/file and retain the
     * browser's original name for exact filename validation. */
    options?: { operationId?: string; fileId?: string; fileName?: string },
  ): Promise<Schema['StagedUploadCompleteResultDTO']>;
  uploadFile?(
    purpose: Schema['StagedUploadBeginRequestDTO']['purpose'],
    source: FileChunkSource,
    options?: {
      operationId?: string;
      fileId?: string;
      fileName?: string;
      onProgress?: (progress: FileUploadProgress) => void;
    },
  ): Promise<Schema['StagedUploadCompleteResultDTO']>;
  download?(id: string, maxBytes?: number): Promise<Uint8Array>;
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
  if (error instanceof Error) return error.message;
  if (typeof error === 'string' && error.trim()) return error;
  if (
    error !== null &&
    typeof error === 'object' &&
    'message' in error &&
    typeof error.message === 'string' &&
    error.message.trim()
  ) {
    return error.message;
  }
  return 'The request failed for an unknown reason.';
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
