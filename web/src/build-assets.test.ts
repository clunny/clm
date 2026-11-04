// @vitest-environment node
// req:web/invited-users@cfd18219
import { mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { buildAssets, invitedLogins } from '../build-assets';

const SPEC = new URL('../../specs/web/SPEC.md', import.meta.url);

describe('invited logins', () => {
  it('list from the spec: exactly the logins in the Contract', () => {
    const spec = readFileSync(SPEC, 'utf8');
    // A separate, simpler parse: every `- \`login\`` line in the Contract section.
    const contract = spec.split('\n## Contract\n')[1].split('\n## ')[0];
    const simple = [...contract.matchAll(/^- `([^`]+)`$/gm)].map((m) => m[1]);
    expect(invitedLogins(spec)).toEqual(simple);
    expect(invitedLogins(spec)).toEqual(['clunny']);
  });

  it('takes only the bullets of the Invited users paragraph', () => {
    const spec = '# W\n\n## Contract\n\n- `not-me`\n\n**Invited users** (`x`). Only these.\n\n- `a`\n- `b-2`\n\n**Other** text.\n\n- `c`\n\n## Design\n\n- `d`\n';
    expect(invitedLogins(spec)).toEqual(['a', 'b-2']);
  });

  it('reads a checkout with CRLF line endings', () => {
    const spec = '## Contract\r\n\r\n**Invited users** (`x`).\r\n\r\n- `a`\r\n- `b`\r\n\r\n## Design\r\n';
    expect(invitedLogins(spec)).toEqual(['a', 'b']);
  });

  it('fails when the list is empty', () => {
    expect(() => invitedLogins('## Contract\n\n**Invited users**. None.\n\n## Design\n')).toThrow(/invited/i);
  });
});

describe('the build plugin', () => {
  it('writes the invited list to _clm/ beside the app output, and nothing else', () => {
    const dist = mkdtempSync(join(tmpdir(), 'clm-dist-'));
    try {
      const plugin = buildAssets(new URL('../../', import.meta.url));
      (plugin.writeBundle as unknown as (o: { dir: string }) => void)({ dir: join(dist, 'app') });
      expect(readdirSync(dist)).toEqual(['_clm']);
      expect(readdirSync(join(dist, '_clm'))).toEqual(['invited.json']);
      expect(JSON.parse(readFileSync(join(dist, '_clm/invited.json'), 'utf8'))).toEqual(['clunny']);
    } finally {
      rmSync(dist, { recursive: true, force: true });
    }
  });
});
