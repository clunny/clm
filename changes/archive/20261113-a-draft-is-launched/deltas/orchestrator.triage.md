---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: draft-resume <!-- base: 62d8d617ff02f02a -->
`clm draft --resume <change-id>` SHALL fetch `origin`, take the place `<change-id>` (reusing it when it still counts, `orchestrator/places`), and, when `origin/spec/<change-id>` or `origin/autosync/spec/<change-id>` exists, write to `.clm/prompts/draft/<change-id>.md` the prompt for a session that switches its checkout to that branch (the newer of the two when both exist) as `spec/<change-id>`, reads its `changes/<change-id>/change.md` and the items it names, and carries the draft on from where its commits leave off under the same rules as a draft `clm draft` writes, then print the launcher as `clm draft` does and `draft <change-id>` as the last line on stderr, and exit 0. The `git switch` command the prompt gives for that switch SHALL succeed when run in a worktree of the clone, leaving it on `spec/<change-id>` at the commit of that branch on `origin`, whether the clone holds no local branch `spec/<change-id>` or one, at any commit, that no worktree has checked out, as the draft's first session leaves behind. When neither branch exists it SHALL free the place it took, claim nothing, print that the draft left nothing to resume, and exit 1; the items the draft claimed stay held until their claims lapse. It SHALL exit 3, taking nothing, when no place is free.

#### Scenario: a draft that pushed
- GIVEN `origin/spec/20261112-x` holding `changes/20261112-x/change.md`, whose draft's session ended without returning
- WHEN `clm draft --resume 20261112-x` runs
- THEN it exits 0, `.clm/prompts/draft/20261112-x.md` names `spec/20261112-x`, the place `20261112-x` counts, and stderr ends `draft 20261112-x`

#### Scenario: nothing pushed
- GIVEN no `spec/20261112-y` or `autosync/spec/20261112-y` on `origin`
- WHEN `clm draft --resume 20261112-y` runs
- THEN it exits 1 saying the draft left nothing to resume, and no place `20261112-y` is left

#### Scenario: the first session's local branch is still there
- GIVEN `origin/spec/20261112-z` holding `changes/20261112-z/change.md`, and the clone holding a local branch `spec/20261112-z` at an older commit, which no worktree has checked out
- WHEN `clm draft --resume 20261112-z` runs, and the `git switch` command its prompt gives is then run in a new worktree of the clone
- THEN that command exits 0, and the worktree is on `spec/20261112-z` at the commit of `origin/spec/20261112-z`

#### Serves
- capability/build-from-spec
- capability/triage-unattended
