// Repo text is shown on the app's origin, where the session cookie can approve PRs, so
// whatever a PR's author writes must not run as script (the Contract's Approve).
import { describe, expect, it } from 'vitest';
import { md } from './markdown';

describe('md', () => {
  // req:web.views/repo-markdown@342a93bb
  it('renders Markdown', () => {
    const e = md('## Intent\n\nMake **b** better, see `x`.');
    expect(e.querySelector('h2')?.textContent).toBe('Intent');
    expect(e.querySelector('strong')?.textContent).toBe('b');
    expect(e.querySelector('code')?.textContent).toBe('x');
  });

  // req:web.views/repo-markdown@342a93bb
  it('drops script, event handlers and javascript: links', () => {
    const e = md('<img src=x onerror="alert(1)"> <script>alert(2)</script> [x](javascript:alert(3))');
    expect(e.querySelector('script')).toBeNull();
    expect(e.querySelector('[onerror]')).toBeNull();
    for (const a of e.querySelectorAll('a')) expect(a.getAttribute('href') ?? '').not.toMatch(/^javascript:/i);
  });

  // req:web.views/repo-markdown@342a93bb
  it('what sanitising drops: bold stays bold, and no script element, onclick attribute or javascript: link is left', () => {
    const e = md('**bold** <script>alert(1)</script> [x](javascript:alert(1)) <a href="https://e" onclick="alert(1)">e</a>');
    expect(e.querySelector('strong')?.textContent).toBe('bold');
    expect(e.querySelector('script')).toBeNull();
    expect(e.querySelector('[onclick]')).toBeNull();
    expect(e.innerHTML).not.toMatch(/javascript:/i);
    expect(e.querySelector('a[href="https://e"]')?.textContent).toBe('e');
  });
});
