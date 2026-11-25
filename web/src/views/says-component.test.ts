import { describe, expect, it, vi } from 'vitest';
import { renderConstraint } from './architecture';
import { renderBacklog } from './backlog';
import { renderCapabilities, renderCapability } from './capabilities';
import { renderImplementation } from './implementation';
import { renderReview } from './pr';
import { NOW, architecture, capabilities, implementation, review, snapshot } from './fixtures';

/** The old words `component-words` retires, standing alone: no letter, digit or `_` beside them, and not joined by `-`, `/`, `.` or `:` to more of an ID, a path or a key. */
const OLD = /(?<![A-Za-z0-9_\-/])(?<![A-Za-z0-9_][.:])(nodes?|spines?|stubs?)(?![A-Za-z0-9_\-/])(?![.:][A-Za-z0-9_])/gi;
/** Every text a view shows, element by element. */
const shown = (view: HTMLElement) => [...view.querySelectorAll('*')].map((e) => e.textContent).join(' ');

describe('says component', () => {
  // req:web.views/says-component@3ee2caa8
  it('the Implementation view shows Components and Pending components, and no old word', () => {
    const view = renderImplementation(implementation(), snapshot(), { now: NOW, show: 'flight', onShow: () => {} });
    const text = shown(view);
    expect(text).toContain('Components');
    expect(text).toContain('Pending components');
    expect(text.match(OLD)).toBeNull();
  });

  // req:web.views/says-component@3ee2caa8
  it('the other views: the Backlog, the reviews, the Capabilities list and the capability and constraint pages say component, and no old word', () => {
    const backlog = renderBacklog(snapshot(), null, null, () => {});
    expect(backlog.querySelector('.filter .note')!.textContent).toBe('Component');
    expect(backlog.querySelector('.head .meta')!.textContent).toBe('3 items across 2 components');

    const opts = { offline: false, accepted: false, onAct: vi.fn(), fullHref: (p: string) => `#full/${p}`, onComment: vi.fn(async () => null), onRequest: vi.fn(async () => null) };
    const two = review();
    two.nodes = [two.nodes[0], { ...two.nodes[0], id: 'm' }];
    const touching = renderReview(two, opts);
    const kv = [...touching.querySelectorAll('.aside .kv')].find((k) => k.textContent?.startsWith('Components touched'))!;
    expect(kv.querySelector('b')!.textContent).toBe('2');
    const empty = review();
    empty.nodes = [];
    const none = renderReview(empty, opts);
    expect(none.querySelector('.doc .note')!.textContent).toBe('No component changes.');

    const list = renderCapabilities(capabilities(), { flaggedOnly: false, onFilter: vi.fn(), link: (s) => s });
    const page = { offline: false, busy: false, value: null, onValue: vi.fn(), onDecide: vi.fn(), back: '#', note: null };
    const capability = renderCapability(capabilities(), 'a', page);
    const constraint = renderConstraint(architecture(), 'e', page);
    const split = /a component's cost is split by its rules' share/;
    expect(list.querySelector('.head .meta')!.textContent).toMatch(split);
    expect(capability.querySelector('section[data-sec=cost]')!.textContent).toMatch(split);
    expect(constraint.querySelector('section[data-sec=cost]')!.textContent).toMatch(split);

    for (const view of [backlog, touching, none, list, capability, constraint]) expect(shown(view).match(OLD)).toBeNull();
  });
});
