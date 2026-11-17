---
title: "clm-check still re-exports the coverage library"
kind: deferred
components: [check, check.coverage, check.affected]
origin: 20261115-one-route-to-coverage
---

The contract step of `20261115-one-route-to-coverage` is the change `20261115-check-passes-on-no-coverage`, left `proposed` on `main`: it removes `check/path-ownership`, takes the re-export sentence out of `check/coverage-lists` and `check.coverage/state-names`, and points four requirements of `check.affected` at `check.coverage/owner`.

It waits on `backlog/twelve-crates-reach-coverage-through-clm-check-s-re-exports`: when that item is gone, no crate but `clm-check` itself names a re-exported item as `clm_check::`. It also waits on the 42 tests of `check.coverage` in `crates/clm-check/tests/` having moved to `clm-coverage`'s crate (`backlog/some-tests-prove-a-component-from-another-component-s-files`), since they call the re-exports.

Then, on a spec branch: rebase the change's deltas on `clm hashes check`, `clm hashes check.coverage` and `clm hashes check.affected`, apply it, and delete this item. The builds of `check`, `check.coverage` and `check.affected` do the rest.

**After the twelve crates' fixes (2026-11-17).** Every crate imports `clm_coverage` but one file: `crates/clm-orchestrator/tests/common/mod.rs` names `clm_check::Marker`, and the tests of `clm-triage`, `clm-run`, `clm-checkpoint` and the root include it. `clm-triage` has no `clm-coverage` dependency. So the build that removes the re-exports can't compile until that helper names the type another way or `clm-triage` takes the dependency, which is `orchestrator.triage`'s manifest: plan for it in the change. `crates/clm-check/tests/state_names.rs` holds `scenario_list_reaches_clm_check`, which tests the re-export and goes with it.
