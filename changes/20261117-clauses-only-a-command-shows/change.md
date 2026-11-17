---
status: applied
resolves: [twenty-view-tests-show-what-only-the-commands-do, six-check-coverage-tests-assert-what-other-crates-do, view-repo-s-commit-line-is-rendered-and-proven-in-view]
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, step 4 of "Phase 4, what is next", which waits on this change. Step 3 moved every test it could into the crate of the component it cites and left 26 that can't move as they stand: 20 of `view`'s and six of `check.coverage`'s, one test being both, assert what only the `clm status`, `clm view` and `clm pending` commands, or `clm-check`'s narrowing of the pending list, do, and one in `view`'s file proves a line `view.repo` states and `view`'s code draws. This change gives each such clause to the component whose crate can show it, so the tests move under their own names and `clm/routes-only` waits on nothing else.

The human agreed on 2026-11-17 that these clauses go to `view.repo` and `check`, where their standing answer of 2026-11-15 sent a clause only a `clm` process shows to the root (`commands-live-in-their-crates`). Nothing is cut: every clause is moved or split in the same words, and no test is dropped.

## Problem

Three `deferred` items hold the 26 tests. While a test in the root's file cites `view` or `check.coverage`, that component is proven from the root's files, `clm/routes-only` can't be built, and the clause "a file that cites the component" can't leave `gate/own-files`.

- **20 of `view`'s, in `crates/clm/tests/status.rs` and `view.rs`.** `view`'s requirements state, beside what the views draw, what the commands read (the checkout from `HEAD` and `origin/main`, the sizes, no fetch, no ledger), their argument errors and their exit status. `clm-view` holds none of that: `clm-view-repo`, above it, does.
- **Six of `check.coverage`'s.** Two in the root's `pending.rs` and one in its `status.rs` assert `clm pending`'s and `clm status`'s exit status; one in the root's `status.rs` asserts the `changes` section and summary line `clm-view` lays out; one in `check`'s `pending.rs` asserts `clm-check`'s refusal of a component the graph doesn't hold; one in `check`'s `state_names.rs` tests the re-export of `STATE_NAMES`.
- **One in `view`'s file** proves how the line naming a commit renders, which `view.repo/view-at-rev` states and `clm-view`'s `render` does.

Reading the tests found what the items didn't say:

- **`clm-view-repo` is handed the budgets.** It loads no settings, so a test in its files can show that a component over the budget it is passed is marked, and not that the budget is `clm.toml`'s. Only the binary shows that join.
- **`view` has no test of its own that `clm status` marks a component oversized.** Its tests cover the `spec` view's mark; the status row's `oversized` cell is proven only from the root's file.
- **`view/status-report` needs `check.coverage/status-report`**, so changing the latter here would have `check.coverage`'s build and `view`'s each wait on the other's tests.

## Approaches

Where the clauses go, put to the human on 2026-11-17:

- **To the crate that does it.** Taken. `view.repo` states what the three commands read and how they exit, `check` its own refusal, and `view` the commit line it draws. Each test then calls the crate whose code it proves, with no `clm` process.
- **To the root, as on 2026-11-15.** Not taken. That answer was given where the command's crate could start no process either; here it needs none. The root is at about 60,000 code tokens against a budget of 40,000 and would take some 20 scenarios, proven by running the binary for behaviour a crate decides.
- **Leaving them `view`'s, proven from `view`'s files through a dev-dependency on `clm-view-repo`.** Not taken: the code under test would still be another component's, which the mutation pass can't judge from `view`'s package, and phase 5's rule for a requirement is behaviour at its own component's Contract.

How `view.repo` states them:

- **Two requirements, by what the commands do.** Taken: `reads-the-repository` and `arguments-and-exit`, ten and five scenarios. Phase 5 regroups them with the rest phase 4 added.
- **One requirement per source requirement.** Not taken: five where two say it.
- **Folding them into `view.repo/commands-in-crate`.** Not taken: that requirement states the shape of the call, and `clm/routes-only` needs it as built.

The budgets:

