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

## The rule in full

`ARCHITECTURE.md` stated this as the entry `expand-contract` until 2026-11-13, when the entry was cut to its rule (`human-layer-entries-stay-short`). The text still holds:

A contract change that another component's code or tests use SHALL land in steps, so that every land leaves `main` green and each step is built in the files of the component it changes (`coupled-changes-sequence`): the provider expands, offering the new beside the old; each user moves in a requirement of its own that needs the expansion; and the provider contracts, removing the old once every move is built. The old path is marked `clm:remove-by <change-id>`. An addition a user must handle counts: a variant it matches exhaustively, which it first meets with a default arm marked `clm:remove-by` the change and removes when it handles the variant, or a field it builds by struct literal, which it first builds through a constructor or defaults.

**Scenario: renamed command**
- GIVEN a change renaming a command other components call
- WHEN it lands
- THEN the old name still works, marked for removal by a named change

**Scenario: added variant**
- GIVEN a change adding a variant to an enum that component `b` matches exhaustively
- WHEN it is built
- THEN `b`'s build first adds a default arm marked `clm:remove-by` the change, the provider's build then adds the variant, and `b`'s next build handles it and removes the arm, each land leaving `main` green

**Guarantee**
Every land leaves `main` green while the components that use a contract haven't moved off it yet: CLM builds one component at a time and never stacks branches, so no change lands everywhere at once.

**Untested**
Whether a contract change breaks a caller, and so needs two steps, is a judgement; the markers it leaves are checked.
