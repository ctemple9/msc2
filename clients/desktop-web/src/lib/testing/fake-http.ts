import type { components } from '../api/generated';
import { FakeAuth, type PermissionCategory } from './fake-auth';

export type HttpMethod = 'DELETE' | 'GET' | 'POST' | 'PUT';

export interface HttpRequest {
  body?: unknown;
  headers: Record<string, string>;
  method: HttpMethod;
  path: string;
}

export interface FakeResponse<T> {
  readonly headers: Record<string, string>;
  readonly status: number;
  bytes(): Promise<Uint8Array>;
  json(): Promise<T>;
}

interface RouteOptions {
  headers?: Record<string, string>;
  permission?: PermissionCategory;
  status?: number;
}

type Handler = (request: HttpRequest) => FakeResponse<unknown> | Promise<FakeResponse<unknown>>;
type RegisteredHandler = Handler & { permission?: PermissionCategory };

function copy<T>(value: T): T {
  return structuredClone(value);
}

function jsonResponse<T>(body: T, options: RouteOptions = {}): FakeResponse<T> {
  return {
    headers: { 'content-type': 'application/json', ...options.headers },
    status: options.status ?? 200,
    bytes: async () => new TextEncoder().encode(JSON.stringify(body)),
    json: async () => copy(body),
  };
}

function errorResponse(
  code: string,
  message: string,
  status: number,
): FakeResponse<components['schemas']['ErrorDTO']> {
  return jsonResponse(
    {
      code,
      message,
      helpId: null,
    },
    { status },
  );
}

export interface FakeHttpOptions {
  auth?: FakeAuth;
}

/** A deterministic route table that makes auth and response DTOs visible in tests. */
export class FakeHttp {
  readonly requests: HttpRequest[] = [];
  private readonly auth?: FakeAuth;
  private readonly routes = new Map<string, RegisteredHandler>();

  constructor(options: FakeHttpOptions = {}) {
    this.auth = options.auth;
  }

  onJson<T>(method: HttpMethod, path: string, body: T, options: RouteOptions = {}): void {
    const handler: RegisteredHandler = (request) => jsonResponse(body, options);
    handler.permission = options.permission;
    this.routes.set(`${method} ${path}`, handler);
  }

  onBytes(method: HttpMethod, path: string, bytes: Uint8Array, options: RouteOptions = {}): void {
    this.routes.set(`${method} ${path}`, () => ({
      headers: { 'content-type': options.headers?.['content-type'] ?? 'application/octet-stream' },
      status: options.status ?? 200,
      bytes: async () => bytes.slice(),
      json: async () => {
        throw new Error('binary response has no JSON body');
      },
    }));
  }

  async request<T>(
    method: HttpMethod,
    path: string,
    options: Omit<HttpRequest, 'method' | 'path'> = {},
  ): Promise<FakeResponse<T>> {
    const request: HttpRequest = {
      method,
      path,
      headers: { ...options.headers },
      body: options.body,
    };
    this.requests.push(request);

    const handler = this.routes.get(`${method} ${path}`);
    if (!handler) {
      return errorResponse(
        'not_found',
        `No fixture route for ${method} ${path}`,
        404,
      ) as FakeResponse<T>;
    }

    const requiredPermission = handler.permission ?? 'none';
    if (this.auth && !this.auth.authorize(request.headers, requiredPermission)) {
      const hasCredential = this.auth.authorize(request.headers, 'none');
      return errorResponse(
        hasCredential ? 'forbidden' : 'unauthorized',
        hasCredential
          ? 'The fixture credential lacks this permission.'
          : 'The fixture request is unauthenticated.',
        hasCredential ? 403 : 401,
      ) as FakeResponse<T>;
    }

    return (await handler(request)) as FakeResponse<T>;
  }
}
