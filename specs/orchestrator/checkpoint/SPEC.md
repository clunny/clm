---
title: Checkpoint
status: active
owners: ["@clunny"]
depends_on: [orchestrator, ledger, check, vcs, model]
paths: ["crates/clm-checkpoint/**"]
successor: ""
serves: [capability/wrap-up-session, architecture/git-is-truth]
---

## Contract
`clm wrap-up` removes landed work, reports any work that isn't safe on `origin`, names the changes that are complete but not yet archived, and says whether the session can end (`checkpoint-before-done`). It exits 0 with `wrap-up ok` or 1 with `wrap-up incomplete`. It reads git, the ledger and the clone's build places only, so it works on every host and offline. It deletes the places that don't count (`build-places`), the backstop for builds and drafts nobody finished.

## Design
Crate `clm-checkpoint`. Whether a branch landed comes from `clm-git`'s ancestry queries on `origin/main`, worktrees and branches from `clm-git`'s queries, and the complete changes from `orchestrator`'s queue on `origin/main`'s tree.

**Landed** (`checkpoint`). A branch whose tip is an ancestor of `origin/main` off its first-parent chain reached `main` through a merge, `clm land`'s or a GitHub pull request's, so it needs no `landed` record. The test isn't "its tip is in `origin/main`", which a branch just created from `main` also passes: that branch's tip is a commit `main` itself was at, on the first-parent chain.

**Places** (`checkpoint-frees-places`). The places are judged before any removal because the removals delete a landed build's local and remote `impl/<component>`, and a build place with no branches reads as not yet claimed, so it would count by its `taken` time instead of as landed. One call of `free_stale_places`, never `places` then `free_place`, since a `clm run` or `clm draft` could retake a place of that name between the two, and checkpoint would delete the fresh place. Checkpoint has no `--expiry`: when a session ran its builds with a longer expiry than the default, checkpoint may free a place whose build is still running, which only lets one more build start on the clone than the cap.

**Removals** (`checkpoint`). A build's worktree is the one in `.claude/worktrees/agent-*` because that is where Claude Code puts a subagent's worktree. Its branch has landed and it is never the session's own, so nothing on it is lost when it can't be removed: the `left` line hands the directory to the human, and the local branch stays for the next wrap-up to delete once the human has deleted the directory. No `dirty` line names that directory, since git may not know it as a worktree. A `worktree-agent-*` branch is the one Claude Code makes for a build, which the build leaves when it switches to `impl/<component>`. A live claim is a commit on top of `origin/main`, unlanded work has commits `origin/main` lacks, and a new branch's tip is on the first-parent chain, so the landed test deletes none of them; a landed branch on `origin` is deleted leased on the tip checkpoint judged, so a push after the fetch keeps it. An archived change holds no `propose/<slug>` claim, so a claim made after it is kept, and checkpoint and `clm claim` agree.

**Unsafe work** (`checkpoint-reports-unsafe-work`). Only the session's own work counts against the wrap-up so that several sessions can each finish while the others still work. A build's worktree and branch count as elsewhere because git doesn't record which session made a build's worktree, and `build` already has each build's outcome, a land or why it stopped.

**Review mark** (`checkpoint-review-mark`, `checkpoint-recomputes-new-lands`). Recomputing a land runs the gate on it, and a repository's signed lands only grow, so checkpoint doesn't recompute the whole history each time it runs. The mark is in the clone's git common directory like the build places, so every worktree of the clone shares it. `ledger`'s `verify` is called twice, the second time with `--review` and `--since`; only the findings the recomputation can add are taken from the second call, since its `ledger/unrecorded-land` lines repeat the first's, or, in a repository with no `landed` record yet, name every commit after the mark. A finding holds the mark so that it is reported at every checkpoint until a human acts on it, while a land found clean is never recomputed by this clone again; a land recomputed as needing review while recorded as not gets `ledger/understated-land` first, so its `ledger/unapproved-land` is never left behind the mark. A merge whose `landed` record hasn't reached the clone yet gets only `ledger/unrecorded-land`, so it holds the mark and is recomputed once its record arrives. The mark is a cache derived from this clone's own earlier runs, which `git-is-truth` allows: nothing lands, approves or passes a check on it, deleting it (`git update-ref -d refs/clm/tiers-verified`) costs one full pass at the next checkpoint, and `clm ledger verify --review` with no `--since` is the full audit, which reads no mark. It is a ref rather than a file because the mark is a commit: git refuses a value that isn't one, and `git fetch --prune` leaves it alone. Custom refs are what `per-writer-ledgers` rejected for shared state because no host shows or fetches them, which is right for a clone's own mark.

