---
title: "A git glob pathspec test cites check.coverage/pending-report instead of the rule it proves"
kind: bug
components: [check]
---

`paths_match_as_git_glob_pathspecs_do` (`crates/clm-check/tests/pending.rs`) checks git `:(glob)` semantics, which only `check/path-ownership` states, but it cites `check.coverage/pending-report`. Re-cite it to `check/path-ownership` at its current version. The test lies in `check`'s `paths`, so a `check.coverage` build may re-cite it only to its own requirements (`path-ownership`); fix it with `clm claim check --fix backlog/pending-report-tests-prove-text-it-no-longer-states`.

The item's other half, that `pending-report` didn't state how markers of a component gone without a `remove: true` delta are counted, is fixed by change `20261109-slim-flagged-specs`.
