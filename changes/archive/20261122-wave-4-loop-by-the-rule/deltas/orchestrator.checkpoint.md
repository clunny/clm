---
component: orchestrator.checkpoint
base: { contract: "2c8d6d5ed5491697", design: "8c3f910447f0bf79" }
---

## MODIFIED Contract
`clm wrap-up` removes landed work, reports any work that isn't safe on `origin`, names the changes that are complete but not yet archived, and says whether the session can end (`checkpoint-before-done`). It exits 0 with `wrap-up ok`, 1 with `wrap-up incomplete`, and 2 when git fails, as when `origin` can't be reached, or when the ledger's audit or the sweep of the places fails. It reads git, the ledger and the clone's build places, and calls no host's command, so it works on every host; it needs `origin`, which it fetches first. It deletes the places that don't count (`build-places`), the backstop for builds and drafts nobody finished.

`clm-checkpoint` offers `clm wrap-up` as a call, `wrap_up_command(args, root, env, out, err)` (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. Its arguments are `WrapUpArgs`, which `crates/clm`'s parser reads. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Design
Crate `clm-checkpoint`. Whether a branch landed comes from `clm-git`'s ancestry queries on `origin/main`, worktrees and branches from `clm-git`'s queries, and the complete changes from `orchestrator`'s queue on `origin/main`'s tree.

**Landed** (`checkpoint`). A branch whose tip is an ancestor of `origin/main` off its first-parent chain reached `main` through a merge, `clm land`'s or a GitHub pull request's, so it needs no `landed` record. The test isn't "its tip is in `origin/main`", which a branch just created from `main` also passes: that branch's tip is a commit `main` itself was at, on the first-parent chain.

**Places** (`checkpoint-frees-places`). The places are judged before any removal because the removals delete a landed build's local and remote `impl/<component>`, and a build place with no branches reads as not yet claimed, so it would count by its `taken` time instead of as landed. One call of `free_stale_places`, never `places` then `free_place`, since a `clm run` or `clm draft` could retake a place of that name between the two, and checkpoint would delete the fresh place. Checkpoint has no `--expiry`: when a session ran its builds with a longer expiry than the default, checkpoint may free a place whose build is still running, which only lets one more build start on the clone than the cap.

**Removals** (`checkpoint`). A build's worktree is the one in `.claude/worktrees/agent-*` because that is where Claude Code puts a subagent's worktree. Its branch has landed and it is never the session's own, so nothing on it is lost when it can't be removed. The worktree wrap-up runs in stays, even landed: removing it would pull the directory out from under the session that ran it. **A run still working there** (`a-held-lock-keeps-a-worktree`). A landed branch doesn't mean its run has ended: a draft that lands a fix on `impl/<component>` goes on working in the same worktree, and a build still writes its report after its land, while every step of the build loop runs `clm wrap-up`. So a worktree whose lock is held by a process that still runs (`vcs/held-lock`) is kept with its branch. Claude Code locks each subagent's worktree with a reason naming its own session's process (`claude agent agent-<id> (pid <n> start <t>)`), so wrap-up keeps the worktree of every subagent still running and removes those a session that has gone left locked. Claude Code keeps that lock after the subagent has handed back, until its session ends, so within a session wrap-up removes no subagent's worktree: a returned run's worktree is removed by the `clm run --finish <component> --returned` or `clm draft --finish` the build loop runs on the return, which unlock it whatever its lock names (`orchestrator.run/run-finish`, which also says what `clm run --finish` keeps without `--returned`), and wrap-up removes what a session that has gone left. A lock that names no process is removed as before, so an agent harness that writes no process ID loses nothing it had. A worktree it can't remove isn't judged for uncommitted work, since git may no longer know it as a worktree, and its branch waits for the wrap-up after the human has deleted the directory. A `worktree-agent-*` branch is the one Claude Code makes for a build, which the build leaves when it switches to `impl/<component>`. A live claim is a commit on top of `origin/main`, unlanded work has commits `origin/main` lacks, and a new branch's tip is on the first-parent chain, so the landed test deletes none of them; a landed branch on `origin` is deleted leased on the tip checkpoint judged, so a push after the fetch keeps it. An archived change holds no `propose/<slug>` claim, so a claim made after it is kept, and checkpoint and `clm claim` agree. A main checkout that can't reach `main`, because another worktree has it or the local `main` has commits `origin/main` lacks, is left where git leaves it: forcing `main` would discard the human's commit.

**Unsafe work** (`checkpoint-reports-unsafe-work`). Only the session's own work counts against the wrap-up so that several sessions can each finish while the others still work. A build's worktree and branch count as elsewhere because git doesn't record which session made a build's worktree, and `build` already has each build's outcome, a land or why it stopped. Nothing is judged complete or done on a graph that doesn't load, so a backlog claim waits for a wrap-up on a `main` whose specs load.

**Review mark** (`checkpoint-review-mark`, `checkpoint-recomputes-new-lands`). Recomputing a land runs the gate on it, and a repository's signed lands only grow, so checkpoint doesn't recompute the whole history each time it runs. The mark is in the clone's git common directory like the build places, so every worktree of the clone shares it. `ledger`'s `verify` is called twice, the second time with `--review` and `--since`; only the findings the recomputation can add are taken from the second call, since its `ledger/unrecorded-land` lines repeat the first's, or, in a repository with no `landed` record yet, name every commit after the mark. A finding holds the mark so that it is reported at every checkpoint until a human acts on it, while a land found clean is never recomputed by this clone again; a land recomputed as needing review while recorded as not gets `ledger/understated-land` first, so its `ledger/unapproved-land` is never left behind the mark. A merge whose `landed` record hasn't reached the clone yet gets only `ledger/unrecorded-land`, so it holds the mark and is recomputed once its record arrives. The mark is a cache derived from this clone's own earlier runs, which `git-is-truth` allows: nothing lands, approves or passes a check on it, deleting it (`git update-ref -d refs/clm/tiers-verified`) costs one full pass at the next checkpoint, and `clm ledger verify --review` with no `--since` is the full audit, which reads no mark. It is a ref rather than a file because the mark is a commit: git refuses a value that isn't one, and `git fetch --prune` leaves it alone. Custom refs are what `per-writer-ledgers` rejected for shared state because no host shows or fetches them, which is right for a clone's own mark. An audit that fails is a failure of wrap-up, as it is of `clm ledger verify`: the mark can't be judged without it.

**Failures.** What wrap-up did before git failed is printed before the error, so a person keeps the record of what was deleted.

A branch squash-merged or fast-forwarded into `main` isn't landed by this test: a squash leaves its tip outside `main`, and a fast-forward puts it on the first-parent chain. Keeping it loses nothing.

## MODIFIED Requirements

### Requirement: checkpoint <!-- base: 9f671f32f9e36899 -->
`clm wrap-up` SHALL fetch `origin` with `--prune` (`wrap-up-fetch-retries-race`) and treat a `spec/*` or `impl/*` branch as landed when its current tip is an ancestor of `origin/main` and isn't on `origin/main`'s first-parent chain, whether or not a `landed` record names it. It SHALL remove each worktree on an `impl/*` branch in a `.claude/worktrees/agent-*` directory whose branch is landed, that has no uncommitted changes or untracked files that aren't ignored and whose lock isn't held (`vcs/held-lock`), whole, also when the build left a directory in it without its owner's write permission; it SHALL NOT remove the worktree it runs in (the one holding `--root`, or the current directory), nor delete the branch checked out there, though it is a landed build's. A main checkout with no uncommitted changes or untracked files that aren't ignored on a landed `spec/*` or `impl/*` branch, or on a `worktree-agent-*` branch whose tip is an ancestor of `origin/main`, it SHALL switch to `main`, fast-forwarded to `origin/main`, and delete that branch; when `main` is checked out in another worktree, it SHALL leave the main checkout on its branch and keep the branch, with no line and no failure, and when the local `main` can't be fast-forwarded to `origin/main`, it SHALL switch the main checkout to the local `main` at its commit and delete the branch. It SHALL leave every worktree it doesn't remove or switch in place, with the branch it has checked out. A worktree git still lists whose directory doesn't exist SHALL be neither removed nor reported, and the branch it has checked out kept until `git worktree prune` drops it. It SHALL delete each local `spec/*` or `impl/*` branch that is landed and that no worktree it leaves in place has checked out, each landed `spec/*` or `impl/*` branch on `origin`, each `origin` `autosync/spec/<id>` whose change `<id>` is in `origin/main`, under `changes/<id>/` or `changes/archive/<id>/`, and each `origin` `propose/<slug>` whose `backlog/<slug>.md` isn't in `origin/main` or whose `<slug>` a change outside the archive in `origin/main` lists in `resolves`, printing `removed <path>` for a worktree, `removed <branch>` for a local branch and `removed origin/<branch>` for a branch on `origin`. When such a worktree can't be removed, it SHALL print `left <path>: <error>; delete the directory by hand, then run git worktree prune` in place of `removed <path>`, with the error on one line, keep that worktree's local branch and go on, print no `dirty` or `elsewhere dirty` line for that path, and the line SHALL NOT make the wrap-up incomplete or exit 2. It SHALL delete each local branch `worktree-agent-*` that no worktree has checked out and whose tip is an ancestor of `origin/main`. It SHALL end with `wrap-up ok` and exit 0 when it printed no `dirty` or `unpushed` line of its own (`checkpoint-reports-unsafe-work`) and no violation, and `wrap-up incomplete` with exit 1 otherwise. When git fails, or the ledger's audit (`checkpoint-review-mark`) or the sweep of the places (`checkpoint-frees-places`) does, it SHALL print to standard output the `removed`, `left`, `freed` and `place` lines of what it had already done, a `removed origin/<branch>` line for each branch on `origin` its push deleted before the failure among them, and no other line, write `clm wrap-up: <why>` to standard error, print no `wrap-up ok` or `wrap-up incomplete` line, and exit 2. It SHALL NOT delete a `spec/*` or `impl/*` branch whose tip isn't an ancestor of `origin/main` or is on its first-parent chain, call `gh`, or touch files outside the worktrees it removes other than switching a main checkout to `main` and deleting the clone's places that no longer count (`checkpoint-frees-places`).

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

#### Scenario: the caller's own build worktree kept
- GIVEN the landed `impl/a` of "landed work cleaned up"
- WHEN `clm wrap-up` runs inside its worktree
- THEN the worktree's directory and the local branch `impl/a` are still there, it printed no `removed` line with the worktree's path, no line `removed impl/a` and no `unpushed` line, prints `wrap-up ok` and exits 0

#### Scenario: main checkout that can't reach main
- GIVEN the main checkout clean on landed `spec/y`, in two cases: (a) `main` checked out in a linked worktree; (b) nothing else holding `main`, and a local `main` with a commit `origin/main` lacks
- WHEN `clm wrap-up` runs in each
- THEN in (a) the main checkout is still on `spec/y`, `spec/y` exists and no `removed spec/y` line is printed; in (b) the main checkout is on `main` at that commit, `spec/y` is gone and `removed spec/y` is printed; each exits 0

#### Scenario: build's worktree gone from disk
- GIVEN the landed `impl/a` of "landed work cleaned up", its worktree's directory deleted and not pruned
- WHEN `clm wrap-up` runs
- THEN no line names the worktree's path, the local branch `impl/a` still exists, and it prints `wrap-up ok` and exits 0

#### Scenario: a git failure
- GIVEN four cases: (a) `--root` a temporary empty directory that isn't in a repository; (b) `--root` a path that doesn't exist; (c) the local landed `impl/c` of "merged without a record", a landed `origin/spec/y`, and an executable `git` first on `PATH` that fails every `push` naming `origin` with `fatal: unable to access` and exit 128, running the real `git` for every other call; (d) the landed `origin/spec/y` and `origin/propose/q` of "a claim deleted since the fetch", and a `git` that, for the first `push` naming `origin`, deletes `spec/y` there with the real `git`, then prints ` ! [remote rejected] (delete) -> propose/q (failed to update ref)` on stderr and exits 1, leaving `propose/q` at its tip
- WHEN `clm wrap-up` runs in each
- THEN each exits 2, prints no `wrap-up ok` or `wrap-up incomplete` line, and writes to standard error a line starting `clm wrap-up: `; in (a) and (b) standard output is empty; in (c) it is the line `removed impl/c`, and `impl/c` is gone; in (d) it is the line `removed origin/spec/y`

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

### Requirement: checkpoint-frees-places <!-- base: 380080948318ce02 -->
`clm wrap-up` SHALL, after the fetch and before it removes any worktree or branch, free in one sweep the clone's places that don't count, as `orchestrator/free-stale-places` judges them with its default expiry of 4 hours, printing `freed <name>` for each place it deleted, and SHALL keep each place that counts. A place that `clm run` or `clm draft` writes while the sweep waits for the places lock SHALL be judged as written, never deleted for what it held before. It SHALL keep a place file it can't read or parse and print `place <path>: <reason>` for it, and print the same line, with why, for a place it couldn't delete, in the sweep's order. When the places directory can't be found or made, or its lock can't be taken, it SHALL write `clm wrap-up: <why>` to standard error and exit 2, as for a git failure.

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

### Requirement: checkpoint-review-mark <!-- base: 87d81f940f64a696 -->
`clm wrap-up` SHALL run `clm ledger verify` against `origin/main` without `--review` over the whole chain, and with the recomputation of `ledger/verify-recomputes-review` only for the lands on `origin/main`'s first-parent chain after the commit the local ref `refs/clm/tiers-verified` names, or for the whole chain when the ref is unset, doesn't resolve to a commit, or names one that isn't an ancestor of `origin/main`, taking from the recomputation only its `ledger/understated-land`, `ledger/unrated-land` and `ledger/unapproved-land` findings, and printing each finding once among `clm ledger verify`'s, each making it end `wrap-up incomplete` with exit 1. Before it prints `wrap-up ok` or `wrap-up incomplete` it SHALL set the ref to the first parent of the oldest land it recomputed that got a `ledger/understated-land` or `ledger/unrated-land` finding, or of the oldest commit after the ref that the audit of the whole chain gave `ledger/unrecorded-land`, or to `origin/main` when none did, the oldest by the first-parent chain's order, and SHALL NOT push it or set it when it exits 2. When `clm ledger verify` fails, for git or for a committed `clm.toml` it reads that doesn't parse (`ledger/verify-signed-lands`), it SHALL write `clm wrap-up: ledger: <why>` to standard error and exit 2, as `clm ledger verify` does, with the lines `checkpoint` states for a failure.

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

#### Scenario: an audit that fails
- GIVEN a clone whose `origin/main` holds the history of `ledger`'s *lands no approval can clear* as its second case gives it, with a `clm.toml` that doesn't parse in `c`'s first parent and a counted approval of `c`'s fingerprint, and `refs/clm/tiers-verified` at a commit `m` on `origin/main`'s first-parent chain before `c`
- WHEN `clm wrap-up` runs
- THEN it writes to standard error a line starting `clm wrap-up: ledger: ` and naming `clm.toml`, prints no `wrap-up ok` or `wrap-up incomplete` line, exits 2, and `refs/clm/tiers-verified` still names `m`

#### Serves
- capability/wrap-up-session
- architecture/humans-review-intent

### Requirement: checkpoint-reports-unsafe-work <!-- base: 82ac5307f0fcfe39 -->
`clm wrap-up` SHALL report, one line each, work CLM's flow makes that isn't safe on `origin`: a checkout on a `spec/*` or `impl/*` branch with uncommitted changes or untracked files that aren't ignored is *dirty*; a local `spec/*` or `impl/*` branch that isn't landed (`checkpoint`) is *unpushed* when it has commits its `origin` branch lacks, or no `origin` branch; and this clone's ledger branch is *unpushed* when it holds a record `origin` lacks. It SHALL print `dirty <path>` and `unpushed <branch>` for the session's own work: the checkout it runs in, the branch checked out there and the clone's ledger. It SHALL print `elsewhere dirty <path>` and `elsewhere unpushed <branch>` for the rest, a build's worktree in a `.claude/worktrees/agent-*` directory and its branch always among them. It SHALL also print `open <branch>` for each local `spec/*` or `impl/*` branch that is pushed but not landed, `complete <id>` for each change the queue reports complete on `origin/main`'s tree, the violations of every `clm check` rule on that tree and the findings of the ledger's audit (`checkpoint-review-mark`), each line starting `origin/main: ` before the violation's `<path>: <rule>: <message>` or the finding's `<rule>: <message>`. It SHALL report on that commit the violations `clm check` reports in a checkout of it, the tracked files under directories whose names start with `.` among them (`check/run-listed`). When `origin/main`'s specs don't load, it SHALL print no `complete` line and delete no `origin` `propose/<slug>` (`checkpoint`), and the load's violations make it incomplete. The `open`, `elsewhere`, `complete`, `freed`, `place` and `left` lines SHALL NOT make the wrap-up incomplete.

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

#### Scenario: main's specs don't load
- GIVEN an `origin/main` holding `specs/d/SPEC.md` with no frontmatter, and `origin/propose/q` where `backlog/q.md` isn't in `origin/main`
- WHEN `clm wrap-up` runs
- THEN it prints a line starting `origin/main: specs/d/SPEC.md: `, no `complete` line and `wrap-up incomplete`, exits 1, and `origin/propose/q` still exists

#### Serves
- capability/wrap-up-session
- architecture/git-is-truth

#### Needs
- check/run-listed

### Requirement: wrap-up-fetch-retries-race <!-- base: e4a57cf681acb6b4 -->
`clm wrap-up` SHALL fetch `origin` with `--prune`, trying the fetch again when it fails as `vcs/fetch-retries` states, so a wrap-up whose fetch lost a race with a build in another worktree of the same clone over a remote-tracking ref goes on. When the last try fails, it SHALL remove and delete nothing and exit 2, writing `clm wrap-up: <message>` to standard error: git's message from the last fetch when `origin` answers, and `origin can't be reached` when it doesn't.

#### Scenario: a wrap-up whose fetch loses a ref race
- GIVEN a clone with nothing to clean up, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/impl/a'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm wrap-up` runs
- THEN it prints `wrap-up ok` and exits 0

#### Scenario: a wrap-up whose fetch keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm wrap-up` runs
- THEN it writes to standard error a line starting `clm wrap-up: ` and containing `cannot lock ref`, doesn't print `wrap-up ok`, and exits 2

#### Scenario: a wrap-up that can't reach origin
- GIVEN a clone whose `origin` URL names a directory that no longer exists, and the landed local `impl/c` of `checkpoint`'s "merged without a record"
- WHEN `clm wrap-up` runs
- THEN standard error is the line `clm wrap-up: origin can't be reached`, standard output is empty, `impl/c` still exists, and it exits 2

#### Needs
- vcs/fetch-retries

#### Serves
- capability/work-concurrently

### Requirement: wrap-up-delete-loses-race <!-- base: 6344915bf3bae175 -->
When `origin` refuses a delete `clm wrap-up` makes of a done branch there (`checkpoint`: a landed `spec/*` or `impl/*` branch, an `autosync/spec/<id>` or a `propose/<slug>`) because the branch is no longer at the tip the fetch read, `clm wrap-up` SHALL read that branch on `origin` again and go on. A branch `origin` no longer has SHALL count as done, since another session or command deleted it after the fetch and nothing is left to delete, and a branch whose tip moved SHALL be kept, as the lease means it to be. It SHALL print no `removed` line for either; it SHALL still delete every other done branch on `origin` and print that branch's `removed` line, and the refusal SHALL NOT make it exit 2 or end `wrap-up incomplete`. When a delete fails and `origin` still has the branch at the tip the fetch read, the failure is git's and it SHALL exit 2, as for any git failure.

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

### Requirement: commands-in-crate <!-- base: 4746da3ab92797f9 -->
`clm-checkpoint` SHALL offer `clm wrap-up` as a call of its own that takes the command's arguments as the command line gives them, the repository root that `clm` found from the current directory, which `--root` (a repository root or any worktree of it) overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help` or `-h`, it SHALL write to the output stream the help `clm wrap-up --help` prints, line for line, write nothing to the error stream, and return 0: its first line is `Remove merged work, report work that isn't safe on origin, and say whether the session can end`, then an empty line, then `Usage: clm wrap-up [OPTIONS]`.

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `orchestrator.checkpoint`'s own files
- WHEN it calls `clm-checkpoint`'s call with the arguments of `clm wrap-up` and two buffers
- THEN the buffers hold what `clm wrap-up` prints to standard output and to standard error there, and the call returns the status `clm wrap-up` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm wrap-up` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: help
- GIVEN the same call
- WHEN a test passes it `--help`, and then `-h`
- THEN each time the output buffer holds the help `clm wrap-up --help` prints, its first line `Remove merged work, report work that isn't safe on origin, and say whether the session can end` and its third `Usage: clm wrap-up [OPTIONS]`, the error buffer is empty, and it returns 0

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
