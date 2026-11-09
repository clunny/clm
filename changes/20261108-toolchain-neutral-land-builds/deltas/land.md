---
component: land
---

## ADDED Requirements
### Requirement: overlap-recheck
A `clm land` attempt whose merged tree's code key no `checked` record names with the land's command, directly or through the settings-free key (`settings-unchanged-is-checked`), SHALL check only the overlap when all of these hold: `[land] check` is set; a `checked` record with the same command names the code key of the branch's head `H`; one names the code key of the fetched base `B`; and `H` doesn't contain `B`. It SHALL run `clm affected --overlap <H> <B>` (`check.affected/overlap-selection`) in the tree on the merge commit, and then:
- when it prints nothing, run no `[land] check`, take neither the lock on `.clm/land.lock` nor the one on `.clm/check.lock`, run `clm check` with its detectors on the merge commit in a temporary worktree as an attempt with no lock does, and print `re-check: none, the branch and main since <short merge base> affect no test in common` on stderr;
- when it prints a line, go on as an attempt whose code key isn't checked, with `CLM_SELECTION` naming a file holding that output unchanged in place of `selected-tests`' selection, and print `re-check: overlap, <n> files` on stderr, `<n>` the number of its `file` lines, or `re-check: overlap, every test` when it prints an `all` line;
- when it fails, print `clm affected failed: <the first non-empty line of its stderr, or the spawn error>; the land checks its own selection` and go on as `selected-tests` says.

When `clm check` and any `[land] check` it ran pass, the land SHALL append a `checked` record for the merged tree's code key with the land's command and a `basis` naming the two code keys (`ledger/composed-check`), and its `landed` record SHALL say how it re-checked (`ledger/landed-recheck`): `recheck: none`, or `overlap` with `recheck_files` the number of `file` lines, null for an `all` line. Every other land SHALL say it too: `checked` when its code key counted as checked, and `selected` when it ran `selected-tests`' selection, with `recheck_files` counted the same way, null when every test ran. The gate on the merge commit and the push SHALL run as for any land. An attempt for which any condition fails SHALL run as before this rule. The capability tests whose lands must run a check, `crates/clm/tests/capability/land_safely.rs`'s `a_branch_that_breaks_main` and `slow-tests-after-land`'s land over its target, SHALL still observe the check they assert: this rule's build checks that their fixtures make the overlap select the test they break, or gives them a component graph that does.

#### Scenario: main moved on an unrelated component
- GIVEN components `a` and `b` with no `depends_on` between them, a `[land] check` that appends to a log outside the repository, a branch `impl/a` whose head changes only `crates/a/src/lib.rs` and has a `checked` record, and `main` moved since by a land changing only `crates/b/src/lib.rs`, whose code key has a `checked` record
- WHEN `clm land impl/a` runs while another process holds the lock on `.clm/check.lock`
- THEN it prints `landed`, without waiting, the log is empty, stderr holds `re-check: none`, and the ledger has a `checked` record for the merged tree's code key whose `basis` names both keys, and the `landed` record holds `recheck: none`

#### Scenario: a dependant of both
- GIVEN the same, and a component `c` with `depends_on: [a, b]` and a test `crates/c/tests/t.rs`, and a `[land] check` that copies the file `CLM_SELECTION` names
- WHEN `clm land impl/a` runs
- THEN the copy holds `file crates/c/tests/t.rs` and no file of `a` or `b`, stderr holds `re-check: overlap, 1 files`, and the `landed` record holds `recheck: overlap` and `recheck_files: 1`

#### Scenario: the head was never checked whole
- GIVEN the first scenario's repository with no `checked` record for the head
- WHEN `clm land impl/a` runs
- THEN the check runs with `clm affected --base`'s selection, as `selected-tests` says, and stderr holds no `re-check:` line

#### Scenario: main's head not checked
- GIVEN the first scenario's repository, with `main`'s head a commit pushed outside `clm land`, whose code key no `checked` record names
- WHEN `clm land impl/a` runs
- THEN the check runs with `clm affected --base`'s selection and stderr holds no `re-check:` line

#### Needs
- check.affected/overlap-selection
- ledger/composed-check
- ledger/landed-recheck

### Requirement: land-dir-env
Every run of `[land] check` by `clm land` that holds the lock on `.clm/land.lock` SHALL set `CLM_LAND_DIR` in the command's environment to the `.clm/land` directory of the main worktree's CLM root, the place `shared-land-target` names, replacing any value the caller's environment holds, so a project's check can keep its build output there whatever its toolchain. Every other run of `[land] check`, by a land that holds no such lock or by `clm land --check-only`, SHALL run with `CLM_LAND_DIR` removed from its environment. `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` SHALL still be set as `shared-land-target` says; a later change stops setting them (`backlog/land-still-sets-cargo-variables`).

#### Scenario: a land in the land tree
- GIVEN a `[land] check` that writes `$CLM_LAND_DIR` to a file outside the repository, a branch `impl/a` with no `checked` record, and `CLM_LAND_DIR=/elsewhere` in the caller's environment
- WHEN `clm land impl/a` runs from a linked worktree
- THEN the file holds the path of the main worktree's `.clm/land`

#### Scenario: a check-only run
- GIVEN the same `[land] check` and `CLM_LAND_DIR=/elsewhere` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds an empty line
