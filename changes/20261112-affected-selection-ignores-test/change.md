---
status: applied
resolves: [affected-selection-ignores-test-depends-on]
---

## Intent

Finish what `20261112-components-own-their-tests` started for `clm affected`. That change already rewrote `check.affected/affected-components` so a component whose `test_depends_on` names an affected component is affected too, which is the selection the backlog item asks for, and that requirement is `behind` until `check.affected` is built. Left as written, the Contract and Design still say dependants are found by `depends_on` alone, and `why-chain` can't explain a file selected through `test_depends_on`, since its chain walks `depends_on` edges only. This change brings those three in line, so the build that catches `affected-components` up changes `select.rs` and `why.rs` together.

## Problem

`backlog/affected-selection-ignores-test-depends-on`: `clm context` now prints the Contract of each `test_depends_on` component, so a change to that Contract changes the dependent's bundle, but `clm affected` (`select.rs`, `why.rs`) selects dependants by `depends_on` only, so the dependent's tests may not run. The selection rule is already in the spec (`affected-components`, modified by `20261112-components-own-their-tests`). What remains:

- The Contract's *affected* bullet and the Design's walk name only `depends_on`, contradicting `affected-components`.
- `why-chain` prints the shortest `depends_on` path to a changed file's owner. For a file selected only because its component's `test_depends_on` names an affected component, no such path exists, so the rule as written gives no chain.

## Approaches

- **Leave the item to the pending build of `affected-components`.** That fixes the selection but leaves `--why` with no rule for these files, and `clm land`'s slow-step report (`land/check-time-report`) runs it on exactly such files. Rejected.
- **Chosen: name `test_depends_on` in the Contract and Design, and give `why-chain` a `<x> tests against <y>` link.** A `test_depends_on` edge is taken only as the chain's first edge, and only when no `depends_on` path reaches an owner of a changed file, matching the selection: a component brought in by its tests is not itself a dependant, so the walk doesn't go on from it.

## Outcome

- `check.affected`'s Contract and Design: the affected components include each component whose `test_depends_on` names an owner of a changed file or one of its dependants, not transitively.
- `check.affected/why-chain`: the `tests against` link and a scenario for it. Its build and `affected-components`' build are the same component's, so one build catches both up.

No edge cases or workarounds are known. `land`'s check-time report prints `why`'s lines as they come, so the new link needs nothing of `land`.

## Open questions

None.
