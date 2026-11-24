import { describe, expect, it, vi } from 'vitest';
import { renderArchitecture, renderConstraint } from './architecture';
import { architecture } from './fixtures';

const list = (flaggedOnly = false) => ({ flaggedOnly, onFilter: vi.fn(), link: (s: string) => `#/o/a/architecture/${s}` });
const page = { offline: false, busy: false, value: 'medium' as const, onValue: vi.fn(), onDecide: vi.fn(), back: '#/o/a/architecture', note: null };

// req:web.views/says-architectural-constraint@c93b2f7f
describe('says architectural constraint', () => {
  it('an empty report says so', () => {
    const view = renderArchitecture(architecture({ architecture: [] }), list());
    expect(view.querySelector('.empty')!.textContent).toBe('No architectural constraints: ARCHITECTURE.md holds none.');
  });

  it('none flagged, and an unknown slug, say so', () => {
    const none = architecture();
    none.architecture.forEach((c) => (c.flagged = false));
    expect(renderArchitecture(none, list(true)).querySelector('.empty')!.textContent).toBe('No architectural constraint is flagged.');
    expect(renderConstraint(architecture(), 'zz', page).querySelector('.empty')!.textContent).toBe('The report holds no architectural constraint zz.');
  });
});
