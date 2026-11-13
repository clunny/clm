---
status: done
resolves: [loop-wrap-up-fails-deleting-a-propose-branch-already-gone-from]
---

## Intent

A delete of a branch that is already gone from `origin` counts as done. `clm wrap-up` and `clm draft --finish` each delete done branches on `origin`, leased on the tip their fetch read. When another session or command deletes one of those branches between that fetch and the push, git refuses the lease as stale and the command exits 2, though nothing was left to delete. With several drafts and builds running beside the loop, that race is ordinary. After this change both commands read the refused branch again, skip it when it is gone or has moved, and go on; only a failure that leaves the branch where the fetch saw it is still a git failure.

## Problem

`clm build returned draft:<id> --result 'landed <sha>'` exited 2. It had recorded the return and finished the draft, then computed the next step, which runs `clm wrap-up`. Wrap-up deletes every done branch on `origin` in one push, each with `--force-with-lease` on the tip its fetch read: landed `spec/*` and `impl/*` branches, `autosync/spec/<id>` of changes on `main`, and `propose/<slug>` claims whose item is gone or held by a change. The draft's `propose/<slug>` had been deleted on `origin` after wrap-up's fetch, so git rejected that lease with `stale info`, the push failed, and wrap-up treated it as a git failure (`crates/clm-checkpoint/src/lib.rs`, "Landed branches, released snapshots and done backlog claims"). The loop's state was right, but the step's exit 2 reads to an adapter as the return not being recorded.

No requirement says what a refused lease means here. `orchestrator.checkpoint/checkpoint` says only that a git failure exits 2, so the fix needs a written rule (`code-fix-needs-a-written-rule`).

`clm draft --finish` has the same exposure one function away. It already skips a `propose/<slug>` claim that is gone or whose lease is refused (`draft-finish`), but its delete of `spec/<change>` and `autosync/spec/<change>` propagates any push failure (`delete_branches` in `crates/clm-triage/src/finish.rs`). Wrap-up deletes those same two branches for a landed change, so a wrap-up in one session and a finish in another race for them.

The item names `orchestrator.triage`; the failure it reports is in `orchestrator.checkpoint`. Both get a requirement, so the item's `components` need no correction.

## Approaches

1. **State the rule where each delete is made** (chosen). One added requirement per component: a refused lease is read again; gone counts as done, moved is kept, and neither fails the command. It is a new requirement in each rather than a rewrite of `checkpoint` or `draft-finish`, so the tests citing those two stay current and each build is the fix and its tests.
2. **Have both call `host`'s `branch_delete`**, which already treats a branch that is gone as done. Rejected for checkpoint: it has no dependency on `host`, its Contract says it reads git only, and it deletes all its branches in one push, where `branch_delete` is one push per branch. `orchestrator.triage` already depends on `host`, so its build may reuse `branch_delete` for the two branches if it fits (`reuse-first`); the requirement states only the behaviour. Note `branch_delete` returns a refusal for a moved branch, which the finish must turn into a skip.
3. **Accept it**, since the next step succeeds. Rejected: the exit code is what an adapter reads, and a rule already exists for the same race on claims in `draft-finish`.

## Outcome

- `clm wrap-up` no longer exits 2 when a branch it meant to delete on `origin` was deleted or moved after its fetch; it deletes the rest and ends as it otherwise would.
- `clm draft --finish` does the same for `spec/<change>` and `autosync/spec/<change>`.
- So `clm build returned` and `clm build step` don't fail on this race.

## Constraints

- A moved branch is kept: the lease still protects a push made since the fetch.
- A failure with the branch still at the tip the fetch read is git's, and still exits 2, so a real failure isn't hidden.
- No capability test changes: nothing at the product's boundary differs outside the race.

## Edge cases and workarounds

- One push deletes several branches in wrap-up. Without `--atomic` git updates the refs it can and reports the refused ones, so the others are deleted by the same push. The builder check confirmed it with real git: the push exits 1 and reports `! [rejected] (delete) -> <branch> (stale info)` for a branch that moved and for one already gone alike, also with `-q`.
- After such a push a branch absent from `origin` may be one the push deleted or one deleted before it, and reading `origin` again can't tell which. Wrap-up tells them apart by git's per-branch report of the push, so only the branches it deleted get a `removed` line.
- The tests need a `git` on `PATH` that changes `origin` just before the push. `crates/clm/tests/checkpoint.rs` already runs `clm wrap-up` with a fake program first on `PATH`; `crates/clm-triage/tests/fetch_retry.rs` does it in-process for the finish, and sets the process's `PATH` once, so the finish's race test needs a file of its own.
- In the finish's scenarios the fake `git` must call the real one for its own push, or it recurses.

## Open questions

None.
