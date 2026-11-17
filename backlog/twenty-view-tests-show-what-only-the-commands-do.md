---
title: "Twenty of view's tests show what only the clm status and clm view commands do"
kind: deferred
components: [view, view.repo, clm]
blocks: [clm/routes-only, gate/own-files]
---

`view`'s requirements state what `clm status` and `clm view` print, and `clm-view-repo`, which lies above `clm-view`, offers the commands (`commands-live-in-their-crates`). The fix of `some-tests-prove-a-component-from-another-component-s-files` moved the 29 tests of `view` whose assertions `clm-view` itself decides, given a graph, markers and claims, into `crates/clm-view/tests/status.rs` and `view.rs`. These 20 stayed in the root's top-level test files, each citing `view`, because what they assert is decided by the command's reading of the repository, its arguments, its exit status or its second stream, none of which `clm-view` holds:

- **In `crates/clm/tests/status.rs` (11), citing `view/status-report`:**
  - `invalid_graph_prints_violations`: an invalid graph exits 1 and prints the violations in place of the report. It also cites `check.coverage/status-report`.
  - `status_and_view_never_fetch`: a claim on `origin` that the last fetch didn't bring isn't shown. It also cites `view/view-command`.
  - `scenario_oversized_node` and `scenario_budget_accepted`: which component is oversized comes from the settings' budgets, a component's `size_budget` and the sizes of its files.
  - `sizes_that_cant_be_checked_mark_nothing`: settings that don't load print `size not checked:` on standard error and the command still exits 0.
  - `status_has_no_review_section`: a ledger on `origin` is not read, and nothing goes to standard error.
  - `the_commit_is_cut_to_seven_digits_whatever_git_abbreviates_to`, `scenario_checkout_named`, `scenario_checkout_behind`, `scenario_no_origin` and `no_repository_names_no_commit`: the checkout line as read from `HEAD` and `refs/remotes/origin/main`.
- **In `crates/clm/tests/view.rs` (9):**
  - `scenario_width` (`view/view-command`): with no `--width` the chart is cut at 100 columns, the command's own default.
  - `scenario_offline` (`view/view-command`): the view is printed with an `origin` that can't be reached.
  - `scenario_unknown_view` (`view/view-command`): an unknown view, format or width exits 2 naming the choices.
  - `violations_exit_1` (`view/view-command`): an invalid graph exits 1.
  - `scenario_oversized_on_the_chart` and `sizes_that_cant_be_checked_mark_nothing` (`view/spec-view`): the size report, as above.
  - `scenario_claimed_node_and_command_matches_library` (`view/flight-view`, `view/view-library`): the command's output is compared with the library's for the same repository.
  - `scenario_checkout_behind_on_the_chart` and `scenario_checkout_current` (`view/view-command`): the checkout line, as above.

Every one of them can be shown by a call of `clm-view-repo` (`status_command`, `view_command`), with no `clm` process: only the helper that ran the binary would change.

**What settles it** is a spec change, since only a spec change moves a scenario. The human's answer of 2026-11-15 and 2026-11-17 stands for the class (`commands-live-in-their-crates`): a scenario that only another component's command can show leaves the component that can't show it, word for word, and a mixed scenario is split at the clause. There the scenarios went to the root, because the command's crate could start no `clm` process either. Here the command's crate can show all of it, so the proposal is that these clauses become `view.repo`'s: what `clm status` and `clm view` read from the repository (the checkout line's reading of `HEAD` and `origin/main`, the size report, no fetch, no ledger), their argument errors and their exit status on an invalid graph, with `view` keeping what its library prints for a checkout, a size report and claims it is given, which its own tests already prove. The tests then move into `crates/clm-view-repo/tests/` under their own names with their assertions unchanged, and `view-library`'s *command matches library* scenario becomes `view.repo`'s, since only a crate above `clm-view` can compare the two.

Due once the fixes of `some-tests-prove-a-component-from-another-component-s-files` have landed. The change that carries it out names this item in `from`.