A branch squash-merged or fast-forwarded into `main` isn't landed by this test: a squash leaves its tip outside `main`, and a fast-forward puts it on the first-parent chain. It stays and is reported, which loses nothing. A branch merged outside `clm land` is removed like any other landed branch, and `clm ledger verify` reports its merge as `ledger/unrecorded-land`, which makes the wrap-up incomplete.

## Requirements

### Requirement: checkpoint
`clm wrap-up` SHALL fetch `origin` with `--prune` and treat a `spec/*` or `impl/*` branch as landed when its current tip is an ancestor of `origin/main` and isn't on `origin/main`'s first-parent chain, whether or not a `landed` record names it. It SHALL remove each worktree on an `impl/*` branch in a `.claude/worktrees/agent-*` directory whose branch is landed and that has no uncommitted changes, whole, also when the build left a directory in it without its owner's write permission, and delete each local and `origin` `spec/*` or `impl/*` branch that is landed (leaving a worktree or main checkout with uncommitted changes in place), delete each `origin` `autosync/spec/<id>` whose change `<id>` is in `origin/main`, under `changes/<id>/` or `changes/archive/<id>/`, and delete each `origin` `propose/<slug>` whose `backlog/<slug>.md` isn't in `origin/main` or whose `<slug>` a change outside the archive in `origin/main` lists in `resolves`, printing `removed <path>` or `removed <branch>` for each. When such a worktree can't be removed, it SHALL print `left <path>: <error>; delete the directory by hand, then run git worktree prune` in place of `removed <path>`, with the error on one line, keep that worktree's local branch and go on, print no `dirty` or `elsewhere dirty` line for that path, and the line SHALL NOT make the wrap-up incomplete or exit 2. It SHALL delete each local branch `worktree-agent-*` that no worktree has checked out and whose tip is an ancestor of `origin/main`, switching a main checkout that is on such a branch and has no uncommitted changes to `main`, fast-forwarded to `origin/main`, first. It SHALL end with `wrap-up ok` and exit 0 when it printed no `dirty` or `unpushed` line of its own (`checkpoint-reports-unsafe-work`) and no violation, and `wrap-up incomplete` with exit 1 otherwise; a git failure SHALL exit 2. It SHALL NOT delete a `spec/*` or `impl/*` branch whose tip isn't an ancestor of `origin/main` or is on its first-parent chain, call `gh`, or touch files outside the worktrees it removes other than switching a landed main checkout to `main` and deleting the clone's places that no longer count (`checkpoint-frees-places`).

#### Needs
- vcs/remove-worktree

#### Scenario: landed work cleaned up
- GIVEN `impl/a` checked out in a clean, locked worktree in `.claude/worktrees/agent-1` that holds an ignored directory `cache/` with a file in it, `cache/` without its owner's write permission, and `spec/y` on `origin` with `autosync/spec/y`, both with `landed` records whose commits on `origin/main` have their tips as second parents, and `changes/y/` in `origin/main`
- WHEN `clm wrap-up` runs
- THEN the worktree's directory is gone from disk and `git worktree list` doesn't name it, both local and remote branches and `autosync/spec/y` are gone, it printed `removed` with the worktree's path and `removed impl/a`, prints `wrap-up ok` and exits 0

#### Scenario: build's worktree git can't remove
- GIVEN the landed `impl/a` of "landed work cleaned up", whose worktree `git worktree remove` refuses or can't delete
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

### Requirement: checkpoint-frees-places
`clm wrap-up` SHALL, after the fetch and before it removes any worktree or branch, free the clone's places that don't count with one call of `orchestrator`'s `free_stale_places` with the default expiry, printing `freed <name>` for each place it deleted, and SHALL keep each place that counts. It SHALL NOT read the places with `places` and then delete them with `free_place`. It SHALL keep a place file it can't read or parse and print `place <path>: <reason>` for it, and print the same line, with why, for a place it couldn't delete, in the sweep's order. When the sweep returns an error, checkpoint SHALL exit 2, as for a git failure.

