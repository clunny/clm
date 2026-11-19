---
status: done
resolves: [a-subagent-s-work-is-counted-twice-once-its-worktree-is-removed]
---

## Intent

A build run as a subagent is charged once, to its own branch, however long after its land the transcripts are read again. A subagent's work counted by its worktree's branch is recorded under the session `<session>/<agent>`, and that record pins the subagent to the branch: a later read counts the subagent there even after its worktree is removed or reused, and a subagent in a worktree that has no branch yet waits instead of being counted for the launching session's branch.

## Problem

`effort/transcripts` counts a subagent's entries for the branch checked out in the linked worktree their `cwd` is under, as `git worktree list` names it when the record is made, and falls back to the entries' own `gitBranch` (the launching session's) when the worktree is detached or gone. A record never shrinks (`record-command`), so an entry that moves from one record to another is counted in both:

- A build's subagent lands, recording its work for `impl/a` from its worktree. The worktree is then removed, as it is after every build. The next `clm effort record`, or the launching session's land through `unrecorded`, reads the same entries for their `gitBranch`, so the build is charged again to the launching session's branch, usually `main` and so `unattributed`.
- A worktree path reused for another branch (`clm-loop` reuses `.claude/worktrees/agent-<name>` for a harness without worktrees of its own) moves the old subagent's work onto the new branch, again counting it twice.
- The Design accepted the double count for a worktree still detached when a record is made, judging it rare; removal, which is the common case, it didn't consider.

The capability and architecture reviews price capabilities by these records, and builds are most of the spend, so every build's cost is roughly doubled in the total and its second half lands on `unattributed`.

## Approaches

- **Taken: pin a subagent to the branch it was first recorded for, through the record's `session`.** The work a subagent did in a worktree with a branch is grouped under `<sessionId>/<agent>`, `<agent>` its file's name without `.jsonl` (Claude Code names each launch's file afresh). `record` and `unrecorded` already read the fold; they pass `sessions` the branch of each `<session>/<agent>` the fold holds a record of, and a pinned file's entries count for that branch whatever the worktree holds now. It handles removal and reuse alike, across clones, since the fold is shared. A subagent in a linked worktree that is detached counts for nothing until it is pinned or its worktree is gone, which also ends the overlap the Design accepted.
- **Not taken: an `effort` record field listing the subagent files it counted.** It says the same thing, but it changes `ledger`'s Contract and `Body::Effort`, which `effort` builds by struct literal, so it needs a coupled sequence across two components (`coupled-changes-sequence`) for no behaviour the `session` string can't carry. Nothing reads an `effort` record's `session` but its `id` and `clm ledger show`.
- **Not taken: count nothing for a subagent whose worktree is gone.** It ends the double count on removal with no pin, but loses the work of a subagent never recorded before its worktree went (a draft left for review, a build that stopped), and leaves the reuse case as it is.
- **Not taken: accept it.** Every build run as a subagent is affected, not an edge case, and the review's numbers are what the human decides cuts by.

## Outcome

- `effort/transcripts` (modified): `sessions` takes `pinned`; a subagent's entries in a worktree with a branch count under `<sessionId>/<agent>`; a pinned subagent counts for its pinned branch; one in a detached worktree, unpinned, counts for nothing; an unpinned one whose worktree is gone, or that ran in the main worktree, counts for its `gitBranch` under its `sessionId`, as before.
- `effort/record-command` (modified): pins come from the fold's `effort` records, the earliest `at` deciding between two; a new scenario records a build's subagent once, its worktree removed after the record. `unrecorded`, which `clm land` calls, pins alike (Contract).
- `effort`'s Contract and Design say so, with what stays accepted.

## Edge cases and workarounds

- Records made by the `effort/transcripts@3669b654` build, between its land and this change's, hold a build's subagent work under `<sessionId>` on its `impl/<component>`, unpinned, so a later read counts it for the launching session's branch too, as now. No record is rewritten, since a record never shrinks; it is the day's builds.
- A subagent never recorded while its worktree had a branch, whose worktree path is then reused for another branch, counts for the new branch, once.
- A subagent in a worktree that stays detached until it is removed (no build does that) is counted for its `gitBranch` once the worktree is gone.
- Two clones that recorded one subagent for different branches: the record with the earliest `at` pins it; the other record stays, since a record never shrinks.
- No capability test sees a difference: the effort records' totals for a single build, as `clm effort record` and `clm land` write them, keep their branch; only the `session` string of a subagent's record changes.

## Open questions

None.
