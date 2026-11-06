// The app under /app/ (web/app-at-app-path), its scoped install
// (web/installable) and the retirement of the old root service worker
// (web/retire-root-service-worker). The tests that need the built tree read `dist/` as the
// build left it (the Playwright web server builds it) and serve it from a small static
// server in the layout the deployed assets have, with a stand-in page for the site, which
// deploys on its own, so a root registration can exist beside the app's.
import { createServer, type Server } from 'node:http';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test, type BrowserContext } from '@playwright/test';
import { mockApi } from './mock';

const DIST = fileURLToPath(new URL('../dist/', import.meta.url));
const TYPES: Record<string, string> = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.webmanifest': 'application/manifest+json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
};

interface Site {
  origin: string;
  /** Serves this text as `/sw.js` in place of the built file. */
  oldRootWorker(text: string | null): void;
  stop(): Promise<void>;
}

/** `dist/` as the deployed assets serve it: `<dir>/` is `<dir>/index.html`. The site's pages
 * (`/` and `/guide/...`), which another Worker answers on the zone, are a stand-in page. All
 * answers carry `x-from: network`. */
async function serve(): Promise<Site> {
  let root: string | null = null;
  const server: Server = createServer((req, res) => {
    const path = decodeURIComponent(new URL(req.url ?? '/', 'http://x').pathname);
    if (path === '/sw.js' && root !== null) {
      res.writeHead(200, { 'content-type': 'text/javascript', 'x-from': 'network' }).end(root);
      return;
    }
    if (path === '/' || path.startsWith('/guide/')) {
      res.writeHead(200, { 'content-type': 'text/html', 'x-from': 'network' }).end('<!doctype html><title>site</title><p>the site');
      return;
    }
    const base = normalize(join(DIST, path));
    const candidates = [base, `${base}.html`, join(base, 'index.html')];
    const file = candidates.find((f) => f.startsWith(DIST) && existsSync(f) && statSync(f).isFile());
    if (!file) {
      // A missing path: status 404 (the fake serves the built 404 page; the deployed script answers plainly).
      const page404 = join(DIST, '404.html');
      res.writeHead(404, { 'content-type': 'text/html', 'x-from': 'network' }).end(existsSync(page404) ? readFileSync(page404) : 'not found');
      return;
    }
    res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream', 'x-from': 'network' }).end(readFileSync(file));
  });
  await new Promise<void>((done) => server.listen(0, '127.0.0.1', done));
  const address = server.address();
  if (address === null || typeof address === 'string') throw new Error('no port');
  return {
    origin: `http://localhost:${address.port}`,
    oldRootWorker: (text) => void (root = text),
    stop: () => new Promise((done) => server.close(() => done())),
  };
}

/** What the app's old service worker, at scope `/`, did: cached the shell and answered `/`
 * from it. */
const OLD_WORKER = `
const CACHE = 'clm-shell-old';
self.addEventListener('install', (e) => e.waitUntil(caches.open(CACHE).then((c) => c.put('/', new Response('old app shell', { headers: { 'content-type': 'text/html' } }))).then(() => self.skipWaiting())));
self.addEventListener('activate', (e) => e.waitUntil(self.clients.claim()));
self.addEventListener('fetch', (e) => { const u = new URL(e.request.url); if (u.pathname === '/') e.respondWith(caches.match('/')); });
`;

async function signedOut(context: BrowserContext): Promise<void> {
  await context.route('**/api/**', (route) => route.fulfill({ status: 401, contentType: 'application/json', body: '{}' }));
}

// req:web/app-at-app-path@d9141ff8
test.describe('app at /app/', () => {
  test('built tree: the app under app/, the invited list under _clm/, the retiring worker, and no site', () => {
    const app = readFileSync(join(DIST, 'app/index.html'), 'utf8');
    const scripts = [...app.matchAll(/<script[^>]*\ssrc="([^"]+)"/g)].map((m) => m[1]);
    expect(scripts.length).toBeGreaterThan(0);
    for (const src of scripts) expect(src.startsWith('/app/'), src).toBe(true);
    expect(app).toContain('id="app"');
    expect(readFileSync(join(DIST, 'sw.js'), 'utf8')).toContain('unregister');
    expect(existsSync(join(DIST, 'index.html'))).toBe(false);
    expect(existsSync(join(DIST, 'guide'))).toBe(false);
    expect(JSON.parse(readFileSync(join(DIST, '_clm/invited.json'), 'utf8'))).toEqual(['clunny']);
  });

  test("opened at /app/#/o/a/backlog it shows o/a's backlog", async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/backlog');
    await expect(page.locator('.sidebar .nv[data-view=backlog]')).toHaveClass(/\bon\b/);
  });

  test('signed out, its sign-in button leads to /auth/login', async ({ page, context }) => {
    await signedOut(context);
    await page.goto('/app/');
    await expect(page.locator('a.btn.pri')).toHaveAttribute('href', '/auth/login');
  });
});

