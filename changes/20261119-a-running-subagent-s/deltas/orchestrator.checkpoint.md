---
component: orchestrator.checkpoint
base: { design: "073032e3b9e1aa40" }
---

## MODIFIED Design
Crate `clm-checkpoint`. Whether a branch landed comes from `clm-git`'s ancestry queries on `origin/main`, worktrees and branches from `clm-git`'s queries, and the complete changes from `orchestrator`'s queue on `origin/main`'s tree.

**Landed** (`checkpoint`). A branch whose tip is an ancestor of `origin/main` off its first-parent chain reached `main` through a merge, `clm land`'s or a GitHub pull request's, so it needs no `landed` record. The test isn't "its tip is in `origin/main`", which a branch just created from `main` also passes: that branch's tip is a commit `main` itself was at, on the first-parent chain.

**Places** (`checkpoint-frees-places`). The places are judged before any removal because the removals delete a landed build's local and remote `impl/<component>`, and a build place with no branches reads as not yet claimed, so it would count by its `taken` time instead of as landed. One call of `free_stale_places`, never `places` then `free_place`, since a `clm run` or `clm draft` could retake a place of that name between the two, and checkpoint would delete the fresh place. Checkpoint has no `--expiry`: when a session ran its builds with a longer expiry than the default, checkpoint may free a place whose build is still running, which only lets one more build start on the clone than the cap.

**Removals** (`checkpoint`). A build's worktree is the one in `.claude/worktrees/agent-*` because that is where Claude Code puts a subagent's worktree. Its branch has landed and it is never the session's own, so nothing on it is lost when it can't be removed. **A run still working there** (`a-held-lock-keeps-a-worktree`). A landed branch doesn't mean its run has ended: a draft that lands a fix on `impl/<component>` goes on working in the same worktree, and a build still writes its report after its land, while every step of the build loop runs `clm wrap-up`. So a worktree whose lock is held by a process that still runs (`vcs/held-lock`) is kept with its branch. Claude Code locks each subagent's worktree with a reason naming its own session's process (`claude agent agent-<id> (pid <n> start <t>)`, seen on 2026-11-19), so wrap-up keeps every worktree of a session that still runs and removes those of a session that has gone; a live session's finished runs are removed by the `clm run --finish` or `clm draft --finish` its loop or skill runs on each return, which unlocks first. A lock that names no process is removed as before, so an agent harness that writes no process ID loses nothing it had. When it can't be removed: the `left` line hands the directory to the human, and the local branch stays for the next wrap-up to delete once the human has deleted the directory. No `dirty` line names that directory, since git may not know it as a worktree. A `worktree-agent-*` branch is the one Claude Code makes for a build, which the build leaves when it switches to `impl/<component>`. A live claim is a commit on top of `origin/main`, unlanded work has commits `origin/main` lacks, and a new branch's tip is on the first-parent chain, so the landed test deletes none of them; a landed branch on `origin` is deleted leased on the tip checkpoint judged, so a push after the fetch keeps it. An archived change holds no `propose/<slug>` claim, so a claim made after it is kept, and checkpoint and `clm claim` agree.

**Unsafe work** (`checkpoint-reports-unsafe-work`). Only the session's own work counts against the wrap-up so that several sessions can each finish while the others still work. A build's worktree and branch count as elsewhere because git doesn't record which session made a build's worktree, and `build` already has each build's outcome, a land or why it stopped.

**Review mark** (`checkpoint-review-mark`, `checkpoint-recomputes-new-lands`). Recomputing a land runs the gate on it, and a repository's signed lands only grow, so checkpoint doesn't recompute the whole history each time it runs. The mark is in the clone's git common directory like the build places, so every worktree of the clone shares it. `ledger`'s `verify` is called twice, the second time with `--review` and `--since`; only the findings the recomputation can add are taken from the second call, since its `ledger/unrecorded-land` lines repeat the first's, or, in a repository with no `landed` record yet, name every commit after the mark. A finding holds the mark so that it is reported at every checkpoint until a human acts on it, while a land found clean is never recomputed by this clone again; a land recomputed as needing review while recorded as not gets `ledger/understated-land` first, so its `ledger/unapproved-land` is never left behind the mark. A merge whose `landed` record hasn't reached the clone yet gets only `ledger/unrecorded-land`, so it holds the mark and is recomputed once its record arrives. The mark is a cache derived from this clone's own earlier runs, which `git-is-truth` allows: nothing lands, approves or passes a check on it, deleting it (`git update-ref -d refs/clm/tiers-verified`) costs one full pass at the next checkpoint, and `clm ledger verify --review` with no `--since` is the full audit, which reads no mark. It is a ref rather than a file because the mark is a commit: git refuses a value that isn't one, and `git fetch --prune` leaves it alone. Custom refs are what `per-writer-ledgers` rejected for shared state because no host shows or fetches them, which is right for a clone's own mark.

