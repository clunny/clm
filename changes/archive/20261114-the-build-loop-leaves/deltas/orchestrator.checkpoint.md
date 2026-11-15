---
component: orchestrator.checkpoint
---

## MODIFIED Requirements

### Requirement: checkpoint <!-- base: 9bbd35782dfc0a0d -->
`clm wrap-up` SHALL fetch `origin` with `--prune` and treat a `spec/*` or `impl/*` branch as landed when its current tip is an ancestor of `origin/main` and isn't on `origin/main`'s first-parent chain, whether or not a `landed` record names it. It SHALL remove each worktree on an `impl/*` branch in a `.claude/worktrees/agent-*` directory whose branch is landed and that has no uncommitted changes, whole, also when the build left a directory in it without its owner's write permission, and delete each local and `origin` `spec/*` or `impl/*` branch that is landed (leaving a worktree or main checkout with uncommitted changes in place), delete each `origin` `autosync/spec/<id>` whose change `<id>` is in `origin/main`, under `changes/<id>/` or `changes/archive/<id>/`, and delete each `origin` `propose/<slug>` whose `backlog/<slug>.md` isn't in `origin/main` or whose `<slug>` a change outside the archive in `origin/main` lists in `resolves`, printing `removed <path>` or `removed <branch>` for each. When such a worktree can't be removed, it SHALL print `left <path>: <error>; delete the directory by hand, then run git worktree prune` in place of `removed <path>`, with the error on one line, keep that worktree's local branch and go on, print no `dirty` or `elsewhere dirty` line for that path, and the line SHALL NOT make the wrap-up incomplete or exit 2. It SHALL delete each local branch `worktree-agent-*` that no worktree has checked out and whose tip is an ancestor of `origin/main`, switching a main checkout that is on such a branch and has no uncommitted changes to `main`, fast-forwarded to `origin/main`, first. It SHALL end with `wrap-up ok` and exit 0 when it printed no `dirty` or `unpushed` line of its own (`checkpoint-reports-unsafe-work`) and no violation, and `wrap-up incomplete` with exit 1 otherwise; a git failure SHALL exit 2. It SHALL NOT delete a `spec/*` or `impl/*` branch whose tip isn't an ancestor of `origin/main` or is on its first-parent chain, call `gh`, or touch files outside the worktrees it removes other than switching a landed main checkout to `main` and deleting the clone's places that no longer count (`checkpoint-frees-places`).

#### Needs
- vcs/remove-worktree

#### Scenario: landed work cleaned up
- GIVEN `impl/a` checked out in a clean, locked worktree in `.claude/worktrees/agent-1` that holds an ignored directory `cache/` with a file in it, `cache/` without its owner's write permission, and `spec/y` on `origin` with `autosync/spec/y`, both with `landed` records whose commits on `origin/main` have their tips as second parents, and `changes/y/` in `origin/main`
- WHEN `clm wrap-up` runs
- THEN the worktree's directory is gone from disk and `git worktree list` doesn't name it, both local and remote branches and `autosync/spec/y` are gone, it printed `removed` with the worktree's path and `removed impl/a`, prints `wrap-up ok` and exits 0

#### Scenario: build's worktree git can't remove
- GIVEN the landed `impl/a` of "landed work cleaned up", and a `git` whose `worktree remove` refuses the worktree, leaving it listed and on disk
- WHEN `clm wrap-up` runs
- THEN it printed a line starting `left ` with the worktree's path and `git worktree prune` and no `removed`, `dirty` or `elsewhere dirty` line with that path, the local branch `impl/a` still exists, and it prints `wrap-up ok` and exits 0

#### Scenario: done backlog claims removed
- GIVEN `origin/propose/q` where `backlog/q.md` isn't in `origin/main`; `origin/propose/r` where `changes/y/change.md` in `origin/main` lists `r` in `resolves`; `origin/propose/s` where `backlog/s.md` is in `origin/main` and no change there names `s`; and `origin/propose/t` where `backlog/t.md` is in `origin/main` and only `changes/archive/z/change.md` names `t`
- WHEN `clm wrap-up` runs
- THEN `origin/propose/q` and `origin/propose/r` are gone, and `origin/propose/s` and `origin/propose/t` still exist

#### Scenario: live claim kept
- GIVEN `origin/impl/b`, a claim commit on top of `origin/main` pushed by another session
- WHEN `clm wrap-up` runs
- THEN `origin/impl/b` still exists

#### Scenario: branches from main kept
- GIVEN `spec/n`, created from `origin/main`'s tip and pushed with no commits of its own, and local `spec/m` at an older commit on `origin/main`'s first-parent chain with no commits of its own, neither with a `landed` record
- WHEN `clm wrap-up` runs
- THEN `spec/n` still exists locally and on `origin`, and `spec/m` still exists

#### Scenario: abandoned autosync kept
- GIVEN `origin/autosync/spec/q` and no `changes/q/` in `origin/main`
- WHEN `clm wrap-up` runs
- THEN `origin/autosync/spec/q` still exists

#### Scenario: build's abandoned branch
- GIVEN a local branch `worktree-agent-1` at `origin/main`'s tip that no worktree has checked out, and `worktree-agent-2` with a commit `origin/main` lacks
- WHEN `clm wrap-up` runs
- THEN `worktree-agent-1` is gone and `worktree-agent-2` still exists

#### Scenario: human's worktree kept
- GIVEN a clean worktree `.claude/worktrees/p3` on `impl/a`, with a `landed` record at its tip
- WHEN `clm wrap-up` runs elsewhere
- THEN `.claude/worktrees/p3` and the local branch `impl/a` are still there, and it prints neither as `removed`

#### Scenario: merged without a record
- GIVEN local `impl/c` with no `origin` branch and no `landed` record, whose tip is the second parent of a merge commit on `origin/main`'s first-parent chain
- WHEN `clm wrap-up` runs
- THEN `impl/c` is gone, it prints `removed impl/c` and no `unpushed` line for it, and exits 0
