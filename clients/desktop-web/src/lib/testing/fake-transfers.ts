import type { components } from '../api/generated';

interface UploadRecord {
  bytes?: Uint8Array;
  maxBytes: number;
  request: components['schemas']['StagedUploadBeginRequestDTO'];
}

/** Keeps staged transfers in memory and enforces the contract's one-use byte ceiling. */
export class FakeTransferStore {
  private readonly downloads = new Map<string, Uint8Array>();
  private readonly uploads = new Map<string, UploadRecord>();
  private nextId = 1;

  addDownload(bytes: Uint8Array): string {
    const id = `download-${this.nextId++}`;
    this.downloads.set(id, bytes.slice());
    return id;
  }

  beginUpload(
    request: components['schemas']['StagedUploadBeginRequestDTO'],
    maxBytes = 1024,
  ): components['schemas']['StagedUploadBeginResultDTO'] {
    const id = `upload-${this.nextId++}`;
    this.uploads.set(id, { maxBytes, request: { ...request } });
    return {
      stagedUploadId: id,
      uploadPath: `/v1/staged-uploads/${id}`,
      maxBytes,
      expiresAt: '2026-08-24T00:00:00Z',
    };
  }

  completeUpload(
    id: string,
    bytes: Uint8Array,
  ): components['schemas']['StagedUploadCompleteResultDTO'] {
    const upload = this.uploads.get(id);
    if (!upload) {
      throw new Error(`unknown staged upload: ${id}`);
    }
    if (bytes.byteLength > upload.maxBytes) {
      throw new Error(`staged upload exceeds ${upload.maxBytes} bytes`);
    }

    upload.bytes = bytes.slice();
    return {
      stagedUploadId: id,
      receivedBytes: bytes.byteLength,
      sha256: `fixture-sha256-${bytes.byteLength}`,
    };
  }

  download(id: string): Uint8Array {
    const bytes = this.downloads.get(id);
    if (!bytes) {
      throw new Error(`unknown staged download: ${id}`);
    }
    return bytes.slice();
  }
}