#### Serves
- capability/wrap-up-session
- capability/work-concurrently

#### Needs
- orchestrator/free-stale-places

#### Scenario: stale places freed
- GIVEN places `a`, taken 1 minute ago, whose build's local `impl/a` landed on `origin/main` with a merge commit, was deleted on `origin`, and is checked out in a clean worktree at `.claude/worktrees/agent-1`; and `b`, for a live claim `origin/impl/b` committed 1 hour ago
- WHEN `clm wrap-up` runs
- THEN the place `a` is gone though checkpoint also removed `impl/a` and its worktree, the place `b` is still there, it printed `freed a`, and the places alone don't make it print `wrap-up incomplete`

#### Scenario: damaged place kept
- GIVEN a file `z` in the clone's places directory holding `not a place`
- WHEN `clm wrap-up` runs
- THEN the file is still there and it printed `place` with its path

### Requirement: checkpoint-review-mark
`clm wrap-up` SHALL run `clm ledger verify` against `origin/main` without `--review` over the whole chain, and with the recomputation of `ledger/verify-recomputes-review` only for the lands on `origin/main`'s first-parent chain after the commit the local ref `refs/clm/tiers-verified` names, or for the whole chain when the ref is unset, doesn't resolve to a commit, or names one that isn't an ancestor of `origin/main`, taking from the recomputation only its `ledger/understated-land`, `ledger/unrated-land` and `ledger/unapproved-land` findings, and printing each finding once among `clm ledger verify`'s, each making it end `wrap-up incomplete` with exit 1. Before it prints `wrap-up ok` or `wrap-up incomplete` it SHALL set the ref to the first parent of the oldest land it recomputed that got a `ledger/understated-land` or `ledger/unrated-land` finding, or of the oldest commit after the ref that the audit of the whole chain gave `ledger/unrecorded-land`, or to `origin/main` when none did, the oldest by the first-parent chain's order, and SHALL NOT push it or set it when it exits 2.

#### Needs
- ledger/verify-recomputes-review
- ledger/verify-unrated-land

#### Scenario: clean lands marked
- GIVEN a clone whose `origin/main` holds the history of `ledger`'s *honest lands*, no `refs/clm/tiers-verified`, and no other work
- WHEN `clm wrap-up` runs
- THEN it prints no `ledger/` line and `wrap-up ok`, and `refs/clm/tiers-verified` names `origin/main`'s commit

#### Scenario: lands before the mark not recomputed
- GIVEN a clone whose `origin/main` holds the history of `ledger`'s *review recorded as none*, with `refs/clm/tiers-verified` at `origin/main`
- WHEN `clm wrap-up` runs
- THEN it prints no `ledger/understated-land` line and `wrap-up ok`

#### Scenario: finding holds the mark
- GIVEN a clone whose `origin/main` holds the land of `ledger`'s *review recorded as none* and, after it, the spec land of *honest lands*, and no `refs/clm/tiers-verified`
- WHEN `clm wrap-up` runs twice
- THEN each run prints `ledger/understated-land` naming the first land's commit and `wrap-up incomplete` and exits 1, and `refs/clm/tiers-verified` names that land's first parent

#### Scenario: rewritten main
- GIVEN a clone whose `origin/main` holds the history of `ledger`'s *review recorded as none*, with `refs/clm/tiers-verified` at a commit that isn't an ancestor of `origin/main`
- WHEN `clm wrap-up` runs
- THEN it prints `ledger/understated-land` naming the land's commit

#### Scenario: record arriving late
- GIVEN a clone whose `origin/main` holds the code land of `ledger`'s *honest lands* and, after it, the land of *review recorded as none* with no `landed` record yet, and no `refs/clm/tiers-verified`
- WHEN `clm wrap-up` runs, then the land's signed `landed` record arrives, and `clm wrap-up` runs again
- THEN the first run prints `ledger/unrecorded-land` and leaves `refs/clm/tiers-verified` at the land's first parent, and the second prints `ledger/understated-land` naming the land's commit

