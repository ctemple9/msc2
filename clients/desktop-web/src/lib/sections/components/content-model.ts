import type { Schema, ScreenApi } from '../shared/types';
import { pollOperation } from './model';
import { getPlatform } from '../../platform';
import { reportTransferFailure } from '../shared/transfer-notices';
export interface ContentEntry {
  path: string;
  sha512: string;
  byteSize: number;
  audience: 'server' | 'players' | 'both';
  optional: boolean;
  source?: { provider: string; projectId: string; versionId: string } | null;
  redistributionEvidence?: string | null;
  classificationEvidence: string[];
  unresolvedReasons: string[];
  enabled?: boolean;
  [key: string]: unknown;
}
export interface ContentCollection {
  revision: number;
  currentFiles: Record<string, ContentEntry>;
  playerFiles: Record<string, ContentEntry>;
  upstreamFiles: Record<string, ContentEntry>;
  upstreamPlayerFiles: Record<string, ContentEntry>;
  upstream?: { provider: string; projectId: string; versionId: string } | null;
  basePackId?: string;
  [key: string]: unknown;
}
export interface ContentSnapshot {
  collection: ContentCollection;
  serverBaseline: Record<string, string>;
  requirements: { path: string; state: string; reason: string }[];
}
export interface ContentReview {
  reviewId: string;
  collectionRevision: number;
  collection: ContentCollection;
  changes: Schema['ContentChangeDTO'][];
  warnings: string[];
}
export interface PathChange {
  path: string;
  oldSha512?: string;
  currentSha512?: string;
  newSha512?: string;
  action: string;
  conflict: boolean;
  restored: boolean;
  reason: string;
}
export interface PackReview {
  replacement: boolean;
  reviewId: string;
  collectionRevision: number;
  packId: string;
  name: string;
  version: string;
  serverChanges: PathChange[];
  playerChanges: PathChange[];
  missing: string[];
  warnings: string[];
  collection: ContentCollection;
  missingFiles: {
    path: string;
    reason: string;
    byteSize?: number;
    manualRecoveryAvailable: boolean;
  }[];
}
export const contentRoot = (id: string) => `/v1/servers/${encodeURIComponent(id)}/content`;
export async function contentOperation(
  api: ScreenApi,
  path: string,
  body: unknown,
  progress: (line: string, amount?: { current: number; total: number } | null) => void,
  onOperation: (id: string) => void = () => {},
): Promise<Record<string, unknown>> {
  const accepted = await api.post<{ operationId: string }>(path, body);
  onOperation(accepted.operationId);
  const operation = await pollOperation(api, accepted.operationId, (value) =>
    progress(value.statusLine ?? 'Working…', value.progress),
  );
  if (operation?.state !== 'succeeded')
    throw new Error(
      operation?.error?.message ??
        (operation?.state === 'cancelled' ? 'Operation cancelled.' : 'Operation did not complete.'),
    );
  return operation.result ?? {};
}
export async function uploadReviewedFile(
  api: ScreenApi,
  prefix: string,
  path: string,
  signal: AbortSignal,
  progress: (bytes: number) => void,
): Promise<void> {
  const source = await (
    await getPlatform()
  ).pickFileStream({ label: `Choose ${path.split('/').pop()}`, extensions: [] });
  if (!source) return;
  let stagedId: string | undefined;
  let consumed = false;
  try {
    if (signal.aborted || api.isCurrent?.() === false)
      throw new Error('The selected host changed or upload was cancelled.');
    if (!api.uploadAdmittedFile)
      throw new Error('Update this desktop app to upload reviewed files.');
    const admission = await api.post<Schema['StagedUploadBeginResultDTO']>(`${prefix}/admit`, {
      path,
    });
    stagedId = admission.stagedUploadId;
    if (source.size !== admission.maxBytes || source.name !== path.split('/').pop())
      throw new Error('Choose the exact original filename and size. MSC verifies its hashes.');
    const receipt = await api.uploadAdmittedFile(admission, source, {
      signal,
      onProgress: (value) => progress(value.bytesUploaded),
    });
    await api.post(`${prefix}/complete`, { path, stagedUploadId: receipt.stagedUploadId });
    consumed = true;
  } finally {
    if (stagedId && !consumed) {
      try {
        if (!api.delete) throw new Error('Partial upload cleanup unavailable.');
        await api.delete(`/v1/staged-uploads/${encodeURIComponent(stagedId)}`);
      } catch (error) {
        reportTransferFailure(api, error);
      }
    }
    try {
      await source.close();
    } catch (error) {
      reportTransferFailure(api, error);
    }
  }
}
