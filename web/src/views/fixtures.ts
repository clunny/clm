// Fixture responses for the unit and browser tests. Illustrative data, built here rather
// than read from the repository's specs (tests don't read doc-only paths).

import type { Architecture, BlockersView, BranchReview, Capabilities, Constraint, Cost, FlightView, Implementation, NodeView, Snapshot, SpecBranch, SpecNode, SpecView } from './types';

export function node(id: string, over: Partial<NodeView> = {}): NodeView {
  return {
    id,
    title: id.toUpperCase(),
    status: 'active',
    parent: 'root',
    depends_on: [],
    contract: `The ${id} contract.`,
    requirements: [],
    assets: {},
    ...over,
  };
}

/** The `spec` model's entry for `n`, counting its requirements' states. */
export function specNode(n: NodeView, owners: string[] = []): SpecNode {
  return {
    node: n.id,
    parent: n.parent,
    title: n.title,
    status: n.status,
    owners,
    depends_on: n.depends_on,
    current: n.requirements.filter((r) => r.state === 'current').length,
    behind: n.requirements.filter((r) => r.state === 'behind').length,
    unbuilt: n.requirements.filter((r) => r.state === 'unbuilt').length,
    obsolete: 0,
    total: n.requirements.length,
  };
}

export function spec(nodes: NodeView[], owners: Record<string, string[]> = {}): SpecView {
  return { view: 'spec', nodes: nodes.map((n) => specNode(n, owners[n.id] ?? [])) };
}

export function snapshot(over: Partial<Snapshot> = {}): Snapshot {
  const nodes = [
    node('root', { parent: null, title: 'Root' }),
    node('a', { requirements: [{ id: 'a/x', text: 'A SHALL x.', version: 'aaaa0000', state: 'current', markers: ['t.rs:1'] }] }),
    node('b', {
      status: 'unbuilt',
      depends_on: ['a'],
      requirements: [
        { id: 'b/y', text: 'B SHALL y.', version: 'bbbb0000', state: 'unbuilt', markers: [] },
        {
          id: 'b/a-requirement-with-a-really-long-identifier-that-should-wrap-on-phones',
          text: 'B SHALL wrap.',
          version: 'bbbb1111',
          state: 'unbuilt',
          markers: [],
        },
      ],
    }),
  ];
  return {
    repo: 'o/a',
    sha: '222',
    coverage: { state: 'complete' },
    spec: spec(nodes),
    nodes,
    changes: [
      {
        id: 'c',
        status: 'applied',
        from: null,
        resolves: [],
        nodes: ['a', 'b'],
        current: 1,
        total: 2,
        complete: false,
      },
    ],
    backlog: [
      { slug: 'crash', kind: 'bug', title: 'It crashes', nodes: ['a'], origin: null, blocks: [] },
      { slug: 'later', kind: 'idea', title: 'Do it later', nodes: ['b'], origin: null, blocks: [] },
      { slug: 'setup', kind: 'human-task', title: 'Create the account', nodes: ['a'], origin: 'c', blocks: ['c/a'] },
    ],
    decisions: [],
    violations: [],
    ...over,
  };
}

/** A snapshot with `check` and `check.drift` inside it, `check.drift` owned by `@a`, and
 * `gate` depending on `check.drift`. */
export function nestedSnapshot(): Snapshot {
  const nodes = [
    node('clm', { parent: null, title: 'CLM' }),
    node('check', { parent: 'clm', requirements: [{ id: 'check/run', text: 'It SHALL run.', version: '11110000', state: 'current', markers: ['t.rs:1'] }] }),
    node('check.drift', { parent: 'check', status: 'unbuilt' }),
    node('gate', { parent: 'clm', depends_on: ['check.drift'] }),
  ];
  return snapshot({ nodes, spec: spec(nodes, { 'check.drift': ['@a'] }), changes: [], backlog: [] });
}

