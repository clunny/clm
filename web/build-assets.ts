// Build-time assets read from the repository, so the app holds what the commit it was
// built from says (web/invited-users). They go under `dist/_clm/`, beside the app's `dist/app/`
// and outside it, which `wrangler.toml` sends to the Worker first: only the Worker reads them.

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import type { Plugin } from 'vite';

/** The logins listed under `**Invited users**` in the Contract of `specs/web/SPEC.md`. */
export function invitedLogins(spec: string): string[] {
  const contract = spec.split(/^## Contract$/m)[1]?.split(/^## /m)[0] ?? '';
  const para = contract.split(/^\*\*Invited users\*\*/m)[1]?.split(/^\*\*/m)[0] ?? '';
  const logins = [...para.matchAll(/^- `([^`]+)`\s*$/gm)].map((m) => m[1]);
  if (logins.length === 0) throw new Error('specs/web/SPEC.md: the Contract lists no invited users');
  return logins;
}

/** Writes `_clm/invited.json` in the directory above the app's output (`web/dist/`). */
export function buildAssets(root: URL): Plugin {
  return {
    name: 'clm-build-assets',
    apply: 'build',
    writeBundle(options) {
      const invited = invitedLogins(readFileSync(new URL('specs/web/SPEC.md', root), 'utf8'));
      const dir = join(dirname(options.dir ?? 'dist/app'), '_clm');
      mkdirSync(dir, { recursive: true });
      writeFileSync(join(dir, 'invited.json'), JSON.stringify(invited));
    },
  };
}
