---
title: "A failing land check leaves its output at target/land-logs/<branch>.log and repeats its last lines"
date: 2026-10-28
status: accepted
---

`clm land` copies `[land] check`'s stdout and stderr, through one pipe so their order holds, to its own stderr and to `<main>/target/land-logs/<branch>.log`. When the check fails it prints the last 40 lines of that output again under a line naming the check, and the refusal names the log, which stays; when the check passes the log is removed. A log that can't be created is a warning, and the check runs with its output to stderr alone.

**Why.** Three builds in one session saw `clm land` fail with only `` `.github/scripts/ci-check.sh` fails on the merge with main ``: the script's output had streamed past and the tool that ran the land showed a cut-off copy. Each build landed again, which passed, and none could name the failing test, so none could report it (`defect-reports`); one of the failures was a real CLM defect, a test binary reused from a deleted scratch tree (`fixed-land-tree`). A land's merged tree is gone when the land ends, so the output is the only evidence. The tail serves a tool that keeps the end of long output; the file serves one that keeps only the start, and anyone reading afterwards.

**Why per branch, and removed on success.** One file per land would grow without bound; one file per clone would be overwritten by the next land before a build read it. Keyed by branch, a build always finds its own, and removing the log when the branch's check passes bounds the directory to the branches whose last check failed, which is the set worth keeping.

**Why 40 lines.** `cargo test` ends with the `failures:` list, the result line and the `error: test failed, to rerun pass ...` line; vitest and playwright end with their summaries. Forty lines cover those with the last failure's own output; a fixed count keeps the rule testable.

**Alternatives.**
- *Only repeat the tail.* Nothing is kept when the tool shows only the head.
- *Keep the scratch tree on failure.* Large, and the output is what a build needs.
- *Parse the output for the failing stage and test.* The check is the repository's own script; its stages and runners vary.
- *Log `--check-only` too.* It runs in the session's own worktree, where the failing command can be run again.
