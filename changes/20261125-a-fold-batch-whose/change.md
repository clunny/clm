---
status: applied
resolves: [a-fold-batch-whose-land-fails-on-main-having-moved-goes-back-to]
---

## Intent

When the land of an approved fold batch fails, the build loop brings the batch's branch up to date with `main` before it asks the human anything. The first failed land after an approval, unless `clm land` refused it on `gate/approval`, launches a `rebase:<batch>` run that continues `spec/<batch>`, runs `clm rebase` until `rebase ok`, resolving what it reports, and pushes. The loop then lands the batch again: it lands when the fingerprint is unchanged and the human's approval still holds, and otherwise goes back to the human's queue with a `say` that the batch was brought up to date and needs a fresh approval. A batch whose rebase run can't finish, or whose land fails again before the next approval, goes back to the human's queue with a `say` naming why, as every failed land did before.

## Problem

Between a batch's fold and its land, a component build landed a workaround flag on a requirement the batch modifies. The batch's own `clm check` and gate had passed, but `clm land`'s check of the merge with `main` failed (`this_repository_is_clean`: the flag names the requirement's old version). `plan-fold` sends every failed land back to the human's queue with `the land of fold batch <batch> failed (<line>); it is back in your queue`. The human had already approved it, and nothing in the queue says what to do. The fix is a `clm rebase` on the batch's branch, which is a session's work, after which the diff's fingerprint changes and the human must approve again. No run is launched for the rebase, so the batch waits on the human for work that belongs to a session, against `build-loop-in-clm`: the loop decides every launch.

## Approaches

- **Taken: one rebase run per approval, then land again.** `failed-batch-land-rebased` launches `rebase:<batch>` on the first failed land after an approval, whatever the land's reason, except `gate/approval`. The loop writes the run's prompt itself, as it writes the archive's (`plan-archive`). The run's kind is `fold`, so a rebase orphan is dropped and launched again as a fold orphan is (`loop-start`), and the session runs it as it runs a fold. The batch stays approved while the run goes, so its `proposed <batch> review` line makes the plan print `run land:<batch>` again, and `clm land` decides whether the approval holds. A land refused on `gate/approval` after a rebase run goes back to the human's queue with a `say` asking for a fresh approval. A land that fails again before the next approval goes back to the human's queue as `plan-fold` already says, so a failure no rebase mends never loops: every further round goes through the human's approval.
- **Not taken: rebasing only when the failure comes from the merge with `main`.** Telling that from the land's line would hold a reading of `clm land`'s reasons in the loop. A rebase run on a failure of the branch's own costs one session and ends in the same queue entry, with the run's own reason in the `say`.
- **Not taken: relaunching the fold.** The prompt `clm draft --fold` writes starts a new branch (`spec/<first-id>-fold-2`, by `fold`), so the pull request and the approval would move to another branch, and the drafts' branches it folds from are stale too.
- **Not taken: the rebase run lands the batch itself.** A fold session never lands, and the loop decides every land; landing again from the loop also tells the human when the approval still holds, with nothing asked of them.
- **Not taken: sending the batch to the human with a better `say`.** The human would still have to start a session for work the loop can launch.

## Outcome

- A new requirement, `orchestrator.loop/failed-batch-land-rebased`, and a new run name, `rebase:<batch>`, in the Contract's list of run names.
- `plan-fold`'s rule for a failed land defers to it, and its scenario *a fold that proposes nothing* now names a batch already rebased since its approval, keeping the outcome it had.
- The Design gains a paragraph, *A fold batch whose land fails*.

Edge cases the change knows it needs:
- A land whose line holds `gate/approval` before any rebase run launches none: the approval is missing, not the branch out of date, so only the human can act.
- A rebase that changes nothing leaves the fingerprint unchanged, and the second land lands on the human's approval.

## Constraints

- `build-loop-in-clm`: the loop decides the launch and the land; the rebase run only rebases and pushes.
- `approval-policy`: the run never runs `clm approve`, and a changed fingerprint always goes back to the human.
- `yagni`: one rebase per approval, no count setting, no reading of the land's reason.

## What the requirements no longer state

- `plan-fold`: "A land that returns any line but `landed ...` SHALL put the batch back in the human's queue" no longer holds for the first such land after an approval that isn't refused on `gate/approval`. That land launches `rebase:<batch>` instead (`failed-batch-land-rebased`). The scenario *a fold that proposes nothing* keeps its outcome under its name, with its batch now one already rebased since its approval.
- `plan-fold`: a land refused on `gate/approval` after a rebase run now passes a `say` asking for a fresh approval, where `plan-fold` passed none for a line holding `gate/approval`.

## Open questions

None.
