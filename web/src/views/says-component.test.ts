import { describe, expect, it } from 'vitest';
import { renderImplementation } from './implementation';
import { NOW, implementation, snapshot } from './fixtures';

/** The old words `component-words` retires, standing alone: no letter, digit or `_` beside them, and not joined by `-`, `/`, `.` or `:` to more of an ID, a path or a key. */
const OLD = /(?<![A-Za-z0-9_\-/])(?<![A-Za-z0-9_][.:])(nodes?|spines?|stubs?)(?![A-Za-z0-9_\-/])(?![.:][A-Za-z0-9_])/gi;

// req:web.views/says-component@88d3bff9
describe('says component', () => {
  it('the Implementation view shows Components and Pending components, and no old word', () => {
    const view = renderImplementation(implementation(), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
    const text = [...view.querySelectorAll('*')].map((e) => e.textContent).join(' ');
    expect(text).toContain('Components');
    expect(text).toContain('Pending components');
    expect(text.match(OLD)).toBeNull();
  });
});
