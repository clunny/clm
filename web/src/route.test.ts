import { describe, expect, it } from 'vitest';
import { href, parseRoute, reviewHref, reviewPath } from './route';

describe('routes', () => {
  it('reads repo, view, review, tab and full-document path', () => {
    expect(parseRoute('#/o/a/backlog')).toEqual({ owner: 'o', repo: 'a', view: 'backlog' });
    expect(parseRoute('#/o/a/prs')).toEqual({ owner: 'o', repo: 'a', view: 'prs' });
    expect(parseRoute('#/o/a/prs/spec%2Fx/graph')).toMatchObject({ view: 'prs', review: { branch: 'spec/x' }, prTab: 'graph' });
    expect(parseRoute('#/o/a/prs/spec%2Fx/full/specs/n/SPEC.md')).toMatchObject({ review: { branch: 'spec/x' }, prTab: 'document', full: 'specs/n/SPEC.md' });
    expect(parseRoute('#/o/a/prs/land/abc').review).toBeUndefined();
    expect(parseRoute('')).toEqual({ owner: undefined, repo: undefined, view: 'graph' });
  });

  // req:web/spec-branch-review@a7d7ac89
  it('routes a branch by its name as one segment, and asks the Worker with its slashes', () => {
    const at = reviewHref('o', 'a', { branch: 'spec/x/y' }, 'full', 'specs/n/SPEC.md');
    expect(at).toBe('#/o/a/prs/spec%2Fx%2Fy/full/specs/n/SPEC.md');
    expect(parseRoute(at)).toMatchObject({ review: { branch: 'spec/x/y' }, full: 'specs/n/SPEC.md' });
    expect(parseRoute(reviewHref('o', 'a', { branch: 'spec/x' }, 'graph'))).toMatchObject({ review: { branch: 'spec/x' }, prTab: 'graph' });
    expect(reviewPath({ branch: 'spec/x/y' })).toBe('branches/spec/x/y');
  });

  it('round-trips through href', () => {
    expect(parseRoute(reviewHref('o', 'a b', { branch: 'spec/c' }, 'full', 'specs/x y/SPEC.md'))).toMatchObject({
      repo: 'a b',
      full: 'specs/x y/SPEC.md',
    });
  });

  // req:web/capabilities-list@fde4e68b
  it('reads the capabilities list and a capability page, and round-trips them', () => {
    expect(parseRoute('#/o/a/capabilities')).toEqual({ owner: 'o', repo: 'a', view: 'capabilities' });
    expect(parseRoute('#/o/a/capabilities/review-capability-cost')).toEqual({ owner: 'o', repo: 'a', view: 'capabilities', capability: 'review-capability-cost' });
    expect(parseRoute(href('o', 'a', 'capabilities', 'x'))).toMatchObject({ view: 'capabilities', capability: 'x' });
  });

  // req:web/architecture-list@d644a9d4
  it('reads the architecture list and an entry page, a view of its own, and round-trips them', () => {
    expect(parseRoute('#/o/a/architecture')).toEqual({ owner: 'o', repo: 'a', view: 'architecture' });
    expect(parseRoute('#/o/a/architecture/git-is-truth')).toEqual({ owner: 'o', repo: 'a', view: 'architecture', entry: 'git-is-truth' });
    expect(parseRoute(href('o', 'a', 'architecture', 'e'))).toMatchObject({ view: 'architecture', entry: 'e' });
    expect(parseRoute('#/o/a/capabilities/e').entry).toBeUndefined();
  });

  it("doesn't throw on a malformed escape", () => {
    expect(parseRoute('#/o/%E0/graph')).toMatchObject({ owner: 'o', repo: '%E0', view: 'graph' });
  });
});