// req:web/invited-users@cfd18219
test('the build holds the invited list under _clm/, beside the app', () => {
  expect(JSON.parse(readFileSync(join(DIST, '_clm/invited.json'), 'utf8'))).toEqual(['clunny']);
});

// req:web/installable@e711892d
test.describe('installed app scope', () => {
  test('manifest and service worker are the app only: id, start_url and scope /app/, no cache of /guide/', async ({ browser }) => {
    const site = await serve();
    const context = await browser.newContext({ serviceWorkers: 'allow' });
    try {
      const page = await context.newPage();
      await mockApi(context);
      await page.goto(`${site.origin}/app/`);
      const href = await page.locator('link[rel=manifest]').getAttribute('href');
      const manifest = await (await context.request.get(new URL(href!, `${site.origin}/app/`).href)).json();
      expect({ id: manifest.id, start_url: manifest.start_url, scope: manifest.scope }).toEqual({ id: '/app/', start_url: '/app/', scope: '/app/' });
      for (const icon of manifest.icons) {
        expect(icon.src.startsWith('/app/'), icon.src).toBe(true);
        expect((await context.request.get(site.origin + icon.src)).status(), icon.src).toBe(200);
      }
      const scopes = await page.evaluate(async () => {
        await navigator.serviceWorker.ready;
        if (!navigator.serviceWorker.controller) {
          await new Promise((r) => navigator.serviceWorker.addEventListener('controllerchange', r, { once: true }));
        }
        return (await navigator.serviceWorker.getRegistrations()).map((r) => new URL(r.scope).pathname);
      });
      expect(scopes).toEqual(['/app/']);
      // A request for the guide goes to the network, and nothing outside /app/ was cached.
      const guide = await page.evaluate(async () => {
        const res = await fetch('/guide/quickstart');
        // The service worker's caches; `clm-api` holds the app's own copies of API answers.
        const cached: string[] = [];
        for (const name of (await caches.keys()).filter((k) => k.startsWith('clm-shell-'))) {
          for (const req of await (await caches.open(name)).keys()) cached.push(new URL(req.url).pathname);
        }
        return { from: res.headers.get('x-from'), cached };
      });
      expect(guide.from).toBe('network');
      expect(guide.cached.length).toBeGreaterThan(0);
      for (const path of guide.cached) expect(path.startsWith('/app/'), path).toBe(true);
    } finally {
      await context.close();
      await site.stop();
    }
  });
});

// req:web/retire-root-service-worker@df279fd8
test.describe('the old root service worker', () => {
  const rootScopes = (page: import('@playwright/test').Page) =>
    page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).map((r) => new URL(r.scope).pathname));

  test('/sw.js unregisters the old registration and drops its caches, and / is answered by the network', async ({ browser }) => {
    const site = await serve();
    const context = await browser.newContext({ serviceWorkers: 'allow' });
    try {
      const page = await context.newPage();
      site.oldRootWorker(OLD_WORKER);
      await page.goto(`${site.origin}/guide/quickstart`);
      await page.evaluate(async () => {
        await navigator.serviceWorker.register('/sw.js', { scope: '/' });
        await navigator.serviceWorker.ready;
      });
      await page.goto(`${site.origin}/`);
      expect(await page.content()).toContain('old app shell');
      // The build's /sw.js replaces it; the browser fetches it again.
      site.oldRootWorker(null);
      await page.evaluate(async () => void (await navigator.serviceWorker.getRegistration('/'))?.update());
      // The retiring worker may reload the open page once it has activated, about a second
      // after `update()` returns. An `evaluate` that overlaps that reload throws, which
      // `expect.poll` doesn't retry and `toPass` does, and a `goto` it overlaps is
      // interrupted, so `/` is opened in a page of its own.
      await expect(async () => {
        expect(await rootScopes(page)).toEqual([]);
        expect(await page.evaluate(() => caches.keys())).toEqual([]);
      }).toPass({ timeout: 20_000 });
      const fresh = await context.newPage();
      await fresh.goto(`${site.origin}/`);
      expect(await fresh.content()).not.toContain('old app shell');
    } finally {
      await context.close();
      await site.stop();
    }
  });

  test('the app at /app/ unregisters a registration at scope / and keeps its own', async ({ browser }) => {
    const site = await serve();
    const context = await browser.newContext({ serviceWorkers: 'allow' });
    try {
      const page = await context.newPage();
      await mockApi(context);
      site.oldRootWorker(OLD_WORKER);
      await page.goto(`${site.origin}/guide/quickstart`);
      await page.evaluate(async () => {
        await navigator.serviceWorker.register('/sw.js', { scope: '/' });
        await navigator.serviceWorker.ready;
      });
      // A stand-in for the old worker is what the server answers at /sw.js; the app must not
      // wait for the browser to fetch the retiring one.
      await page.goto(`${site.origin}/app/`);
      await expect.poll(() => rootScopes(page)).toEqual(['/app/']);
    } finally {
      await context.close();
      await site.stop();
    }
  });
});
