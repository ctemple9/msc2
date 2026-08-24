import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';

const root = new URL('../../../dist/', import.meta.url).pathname;
const port = Number(process.env.PORT ?? '4173');
const mime = {
  '.css': 'text/css',
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.json': 'application/json',
};

const topics = {
  'handbook.overview': {
    id: 'handbook.overview',
    kind: 'handbook',
    title: 'Overview',
    category: 'concepts',
    analogy: 'A server is a separate Minecraft room.',
    markdown: 'MSC manages servers and worlds.\n\n- Start a server\n- Choose a world',
    relatedIds: ['concept.server'],
  },
  'concept.server': {
    id: 'concept.server',
    kind: 'concept',
    title: 'One server. Your worlds.',
    category: 'concept-guide',
    markdown: 'A server holds worlds.',
    relatedIds: ['handbook.overview'],
  },
};
const concept = {
  id: 'concept-guide',
  pages: [
    {
      order: 1,
      helpId: 'concept.server',
      eyebrow: 'The Big Picture',
      title: 'One server. Your worlds.',
      body: 'A server contains worlds.',
      diagram: 'server-worlds',
      assetStatus: 'reviewed CSS replacement',
    },
  ],
};
const onboarding = {
  id: 'first-launch-tour',
  reopen: { label: 'Restart this tour', persistenceKey: 'msc_onboarding_tour_complete' },
  skip: {
    label: 'Skip tour',
    effect: 'marks the tour complete; it can be reopened from Preferences',
  },
  steps: [
    {
      order: 0,
      id: 'welcome',
      title: 'Welcome to MSC',
      body: 'Begin the guided tour.',
      actionLabel: "Let's go →",
      anchor: null,
      requiresUserAction: false,
    },
    {
      order: 1,
      id: 'manage-servers',
      title: 'Your Server List',
      body: 'Open your server list.',
      anchor: 'ob_manage_servers',
      requiresUserAction: true,
    },
    {
      order: 2,
      id: 'first-world',
      title: 'Create Your First World',
      body: 'Create a world.',
      anchor: 'ob_world_creation',
      requiresUserAction: false,
      hideCard: true,
    },
    {
      order: 3,
      id: 'done',
      title: "You're All Set",
      body: 'The tour is complete.',
      anchor: null,
      requiresUserAction: false,
      actionLabel: 'Finish',
    },
  ],
};
const servers = [
  {
    id: 'survival',
    name: 'Survival',
    directory: 'servers/survival',
    serverType: 'paper',
    gamePort: 25565,
  },
  {
    id: 'creative',
    name: 'Creative',
    directory: 'servers/creative',
    serverType: 'vanilla',
    gamePort: 25566,
  },
];
const worlds = [
  {
    id: 'world-1',
    name: 'Overworld',
    createdAt: '2026-08-20T12:00:00Z',
    isActive: true,
    hasThumbnail: false,
    worldSeed: '—',
    zipSizeBytes: 1024,
  },
];
const statusRequests = new Map();

function json(response, body, status = 200) {
  response.writeHead(status, { 'content-type': 'application/json' });
  response.end(JSON.stringify(body));
}
function bytes(response, body) {
  response.writeHead(200, { 'content-type': 'application/zip', 'content-length': body.byteLength });
  response.end(body);
}

