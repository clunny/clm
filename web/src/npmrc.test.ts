// @vitest-environment node
// clm:reads web/.npmrc
// `web/.npmrc` keeps npm from running dependencies' install scripts (specs/web/SPEC.md, Design):
// asked through npm itself, whatever the user's own npm settings say. The rule is that file's
// data, which no mutation tool changes, so the marker above names it.
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
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
      // No `ignore-scripts` in the environment, in any case npm reads it. The user file outranks
      // npm's global and built-in settings, so it alone decides wherever a project sets nothing.
      const env = Object.fromEntries(
        Object.entries(process.env).filter(
          ([k]) => !['npm_config_ignore_scripts', 'npm_config_ignore-scripts', 'npm_config_userconfig'].includes(k.toLowerCase()),
        ),
      );
      env.npm_config_userconfig = user;
      const get = (cwd: string) =>
        execFileSync('npm', ['config', 'get', 'ignore-scripts'], { cwd, env, encoding: 'utf8', timeout: 60_000 }).trim();
      // The control: a directory with no settings of its own takes the user's `false`, so the
      // `true` below comes from `web/.npmrc` and not from a user file npm never read.
      const bare = join(dir, 'bare');
      mkdirSync(bare);
      writeFileSync(join(bare, 'package.json'), '{}\n');
      expect(get(bare)).toBe('false');
      expect(get(WEB)).toBe('true');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
