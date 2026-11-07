---
component: orchestrator.checkpoint
base: { design: "5d1cadc611ce4aba" }
---

## MODIFIED Design

Crate `clm-checkpoint`. Whether a branch landed comes from `clm-git`'s ancestry queries on `origin/main`, worktrees and branches from `clm-git`'s queries, and the complete changes from `orchestrator`'s queue on `origin/main`'s tree.

**`clm wrap-up`** (`checkpoint-before-done`). It fetches `origin` with `--prune`. A `spec/*` or `impl/*` branch is *landed* when its current tip is an ancestor of `origin/main` and isn't on `origin/main`'s first-parent chain: its commits reached `main` through a merge, `clm land`'s or a GitHub pull request's, so it needs no `landed` record. The test isn't "its tip is in `origin/main`", which a branch just created from `main` also passes: that branch's tip is a commit `main` itself was at, on the first-parent chain.

**Places** (`build-places`, `checkpoint-frees-places`). Right after the fetch, before any of the removals below, it frees the clone's places that don't count with one call of `free_stale_places`, never `places` then `free_place`: a `clm run` or `clm draft` could retake a place of that name between the two, and checkpoint would delete the fresh place. It judges them first because the removals below delete a landed build's local and remote `impl/<component>`, and a build place whose branches are all gone reads as not yet claimed, so it would count by its `taken` time instead of as landed. A place that still counts stays, since its build or draft may be running. Checkpoint has no `--expiry` and uses the default, 4 hours: when a session ran its builds with a longer expiry, checkpoint may free a place whose build is still running, which only lets one more build start on the clone than the cap.

Then:
1. For each build's worktree, a worktree on an `impl/*` branch whose directory is named `agent-*` and sits directly in a `.claude/worktrees/` directory, which is where Claude Code puts a subagent's worktree, and each local branch `spec/*` or `impl/*`: if its branch is landed and the worktree has no uncommitted changes, it removes the worktree with `vcs`'s `remove_worktree`, which unlocks it and makes it removable first, so a directory the build left without its owner's write permission doesn't stop it, and deletes the local branch, printing `removed <path>` and `removed <branch>`. When the worktree can't be removed (a file owned by another user, a file in use on Windows, a worktree git refuses), it prints `left <path>: <error>; delete the directory by hand, then run git worktree prune` in place of `removed <path>`, the error joined onto one line, as `clm run --finish` does, keeps that worktree's local branch, which the next wrap-up deletes once the directory is gone, and goes on. A build's worktree is never the session's own (3. below), and its branch has landed, so nothing on it is lost: a `left` line doesn't make the wrap-up incomplete or exit 2. A landed branch checked out in any other worktree, such as a human's session worktree, is left alone with its worktree. It also deletes each local branch `worktree-agent-*` that no worktree has checked out and whose tip is an ancestor of `origin/main`: the branch Claude Code made for a build, left behind when the build switched to `impl/<component>`. If the main checkout is on such a branch and has no uncommitted changes, it switches it to `main`, fast-forwards `main` to `origin/main`, and then deletes the branch. With uncommitted changes it leaves the checkout alone and reports it as `dirty`.
2. It deletes on `origin` each landed `spec/*` or `impl/*` branch `clm land` didn't already delete, leased on the tip it judged, and each `autosync/spec/<id>` whose change `<id>` is in `origin/main`'s tree, under `changes/<id>/` or `changes/archive/<id>/`, printing `removed origin/<branch>`. A live claim is a commit on top of `origin/main`, unlanded work has commits `origin/main` lacks, and a new branch's tip is on the first-parent chain, so none of them is ever deleted. A snapshot whose spec branch was abandoned stays. It also deletes each `origin/propose/<slug>` whose job is done: `backlog/<slug>.md` isn't in `origin/main`'s tree, or a change outside the archive there lists `<slug>` in its `resolves`, which holds the item (*on main*). An archived change doesn't count, so a claim made after it is kept, and checkpoint and `clm claim` agree. A claim whose spec work hasn't landed stays, live or lapsed.
3. It reports, one line each, work that isn't safe on `origin`, looking only at work CLM's flow makes: a checkout is *dirty* when it is on a `spec/*` or `impl/*` branch and has uncommitted changes or untracked files that aren't ignored; a local `spec/*` or `impl/*` branch that isn't landed is *unpushed* when it has commits its `origin` branch lacks, or no `origin` branch; a local ledger branch is *unpushed* when `append` left it so (offline work). Only the session's own work makes the wrap-up incomplete (`checkpoint-before-done`): the checkout it runs in, the branch checked out there, and this clone's ledger. Git doesn't record which session made a build's worktree, so a build's worktree and branch are never the session's own: `build` already has each build's outcome, a land or why it stopped, and checkpoint prints them as `elsewhere`. It prints the session's own as `dirty <path>` and `unpushed <branch>`. Everything else, such as another session's checkout or a branch not checked out in its own checkouts, it prints as `elsewhere dirty <path>` and `elsewhere unpushed <branch>`, for the human to see. Several sessions can then each finish while the others still work.
4. It reports `open <branch>` for each local `spec/*` or `impl/*` branch that is pushed but not landed.
5. It runs every `clm check` rule on `origin/main`'s tree, extracted as `clm next` does, and `clm ledger verify` against `origin/main`: without `--review` over the whole chain, as `clm check` runs it on `main`, and with the review recomputed (`ledger`'s `--review`, `checkpoint-verifies-review`) for the lands after the clone's *review mark* only (below): two calls of `ledger`'s `verify`, the first without `--review` and without `--since`, the second with both, keeping from the second only its `ledger/understated-land`, `ledger/unrated-land` and `ledger/unapproved-land` findings, the ones the recomputation can add, and printing the first call's findings and then the second's that the first didn't give. The walk with `--since` doesn't start at the oldest landed commit as the walk without does, so its `ledger/unrecorded-land` lines, which repeat the first call's or, in a repository with no `landed` record yet, name every commit after the mark, are the ones left out. It also prints `complete <id>` for each change the queue reports complete on that tree, so the `build` skill archives it (`archive-on-sync`).

