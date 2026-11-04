// @vitest-environment node
// `web/README.md` carries the GitHub App's manifest, which must be the one in
// `web.worker`'s Design; this test holds a copy of it (no test reads `specs/` for it).
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const README = new URL('../README.md', import.meta.url);

/** The App's manifest, as `specs/web/worker/SPEC.md`'s Design gives it. */
const MANIFEST = {
  name: 'CLM Spec Review',
  url: 'https://clm-app.dev',
  hook_attributes: { url: 'https://clm-app.dev/webhook', active: true },
  callback_urls: ['https://clm-app.dev/auth/callback'],
  public: true,
  default_permissions: { contents: 'write', pull_requests: 'read', metadata: 'read' },
  default_events: ['push', 'pull_request'],
};

describe('README', () => {
  it("carries the App's manifest", () => {
    const readme = readFileSync(README, 'utf8');
    const block = readme.match(/```json\n([\s\S]*?)```/);
    expect(block).not.toBeNull();
    expect(JSON.parse(block![1])).toEqual(MANIFEST);
  });
});
