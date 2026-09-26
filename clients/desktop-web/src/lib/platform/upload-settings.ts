const STORAGE_KEY = 'msc2.upload-chunk-mib';
export const MIN_UPLOAD_CHUNK_MIB = 1;
export const MAX_UPLOAD_CHUNK_MIB = 8;
export const DEFAULT_UPLOAD_CHUNK_MIB = 2;

export function preferredUploadChunkMiB(): number {
  try {
    const stored = Number(localStorage.getItem(STORAGE_KEY));
    if (
      Number.isInteger(stored) &&
      stored >= MIN_UPLOAD_CHUNK_MIB &&
      stored <= MAX_UPLOAD_CHUNK_MIB
    ) {
      return stored;
    }
  } catch {
    // Storage can be unavailable in private or restricted browser contexts.
  }
  return DEFAULT_UPLOAD_CHUNK_MIB;
}

export function savePreferredUploadChunkMiB(value: number): void {
  if (!Number.isInteger(value) || value < MIN_UPLOAD_CHUNK_MIB || value > MAX_UPLOAD_CHUNK_MIB) {
    return;
  }
  try {
    localStorage.setItem(STORAGE_KEY, String(value));
  } catch {
    // The current upload can still use the selected value without persistence.
  }
}
