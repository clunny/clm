// Repo text is shown on the app's origin, where the session cookie can approve PRs, so
// whatever a PR's author writes must not run as script (the Contract's Approve).
import { describe, expect, it } from 'vitest';
import { md } from './markdown';

describe('md', () => {
  it('renders Markdown', () => {
    const e = md('## Intent\n\nMake **b** better, see `x`.');
    expect(e.querySelector('h2')?.textContent).toBe('Intent');
    expect(e.querySelector('strong')?.textContent).toBe('b');
    expect(e.querySelector('code')?.textContent).toBe('x');
  });

  it('drops script, event handlers and javascript: links', () => {
    const e = md('<img src=x onerror="alert(1)"> <script>alert(2)</script> [x](javascript:alert(3))');
    expect(e.querySelector('script')).toBeNull();
    expect(e.querySelector('[onerror]')).toBeNull();
    for (const a of e.querySelectorAll('a')) expect(a.getAttribute('href') ?? '').not.toMatch(/^javascript:/i);
  });
});
