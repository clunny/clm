---
title: "Six of check.coverage's tests assert what other crates do"
kind: deferred
components: [check.coverage, check, view, view.repo, clm]
blocks: [clm/routes-only, gate/own-files]
---

The fix of `some-tests-prove-a-component-from-another-component-s-files` moved 47 tests of `check.coverage` into `crates/clm-coverage/tests/`: 40 from `check`'s crate (`status.rs` 14, `pending.rs` 19, `coverage.rs` 7), which now call `clm-coverage` on a checkout's graph and scanned markers where they called `clm-check`'s wrapper of the same, and 7 from the root's files (`pending_lines.rs` 6, `coverage_lines.rs` 1), which assert the lines the commands print on what the library returns. These six still cite `check.coverage` from a file that isn't its own, unchanged, because what they assert is not `clm-coverage`'s code:

- **In the root's `crates/clm/tests/pending.rs`**, citing `check.coverage/pending-report`:
  - `violations_and_unknown_nodes_exit_1`: `clm pending` exits 1 for an unknown component and for an invalid graph.
  - `scenario_removed_node_from_the_cli`: its last assertion is that `clm pending` exits 1 for a removed component. Its other assertions are also made by `scenario_removed_node` and its neighbours in `crates/clm-coverage/tests/pending.rs`.
- **In the root's `crates/clm/tests/status.rs`**, citing `check.coverage/status-report`:
  - `scenario_groups_in_order`: it asserts `clm status`'s text, the groups and the summary line `changes 2 live: ...`, which `clm-view` prints. `check.coverage`'s own test of the scenario, of the same name, is in `crates/clm-coverage/tests/status.rs`.
  - `invalid_graph_prints_violations`: an invalid graph exits 1 and prints the violations. It also cites `view/status-report` and is on the list of `twenty-view-tests-show-what-only-the-commands-do`.
- **In `check`'s `crates/clm-check/tests/pending.rs`**: `a_removed_node_is_an_unknown_node`, which asserts that narrowing the list to a component the graph doesn't hold is refused, with a violation whose rule is `check.coverage/pending-report`. That refusal is `clm-check`'s `pending_for`; `clm-coverage` returns every component's list.
- **In `check`'s `crates/clm-check/tests/state_names.rs`**: `scenario_list_reaches_clm_check`, which asserts that the list of state names is reachable as `clm_check::STATE_NAMES`. It goes with the re-exports, which `clm-check-still-re-exports-the-coverage-library` removes.

`crates/clm-check/tests/status.rs` is left as an empty test target: every test in it moved, and its entry in `crates/clm-check/Cargo.toml` is `check`'s to remove with the file.

**What settles it** is a spec change, by the human's answers for the class (`commands-live-in-their-crates`): a clause only another component's code can show leaves the component that can't show it. So the proposal is that narrowing the pending list to a component, and refusing one the graph doesn't hold, is stated by `check`, whose crate does it; that the exit status of `clm pending` and `clm status` on an unknown component or an invalid graph is stated by `view.repo`, which offers the commands; and that `scenario_groups_in_order` is re-cited to `view/status-report` and moves to `crates/clm-view/tests/status.rs`, since it proves `view`'s text. It can be one change with `twenty-view-tests-show-what-only-the-commands-do`. The change that carries it out names this item in `from`.
