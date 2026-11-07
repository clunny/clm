---
status: applied
resolves: [stopped-build-with-unpushed-commits-blocks-its-retry]
---

## Intent

A build that stops after a `clm rebase` and before its push leaves committed work that its retry picks up with no hand work: `clm run <component>` resumes a local `impl/<component>` whenever the local branch has held `origin/impl/<component>`'s tip, the test git's `--force-if-includes` makes and `clm branch push` already passes, not only when it still contains that tip. The retry then removes the stopped build's clean, locked worktree as any resume does, and the new build rebases, pushes and lands the same branch. Decision: `resume-what-push-accepts`. Resolves `backlog/stopped-build-with-unpushed-commits-blocks-its-retry`.

## Problem

`clm rebase` rebases an `impl/*` branch onto `origin/main`, rewriting every commit on it, the claim commit included; the build pushes after it with `clm branch push`, whose `--force-if-includes` accepts the rewrite because the local branch's reflog holds the old tip. A build that stopped between the rebase and the push (its `clm land --check-only` was killed for low memory) left a local branch that no longer contained `origin/impl/<component>`. `clm run --finish` kept the branch, the claim and the worktree, as it should. The retry's `clm run` then refused: its resume rule asks that the local branch contain `origin/impl/<component>`, and otherwise exits 1 with "reconcile or delete it". The build skill treats exit 1 as "tell the human, launch nothing", and names no way to reconcile. Recovery took a hand `clm branch push` from the stopped build's worktree, then unlocking and removing that worktree.

The refusal guards against two things: commits on the local branch nobody pushed that a fresh claim would discard, and commits on `origin` the local branch lacks that the build's next push would overwrite. A rewrite of the clone's own pushed tip is neither: the local branch holds that tip's work, and `clm branch push` will accept it. So the rule was stricter than the push it protects.

## Approaches

- **Resume what `clm branch push` would accept (chosen).** The local branch has held `origin/impl/<component>`'s tip: the tip is an ancestor of the branch's tip or of an entry of its reflog. Reuses git's own test and the one `clm branch push` already makes (`reuse-first`), so `clm run` and the resumed build's first push agree by construction. Containment stays the common case.
- **`clm run --finish` pushes a stopped build's branch.** Rejected: the step that frees a place would start writing to `origin` for a build that may have stopped part-way, from the orchestrating checkout rather than on the checked-out branch `clm branch push` works on. The resumed build pushes the same branch at its first rebase point anyway.
- **Accept only a rebased claim commit, by structure.** Rejected: a build that had pushed work and then rebased rewrote that work too, which a structural test of the claim alone would still refuse.
- **Teach the build skill to reconcile.** Rejected: the orchestrating session may not run git in a build's worktree, and `clm run` can judge the case itself. With the fix, the exit 1 that remains is a branch that has never held `origin`'s tip, which only the human can reconcile, so the skill's existing rule (tell the human what it printed, launch nothing) is right and the plugin needs no change.

## Outcome

- `orchestrator.run/run`: resumes on the held-tip test; a new scenario, *stopped after a rebase*, covers the reported case, locked worktree included; the *unpushed leftover branch* scenario says its branch never held `origin`'s tip, so it stays a refusal.
- `orchestrator.run`'s Contract says a stopped build keeps its branch, pushed or not; its Design states the rule and why it matches `clm branch push`.
- No `plugin` delta: the item named `plugin` for the skill's handling of the refusal, which the fix makes unnecessary for this case.

## Edge cases and workarounds

- A clone with reflogs off (a bare repository's default) can't show the tip was held, so a rewritten branch is refused as before; its build's own push after a rebase fails the same way. Accepted (`resume-what-push-accepts`).
- A reflog entry git has expired no longer counts; a retry comes long before.
- A local branch with work and no `origin/impl/<component>` is still refused.

## Constraints

`lease-claims`: a branch whose `origin` tip this clone never held is still refused, so another clone's claim or work is never resumed or overwritten. `builds-in-subagents`: the orchestrating session runs no git of its own for this. `redesign-before-workaround`: this aligns the resume rule with the push rule rather than adding a recovery step.

## Open questions

None.
