import { reportTransferFailure } from './transfer-notices';
import { getPlatform } from '../../platform';
import type {
  FileChunkSource,
  FilePickerRequest,
  NativeFileSource,
  NativeUploadBinding,
  NativeUploadProgress,
  FileUploadProgress,
} from '../../platform/types';
import type { Schema, ScreenApi } from './types';

export type UploadSource = (NativeFileSource | FileChunkSource) & {
  readonly uploadApi?: ScreenApi;
  readonly olderAgent?: boolean;
  readonly uploadBindings?: UploadBindings;
};
export type UploadBindings = Pick<
  NativeUploadBinding,
  'targetServerId' | 'operationId' | 'fileId'
> & { droppedSourceId?: string };
export const isNativeSource = (source: UploadSource): source is NativeFileSource & UploadSource =>
  'kind' in source && source.kind === 'native';

/** Cleanup must remain usable after navigation; reads/mutations must not. */
export function guardUploadApi(api: ScreenApi, current: () => boolean): ScreenApi {
  const check = () => {
    if (!current()) throw new Error('The selected host or server changed. Choose the file again.');
  };
  return {
    ...api,
    isCurrent: current,
    snapshot: () => guardUploadApi(api, current),
    get: async <T>(path: string) => {
      check();
      const result = await api.get<T>(path);
      check();
      return result;
    },
    post: async <T>(path: string, body?: unknown) => {
      check();
      const result = await api.post<T>(path, body);
      check();
      return result;
    },
  };
}

export async function pickUploadSource(
  api: ScreenApi | undefined,
  purpose: Schema['NativeUploadBeginRequestDTO']['purpose'],
  request: FilePickerRequest,
  bindings: UploadBindings = {},
  ownerCurrent: () => boolean = () => true,
): Promise<UploadSource | null> {
  if (!api) throw new Error('Connect to an agent before choosing a file.');
  if (!ownerCurrent()) return null;
  if (api.selectUploadSource) {
    const selected = await api.selectUploadSource(purpose, request, bindings);
    if (!ownerCurrent()) {
      try {
        await selected?.close();
      } catch (error) {
        reportTransferFailure(selected?.uploadApi, error);
        throw error;
      }
      return null;
    }
    return selected;
  }
  // Retained for older adapters. The supported desktop adapter always selects
  // a native handle, including when its agent needs legacy transfer support.
  const scoped = api.snapshot?.() ?? api;
  const source = await (await getPlatform()).pickFileStream(request);
  if (!ownerCurrent()) {
    await source?.close();
    return null;
  }
  return source ? { ...source, uploadApi: scoped, uploadBindings: bindings } : null;
}
export function sourceApi(
  source: UploadSource | undefined,
  fallback: ScreenApi | undefined,
): ScreenApi | undefined {
  return source?.uploadApi ?? fallback?.snapshot?.() ?? fallback;
}
export function nativeProgress(progress: NativeUploadProgress): FileUploadProgress {
  return {
    phase: progress.phase,
    bytesUploaded: progress.bytesTransferred,
    totalBytes: progress.totalBytes,
    bytesPerSecond: progress.bytesPerSecond,
    transport: progress.transport,
  };
}

export async function discardUpload(api: ScreenApi | undefined, id: string): Promise<void> {
  if (!api?.delete) throw new Error('Agent cleanup could not be confirmed.');
  try {
    await api.delete(`/v1/staged-uploads/${id}`);
  } catch (error) {
    // An accepted import, including a failed one, owns its cleanup. Never try
    // to take its input back after a lost reply or a host switch.
    try {
      const status = await (api.cleanupStatus ?? api.get)<Schema['NativeUploadStatusDTO']>(
        `/v1/staged-uploads/${id}/status`,
      );
      if (status.state === 'redeemed') return;
    } catch {
      /* Keep the actual failed deletion as the reported outcome. */
    }
    throw error;
  }
}
