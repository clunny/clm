---
status: applied
resolves: [path-ownership-names-no-exception-for-a-split-child-s-build]
---

## Intent

Name the split move as an exception in `path-ownership`, and have the build prompt state it.

## Problem

When a change carves a child out of a component (`nested-splits`), the child's build moves the carved-out code and tests out of the parent's `paths`, rewires the parent's imports and entry module, and re-points the parent's markers of the moved requirements to their new IDs. `path-ownership` says a build changes only its own `paths`, with four exceptions, none of which is this move; only the decision `nested-splits` sanctions it, and no requirement of the build prompt states it. The architecture audit flags every split change against `path-ownership` (it did on `20261112-web-on-the-move`), and a builder reading the constraint or the prompt alone would refuse the move.

## Approaches

- *Accept it as established practice.* Every split so far was built this way because its change's Design told the build to, but the constraint stays contradicted on every split and each split's Design must say it again. Rejected.
- *A fifth exception in the constraint's text only.* Fixes what the audit reads, but the constraint's other exceptions are each stated in the build prompt and listed under `#### Enforced by`; one stated only in `ARCHITECTURE.md` reaches no build (`human-layer-proof`). Rejected.
- *Chosen: a fifth exception in `path-ownership`, stated in the build prompt by a new requirement `orchestrator.run/split-move-fix`*, which the constraint lists under `#### Enforced by`, as `template-migration-fix` and `capability-tests-follow-rule` are listed for theirs (`reuse-first`: same shape, same `## Fixes in other components` section).

## Outcome

- `ARCHITECTURE.md`'s `path-ownership` (hand edit) reads "Five exceptions", adds the split move (moving the carved-out code and tests into the moving component's `paths`, deleting the other's copy, rewiring the other component's imports, entry module and wiring only to reach the moved code, re-pointing the moved requirement's markers), a scenario `split moves carved-out code`, and `orchestrator.run/split-move-fix` under `#### Enforced by`.
- `orchestrator.run/split-move-fix`: the prompt text, in every prompt, beside the other three exception texts. Its build adds the paragraph to `crates/clm-run/src/lib.rs` and a test in `crates/clm-run/tests/prompt.rs` citing it.
- `nested-splits` gains one sentence pointing at the exception, and `path-ownership-by-prompt` an amendment naming it.

## Constraints

- The move is stated generally (a requirement moved from another component), not only parent to child, since `moved-markers-wait` covers any move and the build that re-points the markers is always the move's target.
- The rewiring is limited to reaching the moved code where it now lives; behaviour the other component keeps doesn't change, as for the forced fixes.

## Edge cases and workarounds

None.

## Settled by the builder check and the audit

- The builder check found nothing the requirement relies on that isn't current, no code work the outcome promises beyond `split-move-fix`, and no capability test that sees a difference.
- The audit found that "moves the code and tests" contradicted `tests-count-where-cited`, which keeps the CLI's integration tests in `crates/clm/tests`; the exception, its scenario and the requirement now move only the tests that can run from the moving component's `paths` and leave the rest in place, re-cited.
- The audit asked for an amendment note on `path-ownership-by-prompt`, as each earlier exception has; added.
