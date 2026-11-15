---
status: applied
resolves: [a-batch-whose-main-moved-under-its-check-runs-again-from-nothing]
---

## Intent

Lands taken together keep the check they passed when `main` moves under it, as a land alone does: only the tests the lands and `main`'s new commits can both break run again, and the lands still reach `main` in one push. Today such a batch pushes nothing and every land in it runs a check of its own.

This finishes `20261113-land-starvation`, the landing work the human excepted from the plan's freeze on 2026-11-13 and again, for the lands taken together, on 2026-11-15. It adds no new intent: `land`'s Design and the decision `batched-lands` already say a batch keeps its check this way. No requirement said it, so the build left it out.

## Problem

`land`'s Design says "Lands taken together keep their check the same way, `P` being the last of their merge commits", and `docs/decisions/batched-lands.md` says a push from another clone costs a batch what it costs any land, the overlap re-check. The requirement `land/batched-lands` says nothing of a `main` that moved, and `land/overlap-recheck` speaks of one `clm land`'s own attempts.

So the build of `batched-lands` does this when `main` has moved by the time the batch takes the push turn: it keeps the `checked` record for the batch's tip, pushes nothing, the holder starts its attempt afresh and runs its own check, and the lands it took wait for their own turns and run theirs. A batch of three that loses the race costs four checks where a land alone that loses it costs one and an overlap.

It is not rare. A land with nothing to check takes only the push turn (`land/push-turn`), so it moves `main` under a batch's check on the same clone, and this repository's spec lands are mostly of that kind. The longer check a batch runs is the likelier to be overtaken.

## Approaches

- **Taken: one added requirement, `land/batch-keeps-its-check`.** Inside the push turn the holder asks `clm affected --overlap <P> <M>`, rebuilds the chain on the new `main`, judges each rebuilt merge commit as it judged the first, runs `clm check` on the rebuilt tip and `[land] check` with the overlap's selection when it printed a line, and pushes. The records are the ones a land alone that kept its check writes.
- **Not taken: accept the behaviour and correct the Design.** The freeze's default answer for a defect. It would take back an outcome the human approved in `20261113-land-starvation` ("a land runs its check once, and after that at most the tests its changes and `main`'s new commits can both break, however many lands the clone runs at once"), and the cost falls on the busiest sessions.
- **Not taken: modify `batched-lands` or `overlap-recheck`.** Either already holds more than one rule, and every test citing it would fall `behind` for a rule none of them proves.
- **Not taken: rebuild with the lands that still merge, leaving out one the new `main` refuses.** The check ran on a tree holding every land taken. A tree without one of them is a tree no check ran on, so the batch gives way instead.
- **Not taken: finding suspects when the overlap's check fails.** The overlap is small and its failure rare. Every land taken is then a suspect and `failed-batch` goes on from there, with no new rule for parts of an overlap.

## Outcome

- When `main` moved while the check of lands taken together ran, the lands reach `main` in one push after at most an overlap check, each with its own merge commit, `landed` record and `landed` line.
- When the rebuilt chain can't stand for what was checked (a land the new `main` refuses or conflicts with, a `main` whose code no `checked` record names, a changed check environment, a failed `--overlap`, or `main` moving once more before the push), the batch pushes nothing and its lands go on as they do today: the holder alone at once, the others on their own turns.
- When the overlap's check fails, every land taken is a suspect of a failed batch.
- `land`'s Design gains the detail the build needs. Its Contract is unchanged, since it already says a land whose check passed runs again only the overlap.

## Constraints

- `clm land` stays fast (`land-checks-stay-fast`): the rule adds one `clm affected --overlap`, the chain's gates and one `clm check`, only when `main` moved, in place of a full check per land.
- Every commit a push leaves `main` at is still a checked tree (`land-locally`): the rebuilt tip's code key gets a composed `checked` record before the push, and every rebuilt commit's `Checked:` line names that key, so `unrecorded-land-finished` and `clm ledger verify` read the push as they read any batch's.
- No change to `ledger`, `check.affected` or `gate`: `composed-check`, `landed-recheck` and `overlap-selection` are current and are used as a land alone uses them.

## Edge cases

- A land the rebuilt chain would have to leave out: the batch gives way, as above.
- `main` moves again between the re-check and the push, which only a push from another clone or an older `clm` can do inside the push turn: the batch gives way. A land alone goes round again; a batch doesn't, so the holder's turn stays short.
- The rebuilt tip's code key already counts as checked: no composed record is appended.
- Lands taken together again after a failed batch, or as a half, are lands taken together: the rule holds for them too.
- A chain of one (a land taken alone after others were left out) is not a batch and lands as `overlap-recheck` says.

## Open questions

- **Whether this is inside the landing exception to the freeze.** The session read it as inside: it completes a change the exception names, to that change's own Design. The human's approval of this change is the answer; a no leaves the item in the backlog.
