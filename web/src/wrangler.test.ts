// @vitest-environment node
// The app's Worker as `wrangler` reads its configuration in `web/` (specs/web/SPEC.md,
// Deploy): wrangler finds the file and reads it itself, so the test sees what a deploy run
// there sees.
import { fileURLToPath } from 'node:url';
import { experimental_readRawConfig, unstable_convertConfigBindingsToStartWorkerBindings, unstable_readConfig } from 'wrangler';
import { describe, expect, it } from 'vitest';

/** `web/`, where the deploy runs `wrangler`. */
const WEB = fileURLToPath(new URL('..', import.meta.url));

/** `read` run in `web/`, as a deploy runs `wrangler` there, and then back where it was. */
function inWeb<T>(read: () => T): T {
  const was = process.cwd();
  process.chdir(WEB);
  try {
    return read();
  } finally {
    process.chdir(was);
  }
}

describe('wrangler.toml', () => {
  // The file wrangler finds from `web/`, as it is written and as wrangler reads it for a deploy.
  const { rawConfig } = inWeb(() => experimental_readRawConfig({}));
  const config = inWeb(() => unstable_readConfig({}, {}));

  // The bindings a deploy creates, as wrangler converts them from the configuration.
  const bindings = unstable_convertConfigBindingsToStartWorkerBindings(config) ?? {};
  const migrations: { new_sqlite_classes?: string[] }[] = rawConfig.migrations ?? [];
  const migrationOf = (cls: string) => migrations.findIndex((m) => m.new_sqlite_classes?.includes(cls));

  // req:web/worker-deploy@4d72cb08
  it('binds the Live class as LIVE in a SQLite migration', () => {
    expect(bindings.LIVE).toEqual({ type: 'durable_object_namespace', class_name: 'Live' });
    expect(migrationOf('Live')).toBeGreaterThanOrEqual(0);
  });

  // req:web/worker-deploy@4d72cb08
  it('binds the Replica class as REPLICA, declared in a SQLite migration after the Live one', () => {
    expect(bindings.REPLICA).toEqual({ type: 'durable_object_namespace', class_name: 'Replica' });
    expect(migrationOf('Replica')).toBeGreaterThan(migrationOf('Live'));
  });

  // req:web/worker-deploy@4d72cb08
  it('keeps an empty crons list, so a deploy clears the Cron Trigger', () => {
    expect(rawConfig.triggers?.crons).toEqual([]);
  });

  // req:web/worker-deploy@4d72cb08
  it('binds nothing but LIVE, REPLICA and ASSETS, and no KV namespace', () => {
    expect(Object.keys(bindings).sort()).toEqual(['ASSETS', 'LIVE', 'REPLICA']);
    expect(bindings.ASSETS).toEqual({ type: 'assets' });
    expect(rawConfig.kv_namespaces ?? []).toEqual([]);
  });

  // How Workers assets read `run_worker_first`: a path goes to the Worker when it matches a
  // pattern and no `!` pattern, `*` standing for any run of characters.
  const toWorker = (path: string): boolean => {
    const rules = config.assets?.run_worker_first;
    if (!Array.isArray(rules)) throw new Error('run_worker_first is not a list');
    const matches = (p: string) => new RegExp(`^${p.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*')}$`).test(path);
    return rules.some((r) => !r.startsWith('!') && matches(r)) && !rules.some((r) => r.startsWith('!') && matches(r.slice(1)));
  };

  it('finds the configuration in web/ (req:web/site-served-directly@7019d7fa)', () => {
    expect(config.configPath).toBe(`${WEB}wrangler.toml`);
  });

  it('sends only the API, sign-in, webhook, invited list and /app to the script (req:web/site-served-directly@7019d7fa)', () => {
    for (const path of ['/api/repos', '/auth/login', '/webhook', '/_clm/invited.json', '/app']) {
      expect(toWorker(path), path).toBe(true);
    }
    for (const path of ['/app/', '/app/main.js', '/sw.js']) {
      expect(toWorker(path), path).toBe(false);
    }
    expect(config.assets?.run_worker_first).toEqual(['/api/*', '/auth/*', '/webhook', '/_clm/*', '/app']);
    expect(config.assets?.not_found_handling).toBe('none');
  });

  it('deploys the assets in ./dist as the Worker clm (req:web/site-served-directly@7019d7fa)', () => {
    expect(config.assets?.directory).toBe('./dist');
    expect(config.name).toBe('clm');
  });

  it('binds the built files as ASSETS and sends the invited list to the script (req:web/invited-users@97e590dd)', () => {
    expect(config.assets?.binding).toBe('ASSETS');
    expect(toWorker('/_clm/invited.json')).toBe(true);
  });

  const routes = (config.routes ?? []) as { pattern: string; zone_name?: string; custom_domain?: boolean }[];
  // A route pattern matches a request URL's host and path, `*` standing for any run of characters.
  const routed = (path: string): number =>
    routes.filter((r) => new RegExp(`^${r.pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*')}$`).test(`clm-app.dev${path}`)).length;

  it('routes exactly the app\'s seven paths on clm-app.dev, with no Custom Domain (req:web/site-served-directly@7019d7fa)', () => {
    expect(routes.map((r) => r.pattern).sort()).toEqual(
      ['/app', '/app/*', '/api/*', '/auth/*', '/webhook', '/_clm/*', '/sw.js'].map((p) => `clm-app.dev${p}`).sort(),
    );
    for (const r of routes) {
      expect(r.zone_name).toBe('clm-app.dev');
      expect(r.custom_domain ?? false).toBe(false);
    }
  });

  it('leaves the site\'s paths to the site\'s Worker (req:web/site-served-directly@7019d7fa)', () => {
    for (const path of ['/', '/guide/quickstart', '/blog/', '/blog/feed.xml', '/privacy', '/404.html']) {
      expect(routed(path), path).toBe(0);
    }
    for (const path of ['/app/', '/api/repos', '/auth/login', '/webhook', '/_clm/invited.json', '/sw.js']) {
      expect(routed(path), path).toBe(1);
    }
  });
});
