---
title: "Breaking contract changes expand, then contract, with the old path marked for removal"
date: 2026-10-22
status: accepted
---

Every PR must leave `main` green, tasks are one per node, and PRs never stack (`no-stacked-prs`). So a contract change that breaks a dependent node can't land in one step: the provider's PR fails the dependent's tests, and the dependent's PR can't land first. CLM does it in two changes instead:

1. **Expand.** The provider adds the new contract alongside the old one, and dependents move to the new one. Every PR stays green.
2. **Contract.** A second change removes the old contract and its code. Changes declare no order (`node-reconciliation`), and an applied change's text is built at once, so the contract change stays `proposed`, unapplied, until the requirements that move dependents to the new contract are current; a later spec PR then applies it. Its `change.md` names those requirements. (Amended 2026-10-24, `20261024-hash-in-worktrees`: this step used to order the change `after` the expand change, which `change-order` allowed until `node-reconciliation` superseded it.)

The old path is marked in code with `clm:remove-by <change-id>`, naming the contract change. `clm check` fails when a marker names a change that isn't live: missing, `done`, `abandoned` or archived (`check/remove-by-markers`). So:

- the expand change's spec PR must also propose the contract change, or its markers fail at once. Both go in one spec PR, which costs one human review;
- the contract change can't finish while any of its markers remain, so the removal can't be skipped;
- abandoning the contract change fails the markers, forcing a decision about the old path.

The specs describe both contracts while both exist, since both are current (`spec-is-current`). The contract change's delta removes the old one.

(Amended 2026-10-26, `20261026-contract-removal-across-nodes`: when the provider and its dependants are different nodes, one change can carry both halves, the removing requirement holding a `#### Needs` on the dependants' requirements that move off the old contract (`contract-removal-needs`). The two changes above remain for a removal no requirement can carry, and a build never edits a dependant to absorb a removal. Amended 2026-10-29: except for the compile-only and test-fixture fixes the removal forces, named in its page (`path-ownership`, `contract-removal-needs`).)

(Amended 2026-11-01, `20261101-a-contract-step-from`: the branch that proposes a contract change left unapplied also adds a `kind: deferred` backlog item naming it and the requirements, at their versions, that it waits on, so `clm triage` shows the step until the human judges it due (`contract-step-deferred`).)

Rejected:
- **One PR spanning the provider and its dependents.** It needs multi-node tasks, which break spine-scoped planning (`spine-scoped-context`).
- **Dated deprecations or TODO comments.** A date says nothing about whether the removal is planned, and a TODO isn't checked.

(Amended 2026-11-12, `20261112-components-own-their-tests`: the exception for the fixes a removal forces is withdrawn. Every contract change another component's code or tests use, additions it must handle included, lands as expand, each user's move, then contract, each built in its own component's files (`coupled-changes-sequence`).)
