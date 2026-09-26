import type { components } from './generated';
import type { TransportCredentialAdapter } from './auth';
import { cookieCredentialAdapter } from './auth';
import type { FileChunkSource, FileUploadProgress } from '../platform/types';

export type HttpMethod = 'DELETE' | 'GET' | 'POST' | 'PUT';
export type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

export interface ApiClientOptions {
  baseUrl: string;
  hostId: string;
  fetchImpl?: FetchLike;
  credentialAdapter?: TransportCredentialAdapter;
  clientApiVersion?: string;
  maxDownloadBytes?: number;
  onCapabilities?: (capabilities: components['schemas']['CapabilitiesDTO']) => void;
}

export interface JsonRequestOptions {
  body?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
}

export interface FileUploadOptions {
  chunkSizeBytes?: number;
  onProgress?: (progress: FileUploadProgress) => void;
  signal?: AbortSignal;
}

export class UploadCancelledError extends Error {
  constructor() {
    super('The upload was cancelled.');
    this.name = 'UploadCancelledError';
  }
}

const MIN_UPLOAD_CHUNK_BYTES = 1024 * 1024;
const DEFAULT_UPLOAD_CHUNK_BYTES = 2 * 1024 * 1024;
const MAX_UPLOAD_CHUNK_BYTES = 8 * 1024 * 1024;

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly error: components['schemas']['ErrorDTO'],
  ) {
    super(error.message);
    this.name = 'ApiError';
  }
}

export type CompatibilityState = 'unknown' | 'supported' | 'old-agent' | 'unsupported-client';

/** One host-aware HTTP surface for the browser and Tauri clients. */
export class ApiClient {
  private readonly baseUrl: string;
  private readonly hostId: string;
  private readonly fetchImpl: FetchLike;
  private readonly credentialAdapter: TransportCredentialAdapter;
  private readonly clientApiVersion: string;
  private readonly maxDownloadBytes: number;
  private readonly onCapabilities?: ApiClientOptions['onCapabilities'];
  private refreshingCapabilities = false;
  private capabilitiesValue: components['schemas']['CapabilitiesDTO'] | null = null;
  private apiVersionValue: string | null = null;
  private compatibilityValue: CompatibilityState = 'unknown';