/** `spec/c` with page #7 "Change b", and `spec/d` with none. */
export const branches: SpecBranch[] = [
  { branch: 'spec/c', head_sha: 'h7', page: { number: 7, url: 'https://github.com/o/a/pull/7', title: 'Change b' } },
  { branch: 'spec/d', head_sha: 'h8', page: null },
];

export const SPEC_BEFORE = `---
title: N
status: active
---

# N

### Contract

The contract.

### Design

The design, first paragraph.

The design, second paragraph.

Third.

Fourth.

Fifth.

Sixth.

### Requirements

### Requirement: r
It SHALL do r.

### Requirement: s
It SHALL do s.
`;

export const SPEC_AFTER = SPEC_BEFORE.replace(
  'The design, first paragraph.',
  'The design, first paragraph, now changed.',
).replace('It SHALL do s.', 'It SHALL do s twice.');

/** `spec/c`'s review: fingerprint `f1`, no approvals, page #7. */
export function review(over: Partial<BranchReview> = {}): BranchReview {
  return {
    branch: 'spec/c',
    base: 'main',
    base_sha: 'b1',
    head_sha: 'h7',
    review: true,
    fingerprint: 'f1',
    approvals: [],
    signed: false,
    checked: true,
    page: { number: 7, url: 'https://github.com/o/a/pull/7' },
    ...body(),
    ...over,
  };
}

function body() {
  return {
    change: {
      id: 'c',
      status: 'applied',
      from: null,
      resolves: ['crash'],
      intent: '## Intent\n\nMake b better.\n\n## Other\n\n- r changes.',
      intent_section: 'Make b better.',
    },
    nodes: [
      {
        id: 'n',
        status: 'modified',
        frontmatter: null,
        contract: null,
        design: null,
        requirements: {
          added: [],
          modified: [{ id: 'n/r', before: 'It SHALL do r.', after: 'It SHALL do r quickly.' }],
          removed: [],
        },
        assets: {
          added: [{ path: 'm.html', before: null, after: '<p>mock</p>' }],
          modified: [],
          removed: [],
        },
      },
    ],
    decisions: [{ name: 'd', status: 'added', before: null, after: '# D\n\nWe decided.' }],
    backlog: [],
    documents: [{ path: 'specs/n/SPEC.md', before: SPEC_BEFORE, after: SPEC_AFTER }],
  };
}

export const HOUR = 3600;
export const NOW = 1_800_000_000;

/** `a` blocked by the `setup` item, `b` claimed an hour ago and lapsing in 3 hours. */
export function flight(over: Partial<FlightView> = {}): FlightView {
  return {
    view: 'flight',
    totals: { pending_nodes: 2, by_state: { blocked: 1, claimed: 1 }, live_changes: 1, requirements_pending: 2 },
    nodes: [
      {
        node: 'b',
        priority: 'P1',
        state: 'claimed',
        pending: [{ id: 'b/y', version: 'bbbb0000', state: 'unbuilt', cited: [], markers: [], priority: 'P1', blocked_by: [], waits_on: [] }],
        claim: { lapses_at: NOW + 3 * HOUR, pushed_hours_ago: 1, lapses_in_hours: 3 },
        blocked_by: [],
      },
      {
        node: 'a',
        priority: 'P5',
        state: 'blocked',
        pending: [{ id: 'a/x', version: 'aaaa0000', state: 'behind', cited: ['aaaa9999'], markers: ['t.rs:1'], priority: 'P5', blocked_by: ['setup'], waits_on: [] }],
        claim: null,
        blocked_by: [{ item: 'backlog/setup', requirements: ['a/x'] }],
      },
    ],
    changes: [{ status: 'applied', changes: [{ change: 'c', priority: 'P1', current: 1, total: 2, obsolete: [], complete: false }] }],
    ...over,
  };
}

