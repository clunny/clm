---
status: applied
resolves:
  - releasing-a-dead-session-s-claim-leaves-the-build-loop-skipping
---

## Intent

Giving up a component's build becomes one command, `clm build release <component>`: it deletes the build's branch on `origin`, renames the clone's `impl/<component>` to `released/<component>/<time>`, keeping its commits and its worktree for the human, frees its place and forgets its run, so the loop builds the component afresh once it is ready, with no step left for the session to do by hand. It refuses, changing nothing, while the build's run may still be alive or when the claim is another clone's. The build skill's sentence that releases a claim with `clm branch delete` alone changes to name the command, through a deferred item on `plugin`, once the command is built.

## Problem

The build skill says to release a given-up build's claim by deleting `impl/<component>` on `origin` with `clm branch delete`. That leaves the clone's local `impl/<component>`, which `clm run` refuses, since a local branch with no `origin/impl/<component>` may hold commits nobody pushed. So every step of the loop skipped the component, and the session had to move the local branch out of `impl/` by hand, a step no skill states (`build-loop-in-clm`).

## Approaches

- **Taken: a loop command, `clm build release <component>`, that sets the local branch aside.** The human's word is recorded as `clm build cleared disk` records theirs. The remote delete goes through `clm branch delete`, whose rule that a clone deletes only a tip it pushed stays `host`'s, and the place through `clm run --finish`, whose rules stay `orchestrator.run`'s. Renaming the local branch out of `impl/` is what the session did by hand; it discards nothing the clone holds, so the command needs no guard beyond the allow rule `clm build` already has, and it copies none of `run-finish`'s worktree rules into the loop. The run is dropped from the loop file, which would otherwise relaunch it as an orphan.
- **Not taken: deleting the local branch and its worktree in the release.** It would discard commits nobody pushed and uncommitted work under an allow rule no classifier judges, needing a guard asking the human (an audit finding), and it would copy `orchestrator.run`'s rules for which build worktrees may be removed into the loop (`build-loop-in-clm`, another audit finding).
- **Not taken: `clm branch delete` also deleting the local branch.** Unattended sessions run it with no classifier judging it (`unattended-steps-run-clm`); it would gain the power to discard commits nobody pushed, and it knows nothing of the loop's runs or places.
- **Not taken: `clm run` resuming a local branch whose remote is gone.** It would drop `run`'s guard against building on commits nobody pushed, for every build.
- **Not taken: a longer sentence in the skill** (rename the branch by hand). Prose a model follows, against `build-loop-in-clm`.

## Outcome

- `orchestrator.loop`: the Contract names `clm build release <component>`, the Design says why it is one command and why it sets the branch aside, the requirement `release` states it, and `depends_on` names `host`, whose `clm branch delete` it runs.
- `backlog/the-build-skill-releases-a-claim-by-hand` (`kind: deferred`, `plugin`): once `orchestrator.loop/release` is current, the build skill's sentence names the command instead of `clm branch delete`. What a skill tells a session is source, not a requirement (`prompt-text-is-source`), and the loop's build can't edit `plugin`'s files (`path-ownership`), so a code branch on `plugin` does it.

## Constraints

- A release leaves a `released/<component>/<time>` branch, and the dead session's worktree checked out on it, in the clone until the human deletes them. Nothing cleans them up: deleting them is discarding work, which is the human's call.

## Edge cases

- A failure after the remote delete exits 2; running the release again finishes it, since `clm branch delete` of a branch already gone exits 0.
- A component with no branch anywhere and no run is released with exit 0, changing nothing.

## Open questions

None.
