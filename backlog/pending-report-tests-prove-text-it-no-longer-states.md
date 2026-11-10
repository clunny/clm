---
title: "Two pending-report tests prove behaviour the requirement no longer states"
kind: bug
components: [check.coverage]
---

The spec auditor found, after `check.coverage/pending-report` lost the sentence "This holds for every obsolete marker whose component is missing, however the component went.", that:

1. `a_node_gone_without_a_removal_delta_moves_too` (`crates/clm-check/tests/pending.rs`) proves markers of a missing component without a `remove: true` delta move to the owning component, and the code does it (`pending.rs`, `by_owner`), but no text states it. Either restore the rule in the requirement or drop the test and the general branch.
2. `paths_match_as_git_glob_pathspecs_do` (same file) checks git `:(glob)` semantics, which only `check/path-ownership` states, and the test cites `pending-report` instead.

Needs a spec change.
