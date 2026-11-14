---
title: "Test fixtures hold their markers in bare comments, outside any test"
kind: deferred
components: [check, check.coverage, check.health, sync, rename, orchestrator, verify, gate, clm, capability]
origin: 20261114-proof-is-real
blocks: [check.coverage/markers-in-tests]
---

`check.coverage/markers-in-tests` makes a marker count only inside a test. 118 tests in ten components build a fixture repository whose marker sits in a bare comment of a file with no test in it (`// req:a/x` in `t.rs`), and assert that it counts. They would fail the day the rule is built, in components its build may not edit, so this item blocks the rule until they are mended.

The work: in each fixture, write the marker inside a test (for a Rust fixture, a `#[test]` function around the comment; for a TypeScript one, an `it(...)` call). That is valid today, changes nothing a test asserts and no requirement, so it is done on one code branch, before this repository sets `wiring`, that resolves this item with the trailer `Resolves: backlog/test-fixtures-hold-markers-outside-any-test` (`deferred-work-resolves-in-code`). A fixture whose point is a marker that must not count, or a dangling one, keeps its marker where it is.

Found on 2026-11-14 by running the whole Rust suite against a prototype of the rule on `9f2746dd`; 4,013 of 4,131 tests passed. The vitest and Playwright suites were not run against it. The tests that failed, by test binary:

- `clm-check::coverage` (13): `bare_and_versioned_markers_both_count_for_dangling`, `capability_removed_by_a_live_change_is_not_dangling`, `dangling_marker_reports_file_and_line`, `marker_dangles_again_once_the_removing_change_is_archived`, `marker_on_architecture_or_untested_entry_reported`, `marker_on_missing_entry_dangling`, `markers_in_carries_versions`, `only_files_git_would_track_are_scanned`, `report_carries_the_state_and_markers`, `scenario_mixed_coverage`, `scenario_obsolete_marker`, `scenario_same_as_checkout`, `scenario_stale_marker_beside_a_current_one`
- `clm-check::ownership` (1): `scan_stays_public`
- `clm-check::pending` (16): `a_behind_need_still_blocks`, `a_behind_requirement_names_a_repeated_need_once`, `a_node_gone_without_a_removal_delta_moves_too`, `a_removed_node_is_an_unknown_node`, `a_top_level_node_is_deeper_than_the_root`, `an_obsolete_marker_whose_node_exists_stays_on_that_node`, `behind_lists_each_distinct_cited_version_and_none`, `paths_match_as_git_glob_pathspecs_do`, `scenario_need_satisfied`, `scenario_nothing_pending`, `scenario_removed_node`, `scenario_removed_node_files_of_two_nodes`, `scenario_removed_node_unowned_file`, `scenario_stale_marker_in_another_components_files`, `scenario_states`, `the_deepest_owner_then_the_first_by_id_takes_a_removed_nodes_marker`
- `clm-check::size` (9): `a_marked_source_part_and_an_unmarked_test_part`, `a_removed_requirement_still_counts`, `a_test_citing_two_nodes_counts_for_both`, `a_test_counts_where_it_is_cited`, `inline_tests_count_where_they_are_cited`, `markers_naming_no_node_count_nowhere`, `only_an_exact_cfg_test_line_splits_a_rust_file`, `size_of_a_listing_matches_the_working_tree`, `size_of_a_listing_with_inline_tests_matches_the_working_tree`
- `clm-check::skipped_tests` (2): `ignored_with_a_reason`, `skipped_python_tests`
- `clm-check::status` (6): `an_unapplied_change_is_never_complete`, `rendered_groups_are_indented`, `scenario_complete_change`, `scenario_one_pending_change`, `scenario_removed_requirement_still_cited`, `scenario_requirement_removed_later`
- `clm-check::tests_cite_their_owner` (4): `a_capability_is_the_capability_components`, `another_components_requirement`, `exempt_until_split`, `run_listed_reports_it_too`
- `clm-coverage::marker_spans` (1): `scenario_the_same_markers_as_markers_in`
- `clm-gate::outside` (1): `a_rename_branch`
- `clm-health::churn` (4): `a_path_git_quotes_is_skipped`, `paths_with_spaces_and_a_users_prefixes`, `scenario_re_cited_across_commits_and_a_rename`, `scenario_what_doesnt_count`
- `clm-health::report` (1): `model_violations_are_findings_under_check`
- `clm-health::state` (1): `scenario_stalled_change`
- `clm-orchestrator::orchestrator` (5): `scenario_dependency_doesnt_order_then_converged`, `scenario_fix_on_a_converged_node`, `scenario_full_cap_empty_queue`, `scenario_same_as_next_on_a_repo`, `scenario_why_it_isnt_ready`
- `clm-rename::rename` (9): `a_behind_marker_stays_behind`, `a_held_back_marker_still_takes_the_new_id`, `a_reworded_requirement_keeps_its_marker_current`, `a_term_left_in_the_code_does_not_hold_the_marker_back`, `a_term_spelt_like_a_rewritten_slug_does_not_hold_the_marker_back`, `another_components_code_naming_it_does_not_hold_the_marker_back`, `current_and_behind_markers_follow_a_rename`, `done_and_archived`, `other_requirements_markers_and_bare_markers_are_handled`
- `clm-sync::sync` (11): `a_committed_fault_is_judged_despite_an_uncommitted_fix`, `a_reconsider_item_is_filed_after_archiving_and_proposals`, `a_repo_without_a_backlog_archives_too`, `activation_rides_the_archive_commit`, `an_untracked_backlog_item_is_left_alone`, `branch_completes_a_change`, `completed_by_another_branch_s_merge`, `node_activated`, `spec_changed_is_reported_with_the_check_result`, `the_result_is_judged_by_check`, `the_settings_error_comes_before_the_check`
- `clm-verify::due` (1): `queue_drained_waits_for_the_last_pending_requirement`
- `clm::affected` (1): `selects_through_the_markers_of_the_files_the_scan_reads`
- `clm::capability` (2): `build_from_spec::drain_the_queue`, `build_from_spec::resume_in_a_new_session`
- `clm::checkpoint` (1): `scenario_complete_change`
- `clm::cli` (1): `coverage_command_prints_report`
- `clm::health` (1): `scenario_findings_dont_fail_it`
- `clm::pending` (4): `scenario_need_satisfied`, `scenario_nothing_pending`, `scenario_removed_node_from_the_cli`, `scenario_states`
- `clm::proposals` (1): `claim_fix_prints_the_branch_then_is_refused`
- `clm::root` (2): `from_a_subdirectory`, `inside_a_larger_repository`
- `clm::run` (3): `scenario_behind_and_obsolete`, `scenario_resumed_branch_behind_main`, `scenario_resumed_branch_that_conflicts_with_main`
- `clm::size` (1): `a_test_counts_toward_the_node_it_cites`
- `clm::status` (5): `scenario_changes_grouped`, `scenario_complete_change`, `scenario_groups_in_order`, `scenario_nothing_in_flight`, `scenario_summary`
- `clm::sync` (1): `sync_leaves_no_temporary_worktree`
- `clm::view` (5): `scenario_a_removed_nodes_markers`, `scenario_change_progress`, `scenario_nested_nodes`, `scenario_removed_requirement_still_cited`, `scenario_requirement_removed_later`
- `clm::view_at` (4): `pending_behind_origin_main`, `pending_up_to_date`, `scenario_ahead_of_the_checkout`, `scenario_header`
- `clm::view_queue` (1): `no_loop_yet_at_a_commit_reads_that_commit`