A branch squash-merged or fast-forwarded into `main` isn't landed by this test: a squash leaves its tip outside `main`, and a fast-forward puts it on the first-parent chain. It stays and is reported, which loses nothing. A branch merged outside `clm land` is removed like any other landed branch, and `clm ledger verify` reports its merge as `ledger/unrecorded-land`, which makes the wrap-up incomplete.

## MODIFIED Requirements

### Requirement: checkpoint <!-- base: 3d2718a5bee53b7a -->
`clm wrap-up` SHALL fetch `origin` with `--prune` and treat a `spec/*` or `impl/*` branch as landed when its current tip is an ancestor of `origin/main` and isn't on `origin/main`'s first-parent chain, whether or not a `landed` record names it. It SHALL remove each worktree on an `impl/*` branch in a `.claude/worktrees/agent-*` directory whose branch is landed, that has no uncommitted changes and whose lock isn't held (`vcs/held-lock`), whole, also when the build left a directory in it without its owner's write permission, and delete each local and `origin` `spec/*` or `impl/*` branch that is landed (leaving a worktree or main checkout with uncommitted changes, and a worktree whose lock is held, in place, with the branch it has checked out), delete each `origin` `autosync/spec/<id>` whose change `<id>` is in `origin/main`, under `changes/<id>/` or `changes/archive/<id>/`, and delete each `origin` `propose/<slug>` whose `backlog/<slug>.md` isn't in `origin/main` or whose `<slug>` a change outside the archive in `origin/main` lists in `resolves`, printing `removed <path>` or `removed <branch>` for each. When such a worktree can't be removed, it SHALL print `left <path>: <error>; delete the directory by hand, then run git worktree prune` in place of `removed <path>`, with the error on one line, keep that worktree's local branch and go on, print no `dirty` or `elsewhere dirty` line for that path, and the line SHALL NOT make the wrap-up incomplete or exit 2. It SHALL delete each local branch `worktree-agent-*` that no worktree has checked out and whose tip is an ancestor of `origin/main`, switching a main checkout that is on such a branch and has no uncommitted changes to `main`, fast-forwarded to `origin/main`, first. It SHALL end with `wrap-up ok` and exit 0 when it printed no `dirty` or `unpushed` line of its own (`checkpoint-reports-unsafe-work`) and no violation, and `wrap-up incomplete` with exit 1 otherwise; a git failure SHALL exit 2. It SHALL NOT delete a `spec/*` or `impl/*` branch whose tip isn't an ancestor of `origin/main` or is on its first-parent chain, call `gh`, or touch files outside the worktrees it removes other than switching a landed main checkout to `main` and deleting the clone's places that no longer count (`checkpoint-frees-places`).

#### Needs
- vcs/remove-worktree
- vcs/held-lock

#### Scenario: landed work cleaned up
- GIVEN `impl/a` checked out in a clean, locked worktree in `.claude/worktrees/agent-1` that holds an ignored directory `cache/` with a file in it, `cache/` without its owner's write permission, and `spec/y` on `origin` with `autosync/spec/y`, both with `landed` records whose commits on `origin/main` have their tips as second parents, and `changes/y/` in `origin/main`
- WHEN `clm wrap-up` runs
- THEN the worktree's directory is gone from disk and `git worktree list` doesn't name it, both local and remote branches and `autosync/spec/y` are gone, it printed `removed` with the worktree's path and `removed impl/a`, prints `wrap-up ok` and exits 0

#### Scenario: a running subagent's worktree kept
- GIVEN the landed `impl/a` of "landed work cleaned up", its worktree locked with the reason `claude agent agent-1 (pid <p> start 1)`, `<p>` the ID of a process that is running
- WHEN `clm wrap-up` runs
- THEN the worktree's directory is still there and `git worktree list` names it on `impl/a`, the local branch `impl/a` still exists, it printed no `removed` line naming the worktree's path and no line `removed impl/a`, prints `wrap-up ok` and exits 0

#### Scenario: an ended session's worktree removed
- GIVEN the landed `impl/a` of "landed work cleaned up", its worktree locked with the reason `claude agent agent-1 (pid <p> start 1)`, `<p>` the ID of a process that has ended
- WHEN `clm wrap-up` runs
- THEN the worktree's directory is gone from disk, the local branch `impl/a` is gone, and it printed `removed` with the worktree's path and `removed impl/a`

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