  constructor(options: ApiClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, '');
    this.hostId = options.hostId;
    // `window.fetch` requires its window receiver in real browsers. Wrapping it
    // keeps the injected-test seam while avoiding an illegal-invocation error.
    this.fetchImpl = options.fetchImpl ?? ((input, init) => fetch(input, init));
    this.credentialAdapter = options.credentialAdapter ?? cookieCredentialAdapter();
    this.clientApiVersion = options.clientApiVersion ?? '1.0';
    this.maxDownloadBytes = options.maxDownloadBytes ?? 512 * 1024 * 1024;
    this.onCapabilities = options.onCapabilities;
  }

  get host(): string {
    return this.hostId;
  }

  /** Builds an absolute URL for non-JSON resources rendered by the UI. */
  resourceUrl(path: string): string {
    return this.urlFor(path);
  }

  get capabilities(): components['schemas']['CapabilitiesDTO'] | null {
    return this.capabilitiesValue ? structuredClone(this.capabilitiesValue) : null;
  }

  get apiVersion(): string | null {
    return this.apiVersionValue;
  }

  get compatibility(): CompatibilityState {
    return this.compatibilityValue;
  }

  async requestJson<T>(
    method: HttpMethod,
    path: string,
    options: JsonRequestOptions = {},
  ): Promise<T> {
    const response = await this.request(method, path, options);
    if (response.status === 204) {
      return undefined as T;
    }
    return (await response.json()) as T;
  }

  async requestBytes(
    method: HttpMethod,
    path: string,
    options: Omit<JsonRequestOptions, 'body'> = {},
  ): Promise<Uint8Array> {
    const response = await this.request(method, path, options);
    return new Uint8Array(await response.arrayBuffer());
  }

  async getCapabilities(): Promise<components['schemas']['CapabilitiesDTO']> {
    const capabilities = await this.requestJson<components['schemas']['CapabilitiesDTO']>(
      'GET',
      '/v1/capabilities',
    );
    this.capabilitiesValue = structuredClone(capabilities);
    this.compatibilityValue = 'supported';
    this.onCapabilities?.(structuredClone(capabilities));
    return structuredClone(capabilities);
  }

  async getOperation(id: string): Promise<components['schemas']['OperationDTO']> {
    return this.requestJson('GET', `/v1/operations/${encodeURIComponent(id)}`);
  }

  async cancelOperation(id: string): Promise<components['schemas']['OperationDTO']> {
    return this.requestJson('POST', `/v1/operations/${encodeURIComponent(id)}/cancel`);
  }

  async beginUpload(
    request: components['schemas']['StagedUploadBeginRequestDTO'],
  ): Promise<components['schemas']['StagedUploadBeginResultDTO']> {
    return this.requestJson('POST', '/v1/staged-uploads', { body: request });
  }

  async uploadBytes(
    uploadPath: string,
    bytes: Uint8Array,
    maxBytes: number,
  ): Promise<components['schemas']['StagedUploadCompleteResultDTO']> {
    if (bytes.byteLength > maxBytes) {
      throw new Error(`staged upload exceeds ${maxBytes} bytes`);
    }
    return this.requestJson('PUT', uploadPath, {
      body: bytes,
      headers: { 'Content-Type': 'application/octet-stream' },
    });
  }

  async stagedUpload(
    request: components['schemas']['StagedUploadBeginRequestDTO'],
    bytes: Uint8Array,
  ): Promise<components['schemas']['StagedUploadCompleteResultDTO']> {
    const slot = await this.beginUpload(request);
    return this.uploadBytes(slot.uploadPath, bytes, slot.maxBytes);
  }

  async stagedUploadFromFile(
    request: components['schemas']['StagedUploadBeginRequestDTO'],
    source: FileChunkSource,
    options: FileUploadOptions = {},
  ): Promise<components['schemas']['StagedUploadCompleteResultDTO']> {
    if (!Number.isSafeInteger(source.size) || source.size <= 0) {
      throw new Error('The selected file has an invalid size.');
    }
    const requestedChunkSize = options.chunkSizeBytes ?? DEFAULT_UPLOAD_CHUNK_BYTES;
    if (
      !Number.isSafeInteger(requestedChunkSize) ||
      requestedChunkSize < MIN_UPLOAD_CHUNK_BYTES ||
      requestedChunkSize > MAX_UPLOAD_CHUNK_BYTES
    ) {
      throw new Error('Upload chunk size must be between 1 and 8 MiB.');
    }

    const onProgress = options.onProgress;
    const signal = options.signal;
    let slot: components['schemas']['StagedUploadBeginResultDTO'] | undefined;
    let cleanupPromise: Promise<void> | undefined;
    let complete = false;
    const cleanup = (): Promise<void> => {
      if (!slot) return Promise.resolve();
      cleanupPromise ??= this.cancelStagedUpload(slot.stagedUploadId);
      return cleanupPromise;
    };
    const cleanupOnAbort = () => {
      void cleanup().catch(() => undefined);
    };

    signal?.addEventListener('abort', cleanupOnAbort, { once: true });
    try {
      onProgress?.({ phase: 'preparing', bytesUploaded: 0, totalBytes: source.size });
      const activeSlot = await this.beginUpload({ ...request, expectedBytes: source.size });
      slot = activeSlot;
      if (signal?.aborted) throw new UploadCancelledError();
      if (
        (request.purpose === 'world-import' || request.purpose === 'active-world-replace') &&
        activeSlot.maxChunkBytes === undefined
      ) {
        throw new Error(
          'This host agent does not support streamed world archives. Update the MSC 2 agent and try again.',
        );
      }
      const hostChunkLimit = activeSlot.maxChunkBytes ?? DEFAULT_UPLOAD_CHUNK_BYTES;
      if (!Number.isSafeInteger(hostChunkLimit) || hostChunkLimit <= 0) {
        throw new Error('The host returned an invalid maximum upload chunk size.');
      }
      const chunkSize = Math.min(requestedChunkSize, hostChunkLimit, MAX_UPLOAD_CHUNK_BYTES);
      if (source.size > activeSlot.maxBytes) {
        throw new Error(`staged upload exceeds ${activeSlot.maxBytes} bytes`);
      }
      onProgress?.({
        phase: 'preparing',
        bytesUploaded: 0,
        totalBytes: source.size,
        chunkSizeBytes: chunkSize,
      });
      let offset = 0;
      while (offset < source.size) {
        if (signal?.aborted) throw new UploadCancelledError();
        onProgress?.({
          phase: 'reading',
          bytesUploaded: offset,
          totalBytes: source.size,
          chunkSizeBytes: chunkSize,
        });
        const expectedChunkBytes = Math.min(chunkSize, source.size - offset);
        const bytes = await source.readChunk(offset, expectedChunkBytes);
        if (signal?.aborted) throw new UploadCancelledError();
        if (bytes.byteLength !== expectedChunkBytes) {
          throw new Error(`Could not read the complete file at byte ${offset}.`);
        }
        onProgress?.({
          phase: 'uploading',
          bytesUploaded: offset,
          totalBytes: source.size,
          chunkSizeBytes: chunkSize,
        });
        const finalChunk = offset + bytes.byteLength === source.size;
        const path = `${activeSlot.uploadPath}/chunks?offset=${offset}&complete=${finalChunk}`;
        // The desktop transport proxies this request through a native command
        // that cannot be interrupted once Rust starts sending it. Wait for the
        // active chunk to settle before deleting its staging slot.
        const response = await this.request('PUT', path, {
          body: bytes,
          headers: { 'Content-Type': 'application/octet-stream' },
          signal,
        });
        if (signal?.aborted) throw new UploadCancelledError();
        if (finalChunk) {
          const result =
            (await response.json()) as components['schemas']['StagedUploadCompleteResultDTO'];
          if (
            result.receivedBytes !== source.size ||
            result.stagedUploadId !== activeSlot.stagedUploadId
          ) {
            throw new Error('The agent returned an incomplete file upload result.');
          }
          if (signal?.aborted) throw new UploadCancelledError();
          offset += bytes.byteLength;
          complete = true;
          onProgress?.({
            phase: 'complete',
            bytesUploaded: offset,
            totalBytes: source.size,
            chunkSizeBytes: chunkSize,
          });
          return result;
        }
        if (response.status !== 204) throw new Error('The agent did not accept the file chunk.');
        offset += bytes.byteLength;
      }
      throw new Error('The selected file is empty.');
    } catch (error) {
      let cleanupError: unknown;
      if (slot && !complete) {
        try {
          await cleanup();
        } catch (failure) {
          cleanupError = failure;
        }
      }
      if (cleanupError !== undefined) {
        const reason = error instanceof Error ? error.message : String(error);
        throw new Error(
          `${reason} The host could not confirm removal of the partial file: ${String(cleanupError)}`,
        );
      }
      if (signal?.aborted) throw new UploadCancelledError();
      throw error;
    } finally {
      signal?.removeEventListener('abort', cleanupOnAbort);
    }
  }

  async cancelStagedUpload(stagedUploadId: string): Promise<void> {
    await this.requestJson<void>(
      'DELETE',
      `/v1/staged-uploads/${encodeURIComponent(stagedUploadId)}`,
    );
  }

  async downloadBytes(
    stagedDownloadId: string,
    maxBytes = this.maxDownloadBytes,
  ): Promise<Uint8Array> {
    const response = await this.request(
      'GET',
      `/v1/staged-downloads/${encodeURIComponent(stagedDownloadId)}`,
    );
    const declaredLength = Number(response.headers.get('Content-Length'));
    if (Number.isFinite(declaredLength) && declaredLength > maxBytes) {
      throw new Error(`staged download exceeds ${maxBytes} bytes`);
    }
    if (!response.body) return new Uint8Array(await response.arrayBuffer());

    const reader = response.body.getReader();
    const chunks: Uint8Array[] = [];
    let received = 0;
    try {
      while (true) {
        const next = await reader.read();
        if (next.done) break;
        received += next.value.byteLength;
        if (received > maxBytes) throw new Error(`staged download exceeds ${maxBytes} bytes`);
        chunks.push(next.value);
      }
    } finally {
      reader.releaseLock();
    }
    const bytes = new Uint8Array(received);
    let offset = 0;
    for (const chunk of chunks) {
      bytes.set(chunk, offset);
      offset += chunk.byteLength;
    }
    return bytes;
  }

  private async request(
    method: HttpMethod,
    path: string,
    options: JsonRequestOptions = {},
  ): Promise<Response> {
    const authHeaders = this.credentialAdapter.headersForRequest
      ? await this.credentialAdapter.headersForRequest(this.hostId, method)
      : await this.credentialAdapter.headersFor(this.hostId);
    const headers: Record<string, string> = {
      Accept: 'application/json',
      'X-MSC-Client-Api-Version': this.clientApiVersion,
      ...authHeaders,
      ...options.headers,
    };
    let body: BodyInit | undefined;
    if (options.body !== undefined) {
      if (options.body instanceof Uint8Array || typeof options.body === 'string') {
        body = options.body as BodyInit;
      } else {
        headers['Content-Type'] ??= 'application/json';
        body = JSON.stringify(options.body);
      }
    }

    let response: Response;
    try {
      response = await this.fetchImpl(this.urlFor(path), {
        method,
        headers,
        body,
        credentials: this.credentialAdapter.requestCredentials,
        signal: options.signal,
      });
    } catch (error) {
      throw new Error(`Unable to reach host '${this.hostId}': ${String(error)}`);
    }

    await this.observeResponseVersion(response);
    if (!response.ok) {
      const error = await readError(response);
      if (response.status === 426 || error.code === 'client_version_unsupported') {
        this.compatibilityValue = 'unsupported-client';
      }
      throw new ApiError(response.status, error);
    }
    return response;
  }

  private async observeResponseVersion(response: Response): Promise<void> {
    const version = response.headers.get('X-MSC-Api-Version');
    if (!version || version === this.apiVersionValue) {
      return;
    }
    const hadVersion = this.apiVersionValue !== null;
    this.apiVersionValue = version;
    if (hadVersion && !this.refreshingCapabilities && !this.capabilitiesPath(response.url)) {
      this.refreshingCapabilities = true;
      try {
        await this.getCapabilities();
      } catch {
        this.compatibilityValue = 'old-agent';
      } finally {
        this.refreshingCapabilities = false;
      }
    }
  }

  private capabilitiesPath(url: string): boolean {
    return new URL(url).pathname.endsWith('/v1/capabilities');
  }

  private urlFor(path: string): string {
    return path.startsWith('http://') || path.startsWith('https://')
      ? path
      : `${this.baseUrl}${path.startsWith('/') ? path : `/${path}`}`;
  }
}

async function readError(response: Response): Promise<components['schemas']['ErrorDTO']> {
  try {
    const body = (await response.json()) as Partial<components['schemas']['ErrorDTO']>;
    return {
      code: body.code ?? 'http_error',
      message: body.message ?? `Request failed with HTTP ${response.status}`,
      helpId: body.helpId ?? null,
      details: body.details ?? null,
    };
  } catch {
    return {
      code: 'http_error',
      message: `Request failed with HTTP ${response.status}`,
      helpId: null,
    };
  }
}
