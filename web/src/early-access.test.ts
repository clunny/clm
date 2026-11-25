// @vitest-environment node
// The form the sign-in page's Request early access opens: only GitHub renders it, so it is
// proven by reading the template (specs/web/SPEC.md, sign-in-early-access).
// clm:reads .github/ISSUE_TEMPLATE/early-access.yml
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

const TEMPLATE = fileURLToPath(new URL('../../.github/ISSUE_TEMPLATE/early-access.yml', import.meta.url));

interface Field {
  validations?: { required?: boolean };
}
interface Form {
  name: string;
  description: string;
  labels: string[];
  body: Field[];
}

describe('the early-access issue form', () => {
  // req:web/sign-in-early-access@c4497420
  it('the form: one required question, early access, works fully without it, and no email', () => {
    const text = readFileSync(TEMPLATE, 'utf8');
    const form = parse(text) as Form;
    expect(form.name).toBe('Request early access');
    expect(form.labels).toContain('early-access');
    for (const words of ['invite-only', 'early access', 'works fully without it', 'https://clm-app.dev/guide/quickstart']) expect(form.description).toContain(words);
    expect(form.body.filter((f) => f.validations?.required === true)).toHaveLength(1);
    expect(text.toLowerCase()).not.toContain('email');
  });
});
