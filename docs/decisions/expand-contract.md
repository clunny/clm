---
title: "Breaking contract changes expand, then contract, with the old path marked for removal"
date: 2026-10-22
status: accepted
---

Every land must leave `main` green, a build changes only its own component's files (`path-ownership`), and branches never stack (`no-stacked-prs`). So a contract change that another component's code or tests use can't land in one step: the provider's branch fails the user's tests, and the user's branch can't land first. It lands in steps instead, each built in the files of the component it changes (`coupled-changes-sequence`):

1. **Expand.** The provider adds the new contract alongside the old one. Every land stays green.
2. **Move.** Each user moves to the new contract in a requirement of its own, which needs the expansion.
3. **Contract.** The provider removes the old contract and its code, once every move is built.

An addition a user must handle counts as such a change: a variant it matches exhaustively, or a field it builds by struct literal. No build edits another component to absorb a removal or an addition; there is no exception for the compile-only and test-fixture fixes a change forces elsewhere. The exceptions `path-ownership` keeps (shared wiring, a capability test that follows its rule, a template and its migration) are named in `coupled-changes-sequence`.

**One change or two.** When the steps are requirements of different components, one change carries them all: the removing requirement holds a `#### Needs` on each user's requirement that moves off the old contract, and the queue orders the builds (`contract-removal-needs`). Two changes remain for a removal no requirement's `Needs` can carry, such as a whole requirement removed or a required component waiting on an optional one. Changes declare no order (`node-reconciliation`), and an applied change's text is built at once, so the contract change then stays `proposed`, unapplied, until the requirements that move users to the new contract are current, and a later spec branch applies it. Its `change.md` names those requirements, and the branch that proposes it also adds a `kind: deferred` backlog item naming it and the requirements, at their versions, that it waits on, so `clm triage` shows the step until the human judges it due (`contract-step-deferred`).

**Markers.** The old path is marked in code with `clm:remove-by <change-id>`, naming the change that removes it. `clm check` fails when a marker names a change that isn't live: missing, `done`, `abandoned` or archived (`check/remove-by-markers`). So:

- with two changes, the expand change's spec branch must also propose the contract change, or its markers fail at once. Both go in one spec branch, which costs at most one human review;
- the removing change can't finish while any of its markers remain, so the removal can't be skipped;
- abandoning it fails the markers, forcing a decision about the old path.

The specs describe both contracts while both exist, since both are current (`spec-is-current`). The removing delta takes the old one out.

Rejected:
- **One branch spanning the provider and its users.** It needs a build that works in several components, which breaks the scoped context a build reads (`spine-scoped-context`) and `path-ownership`.
- **Dated deprecations or TODO comments.** A date says nothing about whether the removal is planned, and a TODO isn't checked.
- **Letting the provider's build make the compile-only and test-fixture fixes its change forces in other components.** It was allowed for a time, one exception per kind of tie, and each new kind stalled a build again; the extra builds of a sequence are machine time, not the human's (`coupled-changes-sequence`).

## The rule in full

`ARCHITECTURE.md`'s entry `expand-contract` states this in short; the rule in full:

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
