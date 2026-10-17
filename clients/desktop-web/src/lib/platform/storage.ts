import type { components } from '../api/generated';

export type StorageSnapshot = components['schemas']['StorageSnapshotDTO'];
export type StorageItem = components['schemas']['StorageItemDTO'];
export type StorageVolume = components['schemas']['StorageVolumeDTO'];
export type StorageScan = components['schemas']['StorageScanDTO'];
export interface LocalFootprint {
  localId: string;
  agentHostId: string | null;
  snapshot: StorageSnapshot | null;
  scanId: string | null;
  done: boolean;
  historical: boolean;
  progress: components['schemas']['StorageProgressDTO'];
  problem: string | null;
}
export interface StorageSummary {
  hostId: string;
  snapshotId: string;
  logicalBytes: number;
  partial: boolean;
  completedAt: number;
  volumes: StorageVolume[];
}
export interface InventoryReply {
  hostId: string;
  historical: boolean;
  snapshot: StorageSnapshot | null;
}
export interface ScanAccepted { hostId: string; operationId: string }
export type StorageCandidates = components['schemas']['StorageCandidatesDTO'];
export type StorageReview = components['schemas']['StorageReviewDTO'];
export type StorageResult = components['schemas']['StorageReviewedResultDTO'];
export type StorageSelection = components['schemas']['StorageReviewRequestDTO'];
export type StorageConfirmation = components['schemas']['StorageCleanupRequestDTO'];
export interface StorageBridge {
  candidates(selection: StorageSelection): Promise<StorageCandidates>;
  review(selection: StorageSelection): Promise<StorageReview>;
  apply(confirmation: StorageConfirmation): Promise<ScanAccepted>;
  result(agentHostId: string, operationId: string): Promise<StorageResult>;
  running(): Promise<Record<string, string>>;
  local(action: 'cached' | 'refresh' | 'status' | 'cancel'): Promise<LocalFootprint>;
  cached(agentHostId: string): Promise<StorageSnapshot | null>;
  overview(agentHostIds: string[]): Promise<StorageSummary[]>;
  composed(agentHostId: string): Promise<StorageSnapshot | null>;
  remote<T>(agentHostId: string, action: 'inventory' | 'refresh' | 'status' | 'cancel', operationId?: string): Promise<T>;
  reveal(itemId: string, agentHostId?: string): Promise<void>;
}

export async function loadStorageBridge(): Promise<StorageBridge> {
  const { invoke } = await import('@tauri-apps/api/core');
  const selected = <T>(action: string, selection: StorageSelection) => invoke<T>('storage_remote', { request: {agentHostId: selection.hostId, action, selection} });
  return {
    candidates: selection => selected<StorageCandidates>('candidates',selection),
    review: selection => selected<StorageReview>('review',selection),
    apply: cleanup => invoke<ScanAccepted>('storage_remote',{request:{agentHostId:cleanup.hostId,action:'cleanup',cleanup}}),
    result: (agentHostId,operationId) => invoke<StorageResult>('storage_remote',{request:{agentHostId,action:'result',operationId}}),
    running: () => invoke<Record<string,string>>('storage_active_scans'),
    local: (action) => invoke<LocalFootprint>('storage_local', { action }),
    cached: (agentHostId) => invoke<StorageSnapshot | null>('storage_cached', { agentHostId }),
    overview: (agentHostIds) => invoke<StorageSummary[]>('storage_overview', { agentHostIds }),
    composed: (agentHostId) => invoke<StorageSnapshot | null>('storage_composed', { agentHostId }),
    remote: <T>(agentHostId: string, action: string, operationId?: string) =>
      invoke<T>('storage_remote', { request: { agentHostId, action, operationId: operationId ?? null } }),
    reveal: (itemId, agentHostId) => invoke<void>('storage_reveal', { itemId, agentHostId: agentHostId ?? null }),
  };
}

/** Keep late replies on their original computer, even after removal/re-pairing. */
export class StorageSession {
  private generations = new Map<string, number>();
  constructor(readonly bridge: StorageBridge) {}
  invalidate(hostId: string): void {
    this.generations.set(hostId, (this.generations.get(hostId) ?? 0) + 1);
  }
  async cached(hostId: string): Promise<StorageSnapshot | null> {
    const generation = this.generations.get(hostId) ?? 0;
    const snapshot = await this.bridge.cached(hostId);
    this.check(hostId, generation);
    if (snapshot && snapshot.hostId !== hostId) throw new Error('Storage belongs to another computer.');
    return snapshot;
  }
  async remote<T extends { hostId: string }>(hostId: string, action: 'inventory' | 'refresh' | 'status', operationId?: string): Promise<T> {
    const generation = this.generations.get(hostId) ?? 0;
    const result = await this.bridge.remote<T>(hostId, action, operationId);
    this.check(hostId, generation);
    if (result.hostId !== hostId) throw new Error('Storage reply belongs to another computer.');
    return result;
  }
  private check(hostId: string, generation: number): void {
    if ((this.generations.get(hostId) ?? 0) !== generation) throw new Error('Computer authorization changed during storage request.');
  }
}
