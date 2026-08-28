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
    helpId: 'handbook.overview',
    title: 'Overview',
    category: 'concepts',
    analogy: 'A server is a separate Minecraft room.',
    body: 'MSC manages servers and worlds.\n\n- Start a server\n- Choose a world',
    relatedIds: ['concept.server'],
  },
  'concept.server': {
    helpId: 'concept.server',
    title: 'One server. Your worlds.',
    category: 'concept-guide',
    body: 'A server holds worlds.',
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
const hostSetupOverrides = new Map();
let broadcastJar = { installed: false, filename: null };

function json(response, body, status = 200) {
  response.writeHead(status, { 'content-type': 'application/json' });
  response.end(JSON.stringify(body));
}
function bytes(response, body) {
  response.writeHead(200, { 'content-type': 'application/zip', 'content-length': body.byteLength });
  response.end(body);
}
async function readJsonBody(request) {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  if (chunks.length === 0) return undefined;
  try {
    return JSON.parse(Buffer.concat(chunks).toString('utf8'));
  } catch {
    return undefined;
  }
}

function hostSetupComplete(request) {
  const cookie = request.headers.cookie ?? '';
  const cookieSaysIncomplete = cookie
    .split(';')
    .some((part) => part.trim() === 'msc_test_host_setup=false');
  const client = request.headers['user-agent'] ?? 'unknown';
  return hostSetupOverrides.get(client) ?? !cookieSaysIncomplete;
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
    'content-type, authorization, x-msc-client-api-version, x-msc-csrf',
  );
  response.setHeader('access-control-allow-methods', 'GET, POST, PUT, DELETE, OPTIONS');
  if (request.method === 'OPTIONS') {
    response.writeHead(204);
    return response.end();
  }
  // BrowserSessionAuth (src/lib/auth/browser.ts) fetches this before every
  // mutation and throws if the agent never serves it -- without this route
  // every POST/PUT/DELETE in the shared browser transport fails client-side
  // before a request is even sent, matching the real agent's
  // GET /v1/auth/csrf (crates/msc-agent/src/routes/browser_session.rs).
  if (url.pathname === '/v1/auth/csrf' && request.method === 'GET')
    return json(response, {
      csrfToken: 'test-csrf-token',
      expiresAt: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
    });
  if (url.pathname === '/__test/host-setup' && request.method === 'POST') {
    const client = request.headers['user-agent'] ?? 'unknown';
    hostSetupOverrides.set(client, false);
    // /v1/status's own count===1 case simulates a reconnect-pending agent
    // for the separate "reconnect fallback" test; a fresh-profile walk
    // calling this reset first has nothing to do with that scenario and
    // must not eat a real 503 on its own first status probe, so mark this
    // client as already past it.
    statusRequests.set(client, 1);
    response.setHeader('set-cookie', 'msc_test_host_setup=false; Path=/; SameSite=Lax');
    return json(response, { complete: false });
  }
  if (url.pathname === '/v1/config/host-setup' && request.method === 'GET')
    return json(response, { complete: hostSetupComplete(request) });
  if (url.pathname === '/v1/config/host-setup/complete' && request.method === 'POST') {
    hostSetupOverrides.set(request.headers['user-agent'] ?? 'unknown', true);
    response.setHeader('set-cookie', 'msc_test_host_setup=true; Path=/; SameSite=Lax');
    return json(response, { complete: true });
  }
  if (url.pathname === '/v1/capabilities')
    return json(response, {
      helpers: { tailscale: false },
      serverTypes: { bedrock: { supported: false, backend: null } },
    });
  if (url.pathname === '/v1/me')
    return json(response, {
      permissions: ['admin', 'fleet', 'worlds', 'addons', 'settings', 'networking'],
    });
  if (url.pathname === '/v1/help/catalog')
    return json(response, {
      topics: Object.values(topics).map(({ helpId, title, category }) => ({
        helpId,
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
  if (url.pathname === '/v1/config/servers-root')
    return json(response, { path: '/Users/camerontemple/MinecraftServers' });
  if (url.pathname === '/v1/config/java-runtime') return json(response, { executablePath: '' });
  if (url.pathname === '/v1/java-runtimes')
    return json(response, {
      runtimes: [{ name: 'Java 21', executablePath: 'java', majorVersion: 21 }],
    });
  if (url.pathname === '/v1/broadcast/jar-status')
    return json(response, {
      installed: broadcastJar.installed,
      downloading: false,
      filename: broadcastJar.filename,
    });
  if (url.pathname === '/v1/broadcast/download-jar' && request.method === 'POST') {
    broadcastJar = { installed: true, filename: 'MCXboxBroadcastStandalone.jar' };
    return json(response, {
      success: true,
      message: 'downloaded',
      filename: broadcastJar.filename,
    });
  }
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
  if (url.pathname === '/v1/modpacks/inspect' && request.method === 'POST')
    return json(response, {
      success: true,
      message: 'Inspected archive.',
      format: 'mrpack',
      packName: 'Test Pack',
      packVersion: '1.0.0',
      minecraftVersion: '1.20.4',
      loaderName: 'Fabric',
      loaderVersion: '0.15.0',
      fileCount: 12,
      clientOnlyFileCount: 0,
      manualFiles: [],
      warnings: [],
    });
  if (url.pathname === '/v1/catalog/search' && request.method === 'GET') {
    const javaFlavor = url.searchParams.get('javaFlavor');
    return json(response, {
      supportsAddons: true,
      addonKind: javaFlavor === 'fabric' ? 'mod' : 'plugin',
      loaderName: javaFlavor ?? 'paper',
      gameVersion: url.searchParams.get('minecraftVersion') ?? '1.20.4',
      results: [
        {
          projectId: 'proj-1',
          slug: 'test-addon',
          title: 'Test Addon',
          description: 'A fake search result from the harness.',
          author: 'Someone',
          downloads: 4200,
          iconURL: null,
          isClientOnly: false,
          projectType: javaFlavor === 'fabric' ? 'mod' : 'plugin',
        },
      ],
    });
  }
  if (url.pathname === '/v1/servers/create' && request.method === 'POST') {
    const body = await readJsonBody(request);
    return json(response, {
      success: true,
      message: 'Server creation started.',
      operationId: 'op-server-create',
      serverName: body?.name ?? 'New Server',
    });
  }
  // Any operation id resolves succeeded on its first poll -- this harness
  // has no queued/running transitions to simulate, matching every route
  // above that hands back an operationId for pollOperation to consume.
  if (url.pathname.startsWith('/v1/operations/') && request.method === 'GET')
    return json(response, {
      id: url.pathname.slice('/v1/operations/'.length),
      type: 'test-operation',
      state: 'succeeded',
      statusLine: 'Done.',
      result: {},
    });
  if (url.pathname === '/v1/worlds/import' && request.method === 'POST') {
    const body = await readJsonBody(request);
    const imported = {
      id: 'world-2',
      name: body?.name ?? 'Imported World',
      isActive: false,
      createdAt: '2026-08-28T12:00:00Z',
      hasThumbnail: false,
    };
    return json(response, {
      success: true,
      message: 'Imported.',
      updated: { slots: [...worlds, imported], activeSlotId: 'world-1', serverRunning: false },
    });
  }
  if (url.pathname === '/v1/worlds/activate' && request.method === 'POST')
    return json(response, { result: 'Activated', operationId: 'op-world-activate' });
  if (url.pathname === '/v1/components/install' && request.method === 'POST') {
    const body = await readJsonBody(request);
    return json(response, {
      success: true,
      message: 'Installed.',
      projectId: body?.projectId ?? 'local-jar',
      operationId: 'op-component-install',
    });
  }
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
