---
status: applied
resolves:
  - releasing-a-dead-session-s-claim-leaves-the-build-loop-skipping
---

## Intent

Giving up a component's build becomes one command, `clm build release <component>`: it deletes the build's branch on `origin` and in the clone, with the worktree that has it checked out, frees its place and forgets its run, so the loop builds the component afresh once it is ready, with no step left for the session to do by hand. It refuses, changing nothing, what only the human can judge: a run that may still be alive, uncommitted work in the build's worktree, the branch checked out in the human's own checkout, and another clone's claim. The build skill's sentence that releases a claim with `clm branch delete` alone changes to name the command, through a deferred item on `plugin`, once the command is built.

## Problem

The build skill says to release a given-up build's claim by deleting `impl/<component>` on `origin` with `clm branch delete`. That leaves the clone's local `impl/<component>`, which `clm run` refuses, since a local branch with no `origin/impl/<component>` may hold commits nobody pushed. So every step of the loop skipped the component, and the session had to move the local branch out of `impl/` by hand, a step no skill states (`build-loop-in-clm`).

## Approaches

- **Taken: a loop command, `clm build release <component>`.** The human's word is recorded as `clm build cleared disk` records theirs, and the command does both halves in the safe order: it checks everything local first, then deletes the remote branch through `clm branch delete` (whose rule that a clone deletes only a tip it pushed stays `host`'s), then the worktree, the local branch and the place (through `clm run --finish`), and drops the run from the loop file, which would otherwise relaunch it as an orphan.
- **Not taken: `clm branch delete` also deleting the local branch.** Unattended sessions run it with no classifier judging it (`unattended-steps-run-clm`); it would gain the power to discard commits nobody pushed, and it knows nothing of the loop's runs or places.
- **Not taken: `clm run` resuming a local branch whose remote is gone.** It would drop `run`'s guard against building on commits nobody pushed, for every build, to serve the one case where the human has said to discard them.
- **Not taken: a longer sentence in the skill** (remove the worktree, then `git branch -D`). Prose a model follows, against `build-loop-in-clm`, and the skill tells the session never to run a command aimed at a run's worktree.

## Outcome

- `orchestrator.loop`: the Contract names `clm build release <component>`, the Design says why it is one command and what it refuses, and the requirement `release` states it.
- `backlog/the-build-skill-releases-a-claim-by-hand` (`kind: deferred`, `plugin`): once `orchestrator.loop/release` is current, the build skill's sentence names the command instead of `clm branch delete`. What a skill tells a session is source, not a requirement (`prompt-text-is-source`), and the loop's build can't edit `plugin`'s files (`path-ownership`), so a code branch on `plugin` does it.

## Constraints

- A release discards commits the build's branch holds that `origin` lacks: that is what giving the build up means, and it runs only on the human's word.
- A worktree with uncommitted work refuses the release, and is the human's to clear: `vcs`'s `remove_worktree` never forces a removal.

## Edge cases

- A failure after the remote delete (a worktree git can't remove) exits 2; running the release again finishes it, since `clm branch delete` of a branch already gone exits 0.
- A component with no branch anywhere and no run is released with exit 0, changing nothing.

## Open questions

None.
