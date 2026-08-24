import { describe, expect, it } from 'vitest';
import oldAgentCapabilities from '../fixtures/old-agent-capabilities.json';
import newAgentCapabilities from '../fixtures/new-agent-capabilities.json';
import operationProgress from '../fixtures/operation-progress.json';
import consoleHistory from '../fixtures/console-history.json';
import type { components } from '../../src/lib/api/generated';
import { FakeAuth } from '../../src/lib/testing/fake-auth';
import { FakeHttp } from '../../src/lib/testing/fake-http';
import { FakeOperationStore } from '../../src/lib/testing/fake-operations';
import { FakeTransferStore } from '../../src/lib/testing/fake-transfers';
import { FakeWebSocket, type ConsoleLineDTO } from '../../src/lib/testing/fake-websocket';

const adminAuth = (): FakeAuth => {
  const auth = new FakeAuth();
  auth.addCredential({
    id: 'admin',
    permissions: ['serverControl', 'players', 'worlds', 'admin'],
    scheme: 'bearer',
    secret: 'admin-secret',
  });
  auth.addCredential({
    id: 'guest',
    permissions: ['players'],
    scheme: 'cookie',
    secret: 'guest-session',
  });
  return auth;
};

describe('contract-backed client harness', () => {
  it('keeps authentication and permission failures in the ErrorDTO envelope', async () => {
    const auth = adminAuth();
    const http = new FakeHttp({ auth });
    const me: components['schemas']['MeResponseDTO'] = {
      isNamedToken: true,
      name: 'admin',
      permissions: ['serverControl', 'admin'],
      role: 'admin',
    };
    http.onJson<components['schemas']['MeResponseDTO']>('GET', '/v1/me', me);
    http.onJson<components['schemas']['SimpleResult']>(
      'POST',
      '/v1/start',
      { result: 'start_requested' },
      { permission: 'serverControl' },
    );

    const unauthenticated = await http.request<components['schemas']['ErrorDTO']>('GET', '/v1/me');
    expect(unauthenticated.status).toBe(401);
    expect((await unauthenticated.json()).code).toBe('unauthorized');

    const cookieResponse = await http.request<components['schemas']['MeResponseDTO']>(
      'GET',
      '/v1/me',
      {
        headers: auth.headersFor('guest'),
      },
    );
    expect(cookieResponse.status).toBe(200);
    expect((await cookieResponse.json()).role).toBe('admin');

    const forbidden = await http.request<components['schemas']['ErrorDTO']>('POST', '/v1/start', {
      headers: auth.headersFor('guest'),
    });
    expect(forbidden.status).toBe(403);
    expect((await forbidden.json()).code).toBe('forbidden');

    const allowed = await http.request<components['schemas']['SimpleResult']>('POST', '/v1/start', {
      headers: auth.headersFor('admin'),
    });
    expect(await allowed.json()).toEqual({ result: 'start_requested' });
  });

  it('accepts old and new capability shapes without requiring future keys', async () => {
    const oldHttp = new FakeHttp();
    oldHttp.onJson<components['schemas']['CapabilitiesDTO']>(
      'GET',
      '/v1/capabilities',
      oldAgentCapabilities as components['schemas']['CapabilitiesDTO'],
    );
    const oldCapabilities = await (
      await oldHttp.request<components['schemas']['CapabilitiesDTO']>('GET', '/v1/capabilities')
    ).json();
    expect(oldCapabilities.serverTypes.bedrock.runtime).toBeUndefined();
    expect((oldCapabilities as Record<string, unknown>).futureCapability).toBeUndefined();

    const newHttp = new FakeHttp();
    newHttp.onJson<components['schemas']['CapabilitiesDTO']>(
      'GET',
      '/v1/capabilities',
      newAgentCapabilities as components['schemas']['CapabilitiesDTO'],
    );
    const newCapabilities = await (
      await newHttp.request<components['schemas']['CapabilitiesDTO']>('GET', '/v1/capabilities')
    ).json();
    expect(newCapabilities.serverTypes.bedrock.runtime?.state).toBe('available');
    expect((newCapabilities as Record<string, unknown>).futureCapability).toEqual({
      profileRoutes: false,
    });
  });

  it('keeps operation snapshots usable across queued, running, and terminal states', () => {
    const store = new FakeOperationStore();
    const [queued, runningFixture, terminalFixture] = operationProgress;
    store.add(queued as components['schemas']['OperationDTO']);

    const running = store.update('op-install-1', {
      state: 'running',
      progress: runningFixture.progress,
      statusLine: 'Still downloading',
    });
    expect(running.progress).toEqual({ current: 1, total: 2 });
    expect(store.get('op-install-1')?.state).toBe('running');

    const terminal = store.update('op-install-1', {
      ...terminalFixture,
      state: 'succeeded',
      result: null,
    });
    expect(terminal.state).toBe('succeeded');
  });

  it('bounds staged uploads and returns download bytes without filesystem access', () => {
    const transfers = new FakeTransferStore();
    const begin = transfers.beginUpload(
      { purpose: 'world-import', contentType: 'application/zip' },
      4,
    );
    const complete = transfers.completeUpload(begin.stagedUploadId, new Uint8Array([1, 2, 3]));
    expect(complete.receivedBytes).toBe(3);
    expect(() =>
      transfers.completeUpload(begin.stagedUploadId, new Uint8Array([1, 2, 3, 4, 5])),
    ).toThrow('exceeds 4 bytes');

    const downloadId = transfers.addDownload(new Uint8Array([9, 8, 7]));
    expect([...transfers.download(downloadId)]).toEqual([9, 8, 7]);
  });

  it('reconnects bounded console history and operation snapshots deterministically', () => {
    const consoleStream = new FakeWebSocket<ConsoleLineDTO>(consoleHistory, 2);
    expect(consoleStream.connect().map((line) => line.text)).toEqual(['Done', 'Ready']);
    expect(
      consoleStream
        .push({
          ts: '2026-08-24T00:00:03Z',
          source: 'server',
          text: 'Live',
        })
        .map((line) => line.text),
    ).toEqual(['Live']);
    consoleStream.disconnect();
    consoleStream.push({ ts: '2026-08-24T00:00:04Z', source: 'server', text: 'While away' });
    expect(consoleStream.reconnect().map((line) => line.text)).toEqual(['Live', 'While away']);
    expect(consoleStream.connectionCount).toBe(2);

    const operationStream = new FakeWebSocket<components['schemas']['OperationDTO']>(
      operationProgress as components['schemas']['OperationDTO'][],
      1,
    );
    expect(operationStream.connect()[0].state).toBe('succeeded');
    expect(operationStream.live).toBe(true);
  });
});
