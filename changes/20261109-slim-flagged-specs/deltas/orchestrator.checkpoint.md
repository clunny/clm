---
component: orchestrator.checkpoint
base: { design: "98603b9fb9f0293a" }
---

## MODIFIED Design
Crate `clm-checkpoint`. Whether a branch landed comes from `clm-git`'s ancestry queries on `origin/main`, worktrees and branches from `clm-git`'s queries, and the complete changes from `orchestrator`'s queue on `origin/main`'s tree.

**Landed** (`checkpoint`). A branch whose tip is an ancestor of `origin/main` off its first-parent chain reached `main` through a merge, `clm land`'s or a GitHub pull request's, so it needs no `landed` record. The test isn't "its tip is in `origin/main`", which a branch just created from `main` also passes: that branch's tip is a commit `main` itself was at, on the first-parent chain.

**Places** (`checkpoint-frees-places`). The places are judged before any removal because the removals delete a landed build's local and remote `impl/<component>`, and a build place with no branches reads as not yet claimed, so it would count by its `taken` time instead of as landed. One call of `free_stale_places`, never `places` then `free_place`, since a `clm run` or `clm draft` could retake a place of that name between the two, and checkpoint would delete the fresh place. Checkpoint has no `--expiry`: when a session ran its builds with a longer expiry than the default, checkpoint may free a place whose build is still running, which only lets one more build start on the clone than the cap.

**Removals** (`checkpoint`). A build's worktree is the one in `.claude/worktrees/agent-*` because that is where Claude Code puts a subagent's worktree. Its branch has landed and it is never the session's own, so nothing on it is lost when it can't be removed: the `left` line hands the directory to the human, and the local branch stays for the next wrap-up to delete once the human has deleted the directory. No `dirty` line names that directory, since git may not know it as a worktree. A `worktree-agent-*` branch is the one Claude Code makes for a build, which the build leaves when it switches to `impl/<component>`. A live claim is a commit on top of `origin/main`, unlanded work has commits `origin/main` lacks, and a new branch's tip is on the first-parent chain, so the landed test deletes none of them; a landed branch on `origin` is deleted leased on the tip checkpoint judged, so a push after the fetch keeps it. An archived change holds no `propose/<slug>` claim, so a claim made after it is kept, and checkpoint and `clm claim` agree.

**Unsafe work** (`checkpoint-reports-unsafe-work`). Only the session's own work counts against the wrap-up so that several sessions can each finish while the others still work. A build's worktree and branch count as elsewhere because git doesn't record which session made a build's worktree, and `build` already has each build's outcome, a land or why it stopped.

**Review mark** (`checkpoint-review-mark`, `checkpoint-recomputes-new-lands`). Recomputing a land runs the gate on it, and a repository's signed lands only grow, so checkpoint doesn't recompute the whole history each time it runs. The mark is in the clone's git common directory like the build places, so every worktree of the clone shares it. `ledger`'s `verify` is called twice, the second time with `--review` and `--since`; only the findings the recomputation can add are taken from the second call, since its `ledger/unrecorded-land` lines repeat the first's, or, in a repository with no `landed` record yet, name every commit after the mark. A finding holds the mark so that it is reported at every checkpoint until a human acts on it, while a land found clean is never recomputed by this clone again; a land recomputed as needing review while recorded as not gets `ledger/understated-land` first, so its `ledger/unapproved-land` is never left behind the mark. A merge whose `landed` record hasn't reached the clone yet gets only `ledger/unrecorded-land`, so it holds the mark and is recomputed once its record arrives. The mark is a cache derived from this clone's own earlier runs, which `git-is-truth` allows: nothing lands, approves or passes a check on it, deleting it (`git update-ref -d refs/clm/tiers-verified`) costs one full pass at the next checkpoint, and `clm ledger verify --review` with no `--since` is the full audit, which reads no mark. It is a ref rather than a file because the mark is a commit: git refuses a value that isn't one, and `git fetch --prune` leaves it alone. Custom refs are what `per-writer-ledgers` rejected for shared state because no host shows or fetches them, which is right for a clone's own mark.

A branch squash-merged or fast-forwarded into `main` isn't landed by this test: a squash leaves its tip outside `main`, and a fast-forward puts it on the first-parent chain. It stays and is reported, which loses nothing. A branch merged outside `clm land` is removed like any other landed branch, and `clm ledger verify` reports its merge as `ledger/unrecorded-land`, which makes the wrap-up incomplete.

## MODIFIED Requirements
### Requirement: checkpoint-frees-places <!-- base: b8507909ad1d954e -->
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
