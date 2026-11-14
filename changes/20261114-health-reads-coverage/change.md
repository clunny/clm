---
status: applied
resolves: [health-churn-cannot-reach-marker-spans]
---

## Intent

`check.health` depends on `check.coverage`, so its build can call `marker_spans`, which `check.health/churn-signal` reads markers with since `20261114-proof-is-real`. This answers the question that build filed, and unblocks the last four requirements of phase 3 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`.

## Problem

`marker_spans` is `clm-coverage`'s, stated in `check.coverage`'s Contract. `check.health` depended on `check` alone, and `clm-check` re-exports no `marker_spans`, so `crates/clm-health` couldn't call it without an import `undeclared-imports` refuses. The build of `churn-signal` stopped and asked which edit should let it.

## Approaches

- **Taken: add `check.coverage` to `check.health`'s `depends_on`.** `depends_on` states what a component's code imports and decides which Contracts its build is given (`ancestor-scoped-context`). `check.health` calls a function of `check.coverage`, so saying so is the fact, and its build then reads the Contract that describes the function. It is one frontmatter line and no code outside `check.health`. `rename` reaches `marker_spans` the same way.
- **Not taken: have `clm-check` re-export `marker_spans`**, as `check-coverage-split` has it re-export the rest. That needs a requirement of `check` whose only content is to pass a function through, and a build of `check`, and it leaves `check.health`'s build reading `check`'s Contract, which doesn't describe what it calls. The decision chose re-exports to spare seven components an edit when coverage was carved out; that is a reason to leave their calls alone, not to route a new one through the parent.

## Outcome

`check.health`'s frontmatter names `check.coverage`. `check-coverage-split` says which route a new call takes. The question item is deleted, so `check.health/churn-signal` is no longer blocked, and `check.coverage/markers-in-tests`, `check/coverage-lists` and `upgrade/migration-markers-in-tests` follow it. The change adds, modifies and removes no requirement, so it is archived on its own branch.

`crates/clm-health` then imports `clm-coverage` for `marker_spans` and still reaches the calls it already uses through `clm-check`. Moving every consumer onto `check.coverage` directly would end the re-exports, and belongs with the plan's later phases.

## Constraints

No behaviour changes. Test selection is unchanged: `check` already depends on `check.coverage`, so a change there already selected `check.health`'s tests.