- **One scenario of `clm/routes-only`, proven by the binary.** Taken, beside "the settings a routed command is passed", which shows the same join for `clm affected`. The root's build writes its test; `view.repo`'s moved tests pass the budget in.
- **A dev-dependency of `clm-view-repo` on `clm-config`.** Not taken: `view.repo` doesn't depend on `config`, and its Contract says the budgets are passed to it.

The commit line:

- **`view/view-command` states it; `view.repo/view-at-rev` names `view` for the form.** Taken, as the item proposed. It costs `view.repo` a second, small build, since `view-at-rev` needs `view/view-command`.
- **Leaving `view-at-rev`'s text and stating the form a second time in `view`.** Not taken: two texts for one line drift apart.

## Outcome

**`view.repo` gains two requirements.**

- `reads-the-repository`, ten scenarios: the working tree and the local refs only, never a fetch, an `origin` that can't be reached, no ledger; the checkout read from `HEAD` and `refs/remotes/origin/main`, cut to 7 digits, none without a commit; the size report under the budgets it is passed, and `size not checked:` on standard error when the settings don't load. "checkout line", "oversized node", "offline", "checkout behind on the chart" and "oversized on the chart" are `view`'s scenarios word for word; the other five state what a test asserted and no scenario did.
- `arguments-and-exit`, five scenarios: the default width, the usage errors, exit 1 with the violations for an invalid graph from all three commands, `clm pending`'s refusal of a component the graph doesn't hold, and "command matches library", which only a crate above `clm-view` can show. "unknown view" and "command matches library" are `view`'s word for word.
- `view-at-rev` says what the command reads for the line naming the commit and names `view/view-command` for its form. `pending-names-its-commit` takes its exit status from `arguments-and-exit`. Their scenarios are unchanged.

**`view` keeps what its library draws for what it is handed.** `status-report`, `view-command`, `spec-view` and `view-library` say "handed" where they said read; four scenarios are restated on a checkout or a size report given to the library; `view-command` states the line naming a commit, with a scenario; `view-library` says no test of `view`'s is left in the root's `status.rs` or `view.rs`. So `view`'s build writes a test of the status row's `oversized` cell, re-cites `the_commit_line_heads_every_chart_at_a_commit` where it stands, moves `scenario_groups_in_order` into its own files citing `view/status-report`, and takes its other tests out of the root's two files, which then hold none. Each needs `view.repo`'s two requirements, so the tests are in `view.repo`'s files before they leave the root's.

**`check/pending-narrows-on-a-graph`** states the violation a component the graph doesn't hold gets, a removed one included, with a scenario; `a_removed_node_is_an_unknown_node` is re-cited to it where it stands.

**`check.coverage/pending-report`** no longer states an exit status, and says none of its tests is left in the root's `pending.rs` or in `check`'s files. It needs `view.repo/arguments-and-exit` and `check/pending-narrows-on-a-graph`.

**`clm/routes-only`** takes the scenario "the budgets a routed command is passed" and needs the requirements above, so it is built when the root's top-level files hold only the root's tests. `gate/own-files` needs it already.

`moved-tests.txt` lists the 26 tests with where each goes.

**Builds, in the order the needs give:** `view.repo` (its two requirements) and `check`; `check.coverage`; `view`; `view.repo` again, to re-cite `view-at-rev` and `pending-names-its-commit`; then the root's `clm/routes-only`, which is step 4.

## What it leaves

- **`check.coverage/status-report` still says `clm status` exits 0 or 1.** Its crate can't show that, and `view.repo/arguments-and-exit` now states it. The sentence stays for phase 5, because `view/status-report` needs that requirement.
- **`scenario_list_reaches_clm_check`** stays in `check`'s `state_names.rs`, citing `check.coverage/state-names`. It tests the re-export, which goes with `20261115-check-passes-on-no-coverage`; `backlog/clm-check-still-re-exports-the-coverage-library` says what that change must do for it.
- **`land.approval`'s 25 tests** (`land-approval-s-rules-for-clm-land-are-proven-only-by-landing`) go with step 5's designs, as the plan says.

## Constraints

- No test is deleted without a test of the same name in the file it moved to; `scenario_oversized_node` and its neighbours keep their assertions and are handed the budget they wrote to `clm.toml`.
- A scenario that names `clm <command>` is not reworded where the crate's call proves it (the human, 2026-11-15).