**Review mark** (`checkpoint-review-mark`, `checkpoint-recomputes-new-lands`). Recomputing a land runs the gate on it, and a repository's signed lands only grow, so checkpoint doesn't recompute the whole history each time it runs. The *review mark* is the local ref `refs/clm/tiers-verified`, in the clone's git common directory like the build places, so every worktree of the clone shares it, and never pushed: the newest commit on `origin/main`'s first-parent chain up to which this clone has recomputed every signed land and found nothing to report. Checkpoint reads it with `git rev-parse --verify` and recomputes the lands after it, `clm ledger verify --review --since <mark>`, and the whole chain when the ref is unset or doesn't resolve to a commit, or its commit isn't an ancestor of `origin/main` (`clm-git`'s `is_ancestor`; a rewritten `main`), as a fresh clone's first checkpoint does. A mark a human set by hand on a commit that is an ancestor but off the first-parent chain makes the walk reach back to where that commit's branch forked, recomputing more than the lands after it and never fewer, which is fine. Just before it prints its last line, `wrap-up ok` or `wrap-up incomplete` alike, it sets the mark with `git update-ref` to the first parent of the oldest land after it, by the chain's order (`git rev-list --first-parent`, not the order `verify` returns its findings in), that got a `ledger/understated-land` or `ledger/unrated-land` finding from the recomputation, or a `ledger/unrecorded-land` finding from the audit of the whole chain, or to `origin/main` when none did: a finding is reported at every checkpoint until a human acts on it, and the recomputation covers only the lands from that one on, while a land found clean is never recomputed by this clone again. Only those three rules hold the mark: every other finding the audit without `--review` reports again regardless, and a land recomputed as needing review while recorded as not gets `ledger/understated-land` first, so its `ledger/unapproved-land` is never left behind the mark. A merge whose `landed` record hasn't reached the clone yet (its writer's ledger push came later, or wasn't fetched) gets only `ledger/unrecorded-land`, so it holds the mark and is recomputed once its record arrives; a commit never recorded, pushed around `clm land`, holds the mark for good, which `ledger/unrecorded-land` reports at every checkpoint anyway. Both git calls go through `clm-git`'s `git`, as the checkpoint's other git calls do. The mark is a cache derived from this clone's own earlier runs, which `git-is-truth` allows: nothing lands, approves or passes a check on it, deleting it (`git update-ref -d refs/clm/tiers-verified`) costs one full pass at the next checkpoint, and `clm ledger verify --review` with no `--since` is the full audit, which reads no mark. It is a ref rather than a file in `<git common dir>/clm/` because the mark is a commit: git refuses a value that isn't one, `git rev-parse --verify` reads it, `git update-ref` sets and deletes it, and `git fetch --prune` leaves it alone. Custom refs are what `per-writer-ledgers` rejected for shared state because no host shows or fetches them, which is right for a clone's own mark.

`open`, `elsewhere`, `complete`, `freed`, `place` and `left` lines alone don't make it incomplete. A git failure exits 2.

A branch squash-merged or fast-forwarded into `main` isn't landed by this test: a squash leaves its tip outside `main`, and a fast-forward puts it on the first-parent chain. It stays and is reported, which loses nothing. A local branch cut from another branch's tip that later landed, with no commits of its own, is landed and removed: nothing is lost, and the flow never cuts one branch from another (`no-stacked-prs`). A branch merged outside `clm land` is removed like any other landed branch, and `clm ledger verify` reports its merge as `ledger/unrecorded-land`, which makes the wrap-up incomplete. On a shallow clone whose history doesn't reach a branch's tip, the ancestry query fails and checkpoint exits 2, as for any git failure.

## MODIFIED Requirements

### Requirement: checkpoint <!-- base: f59546e3dfe6da5e -->
`clm wrap-up` SHALL fetch `origin` and treat a `spec/*` or `impl/*` branch as landed when its current tip is an ancestor of `origin/main` and isn't on `origin/main`'s first-parent chain, whether or not a `landed` record names it. It SHALL remove each worktree on an `impl/*` branch in a `.claude/worktrees/agent-*` directory whose branch is landed and that has no uncommitted changes, with `vcs`'s `remove_worktree`, so the worktree goes whole also when the build left a directory in it without its owner's write permission, and delete each local and `origin` `spec/*` or `impl/*` branch that is landed (leaving a worktree or main checkout with uncommitted changes in place), delete each `origin` `autosync/spec/<id>` whose change `<id>` is in `origin/main`, and delete each `origin` `propose/<slug>` whose `backlog/<slug>.md` isn't in `origin/main` or whose `<slug>` a change outside the archive in `origin/main` lists in `resolves`. When such a worktree can't be removed, it SHALL print `left <path>: <error>; delete the directory by hand, then run git worktree prune` in place of `removed <path>`, with the error on one line, keep that worktree's local branch and go on, and the line SHALL NOT make the wrap-up incomplete or exit 2. It SHALL delete each local branch `worktree-agent-*` that no worktree has checked out and whose tip is an ancestor of `origin/main`. For work in a checkout on a `spec/*` or `impl/*` branch, on local `spec/*` and `impl/*` branches that aren't landed, or on this clone's ledger branch, that isn't safe on `origin`, it SHALL print `dirty <path>` and `unpushed <branch>` when it is the session's own: the checkout it runs in, the branch checked out there and the clone's ledger. It SHALL print `elsewhere dirty <path>` and `elsewhere unpushed <branch>` for the rest. It SHALL also print `open <branch>` for pushed but unlanded branches, `complete <id>` for each change `queue_from_refs` reports complete on `origin/main`'s tree, `clm check`'s violations on that tree and `clm ledger verify`'s on `origin/main`. It SHALL end with `wrap-up ok` and exit 0 when there are no `dirty` or `unpushed` lines of its own and no violations, and `wrap-up incomplete` with exit 1 otherwise. It SHALL NOT delete a `spec/*` or `impl/*` branch whose tip isn't an ancestor of `origin/main` or is on its first-parent chain, call `gh`, or touch files outside the worktrees it removes other than switching a landed main checkout to `main` and deleting the clone's places that no longer count (`checkpoint-frees-places`).

#### Needs
- vcs/remove-worktree

#### Scenario: landed work cleaned up
- GIVEN `impl/a` checked out in a clean, locked worktree in `.claude/worktrees/agent-1`, and `spec/y` on `origin` with `autosync/spec/y`, both with `landed` records whose commits on `origin/main` have their tips as second parents, and `changes/y/` in `origin/main`
- WHEN `clm wrap-up` runs
- THEN the worktree, both local and remote branches and `autosync/spec/y` are gone, it prints `wrap-up ok` and exits 0

#### Scenario: unwritable directory in a build's worktree
- GIVEN the landed `impl/a` of "landed work cleaned up", whose worktree holds an ignored directory `cache/` with a file in it, `cache/` without its owner's write permission
- WHEN `clm wrap-up` runs
- THEN the worktree's directory is gone from disk and `git worktree list` doesn't name it, it printed `removed` with its path and `removed impl/a`, prints `wrap-up ok` and exits 0

#### Scenario: build's worktree git can't remove
- GIVEN the landed `impl/a` of "landed work cleaned up", whose worktree `git worktree remove` refuses or can't delete
- WHEN `clm wrap-up` runs
- THEN it printed a line starting `left ` with the worktree's path and `git worktree prune` and no `removed` line with that path, the local branch `impl/a` still exists, and it prints `wrap-up ok` and exits 0

#### Scenario: done backlog claims removed
- GIVEN `origin/propose/q` where `backlog/q.md` isn't in `origin/main`; `origin/propose/r` where `changes/y/change.md` in `origin/main` lists `r` in `resolves`; `origin/propose/s` where `backlog/s.md` is in `origin/main` and no change there names `s`; and `origin/propose/t` where `backlog/t.md` is in `origin/main` and only `changes/archive/z/change.md` names `t`
- WHEN `clm wrap-up` runs
- THEN `origin/propose/q` and `origin/propose/r` are gone, and `origin/propose/s` and `origin/propose/t` still exist

#### Scenario: live claim kept
- GIVEN `origin/impl/b`, a claim commit on top of `origin/main` pushed by another session
- WHEN `clm wrap-up` runs
- THEN `origin/impl/b` still exists

#### Scenario: new branch kept
- GIVEN `spec/n`, created from `origin/main` and pushed with no commits of its own, and no `landed` record for it
- WHEN `clm wrap-up` runs
- THEN `spec/n` still exists locally and on `origin`

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

#### Scenario: abandoned autosync kept
- GIVEN `origin/autosync/spec/q` and no `changes/q/` in `origin/main`
- WHEN `clm wrap-up` runs
- THEN `origin/autosync/spec/q` still exists

#### Scenario: another session's work
- GIVEN the main checkout on `spec/z` with an uncommitted change and a commit `origin/spec/z` lacks, and `clm wrap-up` run in a linked worktree `w` on `main`
- WHEN `clm wrap-up` runs in `w`
- THEN it prints `elsewhere dirty <main checkout>` and `elsewhere unpushed spec/z`, then `wrap-up ok`, and exits 0

#### Scenario: build's worktree is elsewhere
- GIVEN `clm wrap-up` run in a linked worktree `w`, and `.claude/worktrees/agent-1` on `impl/a` with an uncommitted change
- WHEN `clm wrap-up` runs in `w`
- THEN it prints `elsewhere dirty` with that worktree's path, then `wrap-up ok`, and exits 0

#### Scenario: build's abandoned branch
- GIVEN a local branch `worktree-agent-1` at `origin/main`'s tip that no worktree has checked out, and `worktree-agent-2` with a commit `origin/main` lacks
- WHEN `clm wrap-up` runs
- THEN `worktree-agent-1` is gone and `worktree-agent-2` still exists

#### Scenario: human's worktree kept
- GIVEN a clean worktree `.claude/worktrees/p3` on `impl/a`, with a `landed` record at its tip
- WHEN `clm wrap-up` runs elsewhere
- THEN `.claude/worktrees/p3` and the local branch `impl/a` are still there, and it prints neither as `removed`

#### Scenario: merged without a record
- GIVEN local `impl/c` with no `origin` branch and no `landed` record, whose tip is the second parent of a merge commit on `origin/main`'s first-parent chain, as a GitHub pull request merged it before the cutover
- WHEN `clm wrap-up` runs
- THEN `impl/c` is gone, it prints `removed impl/c` and no `unpushed` line for it, and exits 0

#### Scenario: old branch from main kept
- GIVEN local `spec/m` at an older commit on `origin/main`'s first-parent chain, with no commits of its own and no `landed` record
- WHEN `clm wrap-up` runs
- THEN `spec/m` still exists