#### Scenario: no lands yet
- GIVEN a clone whose `origin/main` has commits but no `landed` record, and no `refs/clm/tiers-verified`
- WHEN `clm wrap-up` runs twice
- THEN neither run prints `ledger/unrecorded-land`

#### Scenario: unrated land
- GIVEN a clone whose `origin/main` holds the history of `ledger`'s *unrated land*, and no other work
- WHEN `clm wrap-up` runs
- THEN it prints `ledger/unrated-land` naming the first land's commit and `ledger/understated-land` naming the second's, ends with `wrap-up incomplete` and exits 1

#### Serves
- capability/wrap-up-session
- architecture/humans-review-intent

### Requirement: checkpoint-reports-unsafe-work
`clm wrap-up` SHALL report, one line each, work CLM's flow makes that isn't safe on `origin`: a checkout on a `spec/*` or `impl/*` branch with uncommitted changes or untracked files that aren't ignored is *dirty*; a local `spec/*` or `impl/*` branch that isn't landed (`checkpoint`) is *unpushed* when it has commits its `origin` branch lacks, or no `origin` branch; and this clone's ledger branch is *unpushed* when it holds a record `origin` lacks. It SHALL print `dirty <path>` and `unpushed <branch>` for the session's own work: the checkout it runs in, the branch checked out there and the clone's ledger. It SHALL print `elsewhere dirty <path>` and `elsewhere unpushed <branch>` for the rest, a build's worktree in a `.claude/worktrees/agent-*` directory and its branch always among them. It SHALL also print `open <branch>` for each local `spec/*` or `impl/*` branch that is pushed but not landed, `complete <id>` for each change the queue reports complete on `origin/main`'s tree, extracted, and the violations of every `clm check` rule on that tree, each line starting `origin/main: ` before the violation's `<path>: <rule>: <message>`. It SHALL judge that tree with `check`'s `run_listed` (`check/run-listed`), given the files of the commit it extracted as `git ls-tree -r --name-only` lists them, so the rules that read the tracked files see the files `clm check` sees in a checkout of that commit, those under directories whose names start with `.` among them. The `open`, `elsewhere`, `complete`, `freed`, `place` and `left` lines SHALL NOT make the wrap-up incomplete.

#### Scenario: unpushed work
- GIVEN the checkout `clm wrap-up` runs in on local branch `spec/z`, with a commit `origin/spec/z` lacks
- WHEN `clm wrap-up` runs
- THEN it prints `unpushed spec/z` and `wrap-up incomplete`, and exits 1

#### Scenario: unpushed ledger
- GIVEN this clone's ledger branch with a record `origin` lacks
- WHEN `clm wrap-up` runs
- THEN it prints `unpushed clm/ledger/<writer>` and `wrap-up incomplete`, and exits 1

#### Scenario: open branch only
- GIVEN local branch `spec/z` pushed and unlanded, with nothing else outstanding
- WHEN `clm wrap-up` runs
- THEN it prints `open spec/z` and `wrap-up ok`, and exits 0

#### Scenario: unrecorded land
- GIVEN an `origin/main` with a merge commit no `landed` record names
- WHEN `clm wrap-up` runs
- THEN it prints `ledger/unrecorded-land` and `wrap-up incomplete`

#### Scenario: complete change
- GIVEN an `applied` change `x` in `origin/main` whose added requirements are all cited at their current versions
- WHEN `clm wrap-up` runs
- THEN it prints `complete x` and `wrap-up ok`, and exits 0

#### Scenario: human's files on main
- GIVEN the main checkout on `main` with an untracked `notes.txt`
- WHEN `clm wrap-up` runs
- THEN it prints no `dirty` line and exits 0

#### Scenario: another session's work
- GIVEN the main checkout on `spec/z` with an uncommitted change and a commit `origin/spec/z` lacks, and `clm wrap-up` run in a linked worktree `w` on `main`
- WHEN `clm wrap-up` runs in `w`
- THEN it prints `elsewhere dirty <main checkout>` and `elsewhere unpushed spec/z`, then `wrap-up ok`, and exits 0

#### Scenario: build's worktree is elsewhere
- GIVEN `clm wrap-up` run in a linked worktree `w`, and `.claude/worktrees/agent-1` on `impl/a` with an uncommitted change
- WHEN `clm wrap-up` runs in `w`
- THEN it prints `elsewhere dirty` with that worktree's path, then `wrap-up ok`, and exits 0

