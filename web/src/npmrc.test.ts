// @vitest-environment node
// `web/.npmrc` keeps npm from running dependencies' install scripts (specs/web/SPEC.md, Design):
// asked through npm itself, whatever the user's own npm settings say.
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const WEB = fileURLToPath(new URL('..', import.meta.url));

describe('web/.npmrc', () => {
  // req:web/no-install-scripts@9b7f85e6
  it('ends up with ignore-scripts=true', () => {
    const dir = mkdtempSync(join(tmpdir(), 'npmrc-'));
    try {
      const user = join(dir, 'npmrc');
      writeFileSync(user, 'ignore-scripts=false\n');
      // No `ignore-scripts` in the environment, in any case npm reads it.
      const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => !['npm_config_ignore_scripts', 'npm_config_ignore-scripts', 'npm_config_userconfig'].includes(k.toLowerCase())));
      env.npm_config_userconfig = user;
      const out = execFileSync('npm', ['config', 'get', 'ignore-scripts'], { cwd: WEB, env, encoding: 'utf8', timeout: 60_000 });
      expect(out.trim()).toBe('true');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