export function blockers(over: Partial<BlockersView> = {}): BlockersView {
  return {
    view: 'blockers',
    roots: [
      { root: 'backlog/setup', kind: 'backlog', state: 'human-task', waits_on_human: true, weight: 1, holds: [{ node: 'a', via: [] }] },
    ],
    ...over,
  };
}

export function implementation(over: Partial<Implementation> = {}): Implementation {
  return {
    sha: '222',
    now: NOW,
    expiry_hours: 4,
    coverage: { state: 'complete' },
    flight: flight(),
    blockers: blockers(),
    pages: {},
    activity: [],
    taken: { changes: [], claims: [] },
    ...over,
  };
}

const spend = (dollars: number) => ({ dollars, work_tokens: dollars * 100_000, cache_read_tokens: dollars * 1_000_000, active_seconds: dollars * 600 });

/** One capability's entry in the Worker's report: no history, so its numbers are null. */
export function cost(slug: string, dollars: number, over: Partial<Cost> = {}): Cost {
  return {
    id: `capability/${slug}`,
    slug,
    version: `${slug.charCodeAt(0).toString(16)}000000`.slice(0, 8),
    statement: `A person can ${slug}.`,
    why: `Without ${slug}, nothing works.`,
    rules: [`n/${slug}-one`, `n/${slug}-two`],
    nodes: ['n'],
    spend: spend(dollars),
    changes: 1,
    approvals: 2,
    flags_settled: 0,
    burdens: [],
    commits: null,
    questions_answered: null,
    blocking: null,
    size_tokens: null,
    flags: [],
    overlap: { only: 1, shared: [[`capability/other`, 1]] },
    decisions: [],
    flagged: false,
    ...over,
  };
}

/** The capabilities `b` ($9) then `a` ($2), `a` flagged by `pipes` on `n/a-one`. */
export function capabilities(over: Partial<Capabilities> = {}): Capabilities {
  return {
    report: {
      capabilities: [
        cost('b', 9, { decisions: [{ at: '2026-10-29T10:00:00Z', writer: 'alice/web', value: 'high', decision: 'keep' }] }),
        cost('a', 2, {
          flagged: true,
          flags: [{ slug: 'pipes', requirement: 'n/a-one', title: 'Judge ignores pipes', text: 'Builds pipe output.', settled: false, added: null }],
        }),
      ],
      unattributed: spend(1),
      architecture_only: spend(0.5),
    },
    signed: false,
    ...over,
  };
}

/** One architectural constraint's entry in the Worker's report. */
export function constraint(slug: string, dollars: number, over: Partial<Constraint> = {}): Constraint {
  return {
    id: `architecture/${slug}`,
    slug,
    version: `${slug.charCodeAt(0).toString(16)}000000`.slice(0, 8),
    statement: `CLM SHALL ${slug}.`,
    enforced_by: [],
    untested: null,
    rules: [],
    nodes: [],
    spend: spend(dollars),
    changes: 1,
    approvals: 1,
    flags_settled: 0,
    burdens: [],
    flags: [],
    overlap: { only: 0, shared: [] },
    decisions: [],
    flagged: false,
    ...over,
  };
}

/** The architecture report: `e` ($5, flagged by `misfires`, enforced by `a/x` current and
 * `a/y` pending, partly untested) then `g` ($0, `#### Untested` only). */
export function architecture(over: Partial<Architecture> = {}): Architecture {
  return {
    architecture: [
      constraint('e', 5, {
        enforced_by: [
          { rule: 'a/x', current: true },
          { rule: 'a/y', current: false },
        ],
        untested: 'Partly judgement.',
        rules: ['a/x', 'a/y'],
        nodes: ['a'],
        overlap: { only: 1, shared: [['architecture/other', 1]] },
        flagged: true,
        flags: [{ slug: 'misfires', requirement: 'a/x', title: 'Gate misfires', text: 'The gate misfires on renames.', settled: false, added: null }],
      }),
      constraint('g', 0, { untested: 'A judgement.' }),
    ],
    signed: false,
    indexing: false,
    ...over,
  };
}