#### Scenario: outside glob under a hidden directory
- GIVEN an `origin/main` whose root sets `outside: [".azure-pipelines/**"]`, tracking `.azure-pipelines/ci.yml` in no component's `paths`, on which `clm check` passes
- WHEN `clm wrap-up` runs
- THEN it prints no `check/unowned-files` line, prints `wrap-up ok` and exits 0

#### Scenario: violation on main
- GIVEN an `origin/main` whose root sets `outside: []`, tracking `tools/x.sh` in no component's `paths`
- WHEN `clm wrap-up` runs
- THEN it prints a line starting `origin/main: tools/x.sh: check/unowned-files: ` and `wrap-up incomplete`, and exits 1

#### Serves
- capability/wrap-up-session
- architecture/git-is-truth

#### Needs
- check/run-listed

### Requirement: wrap-up-fetch-retries-race
`clm wrap-up` SHALL fetch `origin` through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`) with the arguments it fetches with today (`fetch --prune --quiet origin`), so a wrap-up whose fetch lost a race with a build in another worktree of the same clone over a remote-tracking ref goes on. When `fetch_retrying` fails, it SHALL fail as a failed fetch makes it fail today, removing and deleting nothing, with the error's message: git's message from the last fetch for `FetchError::Git`, and `origin can't be reached` for `FetchError::Offline`.

#### Scenario: a wrap-up whose fetch loses a ref race
- GIVEN a clone with nothing to clean up, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/impl/a'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm wrap-up` runs
- THEN it prints `wrap-up ok` and exits 0

#### Scenario: a wrap-up whose fetch keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm wrap-up` runs
- THEN it fails with a message containing `cannot lock ref`, and doesn't print `wrap-up ok`

#### Needs
- vcs/fetch-retries

#### Serves
- capability/work-concurrently

### Requirement: wrap-up-delete-loses-race
When `origin` refuses a delete `clm wrap-up` makes of a done branch there (`checkpoint`: a landed `spec/*` or `impl/*` branch, an `autosync/spec/<id>` or a `propose/<slug>`) because the branch is no longer at the tip the fetch read, `clm wrap-up` SHALL read that branch on `origin` again and go on. A branch `origin` no longer has SHALL count as done, since another session or command deleted it after the fetch and nothing is left to delete, and a branch whose tip moved SHALL be kept, as the lease means it to be. It SHALL print no `removed` line for either, telling a branch whose delete was refused from one its own push deleted by what git reports for each branch of that push, since both are gone from `origin` afterwards; it SHALL still delete every other done branch on `origin` and print that branch's `removed` line, and the refusal SHALL NOT make it exit 2 or end `wrap-up incomplete`. When a delete fails and `origin` still has the branch at the tip the fetch read, the failure is git's and it SHALL exit 2, as for any git failure.

#### Serves
- capability/work-concurrently

#### Scenario: a claim deleted since the fetch
- GIVEN `origin/propose/q` where `backlog/q.md` isn't in `origin/main`, the landed `origin/spec/y` of "landed work cleaned up", nothing else outstanding, and an executable `git` first on `PATH` that, before it runs the first `push` naming `origin`, deletes `propose/q` on `origin`, and runs the real `git` for every call
- WHEN `clm wrap-up` runs
- THEN it prints `removed origin/spec/y`, no `removed` line naming `propose/q` and `wrap-up ok`, exits 0, and `origin` has neither branch

#### Scenario: a claim retaken since the fetch
- GIVEN the same, with a `git` that moves `propose/q` on `origin` to a new commit instead of deleting it
- WHEN `clm wrap-up` runs
- THEN it prints `removed origin/spec/y`, no `removed` line naming `propose/q` and `wrap-up ok`, exits 0, `origin/propose/q` is at the new commit and `origin` has no `spec/y`

#### Scenario: a delete git fails
- GIVEN the same branches, with a `git` that fails every `push` naming `origin` with `fatal: unable to access` and exit 128, changing nothing on `origin`
- WHEN `clm wrap-up` runs
- THEN it exits 2 and doesn't print `wrap-up ok`
