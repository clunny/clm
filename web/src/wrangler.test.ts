// @vitest-environment node
// `web/wrangler.toml` binds `web.live`'s Durable Object class (specs/web/SPEC.md, Deploy),
// read with wrangler's own config reader so the test sees what the deploy sees.
import { fileURLToPath } from 'node:url';
import { experimental_readRawConfig } from 'wrangler';
import { describe, expect, it } from 'vitest';

const WRANGLER = fileURLToPath(new URL('../wrangler.toml', import.meta.url));

describe('wrangler.toml', () => {
  const { rawConfig } = experimental_readRawConfig({ config: WRANGLER });

  it('binds the Live class as LIVE in a SQLite migration (req:web/live-binding@a60502c6)', () => {
    expect(rawConfig.durable_objects?.bindings).toContainEqual(
      expect.objectContaining({ name: 'LIVE', class_name: 'Live' }),
    );
    expect(rawConfig.migrations).toContainEqual(
      expect.objectContaining({ tag: 'v1', new_sqlite_classes: expect.arrayContaining(['Live']) }),
    );
  });

  it('binds the Replica class as REPLICA, declared in a SQLite migration after the Live one (req:web/replica-binding@69721fd8)', () => {
    expect(rawConfig.durable_objects?.bindings).toContainEqual(
      expect.objectContaining({ name: 'REPLICA', class_name: 'Replica' }),
    );
    const migrations: { new_sqlite_classes?: string[] }[] = rawConfig.migrations ?? [];
    const live = migrations.findIndex((m) => m.new_sqlite_classes?.includes('Live'));
    const replica = migrations.findIndex((m) => m.new_sqlite_classes?.includes('Replica'));
    expect(live).toBeGreaterThanOrEqual(0);
    expect(replica).toBeGreaterThan(live);
  });

  it('keeps an empty crons list, so a deploy clears the Cron Trigger (req:web/no-cron-trigger@b75cdebf)', () => {
    expect(rawConfig.triggers?.crons).toEqual([]);
  });

  // How Workers assets read `run_worker_first`: a path goes to the Worker when it matches a
  // pattern and no `!` pattern, `*` standing for any run of characters.
  const toWorker = (path: string): boolean => {
    const rules = rawConfig.assets?.run_worker_first;
    if (!Array.isArray(rules)) throw new Error('run_worker_first is not a list');
    const matches = (p: string) => new RegExp(`^${p.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*')}$`).test(path);
    return rules.some((r) => !r.startsWith('!') && matches(r)) && !rules.some((r) => r.startsWith('!') && matches(r.slice(1)));
  };

  it('sends only the API, sign-in, webhook, invited list and /app to the Worker (req:web/site-served-directly@f8994877)', () => {
    for (const path of ['/api/repos', '/auth/login', '/webhook', '/_clm/invited.json', '/app']) {
      expect(toWorker(path), path).toBe(true);
    }
    for (const path of ['/', '/guide/quickstart', '/404.html', '/app/', '/app/main.js', '/sw.js']) {
      expect(toWorker(path), path).toBe(false);
    }
    expect(rawConfig.assets?.run_worker_first).toEqual(['/api/*', '/auth/*', '/webhook', '/_clm/*', '/app']);
    expect(rawConfig.assets?.not_found_handling).toBe('404-page');
  });
});