createServer(async (request, response) => {
  const url = new URL(request.url ?? '/', `http://${request.headers.host}`);
  // The native Tauri binary has a tauri:// origin, unlike browser tests that
  // load this harness's page. Keep this CORS allowance inside the deterministic
  // test server so both surfaces exercise the same fake contract.
  // The shared client always includes credentials. CORS therefore has to echo
  // the native Tauri origin rather than using `*`, which WebKitGTK rejects for
  // credentialed requests.
  const origin = request.headers.origin;
  if (origin) response.setHeader('access-control-allow-origin', origin);
  response.setHeader('access-control-allow-credentials', 'true');
  response.setHeader(
    'access-control-allow-headers',
    'content-type, authorization, x-msc-client-api-version',
  );
  response.setHeader('access-control-allow-methods', 'GET, POST, PUT, OPTIONS');
  if (request.method === 'OPTIONS') {
    response.writeHead(204);
    return response.end();
  }
  if (url.pathname === '/v1/capabilities') return json(response, {});
  if (url.pathname === '/v1/me')
    return json(response, {
      permissions: ['admin', 'fleet', 'worlds', 'addons', 'settings', 'networking'],
    });
  if (url.pathname === '/v1/help/catalog')
    return json(response, {
      topics: Object.values(topics).map(({ id, kind, title, category }) => ({
        id,
        kind,
        title,
        category,
      })),
    });
  if (url.pathname.startsWith('/v1/help/'))
    return json(
      response,
      topics[decodeURIComponent(url.pathname.slice('/v1/help/'.length))] ?? {
        code: 'not_found',
        message: 'Unknown help topic',
        helpId: null,
      },
      topics[decodeURIComponent(url.pathname.slice('/v1/help/'.length))] ? 200 : 404,
    );
  if (url.pathname === '/v1/guides/concept-guide') return json(response, concept);
  if (url.pathname === '/v1/guides/onboarding') return json(response, onboarding);
  if (url.pathname === '/v1/guides/router-catalog')
    return json(response, { guides: [], troubleshooting: [] });
  if (url.pathname === '/v1/status') {
    const client = request.headers['user-agent'] ?? 'unknown';
    const count = (statusRequests.get(client) ?? 0) + 1;
    statusRequests.set(client, count);
    if (count === 1)
      return json(
        response,
        { code: 'unavailable', message: 'Reconnect pending', helpId: null },
        503,
      );
    return json(response, { activeServerId: 'survival', running: false, serverType: 'paper' });
  }
  if (url.pathname === '/v1/servers' && request.method === 'GET') return json(response, servers);
  if (url.pathname === '/v1/java-runtimes') return json(response, { runtimes: [] });
  if (url.pathname === '/v1/versions')
    return json(response, {
      flavorName: 'Paper',
      isBedrock: false,
      supportsVersions: true,
      versions: [],
    });
  if (url.pathname === '/v1/templates')
    return json(response, { paperTemplates: [], pluginTemplates: [], serverRunning: false });
  if (url.pathname === '/v1/servers/delete' && request.method === 'POST')
    return json(response, { message: 'Server record removed.' });
  if (url.pathname === '/v1/worlds')
    return json(response, { slots: worlds, activeSlotId: 'world-1', serverRunning: false });
  if (url.pathname === '/v1/staged-uploads' && request.method === 'POST')
    return json(response, {
      stagedUploadId: 'upload-1',
      uploadPath: '/v1/staged-uploads/upload-1',
      maxBytes: 1048576,
    });
  if (url.pathname === '/v1/staged-uploads/upload-1' && request.method === 'PUT')
    return json(response, { stagedUploadId: 'upload-1', receivedBytes: 4 });
  if (url.pathname === '/v1/worlds/import' && request.method === 'POST')
    return json(response, { result: 'Imported' });
  if (url.pathname === '/v1/worlds/export' && request.method === 'POST')
    return json(response, { stagedDownloadId: 'download-1' });
  if (url.pathname === '/v1/staged-downloads/download-1')
    return bytes(response, Buffer.from('PK\x03\x04'));
  const file = normalize(join(root, url.pathname === '/' ? 'index.html' : url.pathname));
  try {
    const info = await stat(file);
    if (!info.isFile()) throw new Error('not a file');
    response.writeHead(200, { 'content-type': mime[extname(file)] ?? 'application/octet-stream' });
    response.end(await readFile(file));
  } catch {
    response.writeHead(200, { 'content-type': 'text/html' });
    response.end(await readFile(join(root, 'index.html')));
  }
}).listen(port, '127.0.0.1');
