---
status: applied
priority: P1
from: backlog/queued-lands-each-run-a-full-check
resolves: [a-land-with-a-full-check-to-run-can-lose-every-attempt-to-lands]
---

## Intent

Landing is what holds a build session up: a land whose check finishes after `main` has moved throws the check away and starts again, and every land that needs a check runs its own, one after another. This change is built in two steps. Step one keeps every check that passed and gives each land a short turn to push in, so no land can be starved and a land with nothing to check still lands in seconds. Step two lets the lands waiting for a check on a clone share one. It also edits `land-locally`: once lands share a check, every commit a push leaves `main` at is a checked tree, but not every commit behind it.

## Problem

On 2026-11-13 one build loop ran four builds and drafts at a time on one clone for about 100 minutes.

- **A land lost 8 times in a row.** The draft `20261113-a-spec-branch-can` was ready at 13:15 and landed at 14:35. Its land ran its check at least 8 times (156 to 213 seconds each) and threw each away, because `main` had moved by the time it pushed. The ledger records 181 seconds for it, the attempt that won. The `model` build's land took 53 minutes the same way.
- **A waiting land checks a stale merge.** A land whose code key counts as checked but whose readers select a test builds its merge commit first and takes the land lock afterwards. When it had to wait, the land it waited for has usually moved `main`, and it checks the merge it built before waiting, so its push is refused. A land of unchecked code prepares again after waiting; this one doesn't, and no requirement states either.
- **Lands that take no lock move `main` under the one that holds it.** A land with nothing to run takes no lock (`land/land-target-lock`), so it pushes `main` while another land is in the middle of its check.
- **A check that passed is forgotten.** After a refused push the next attempt merges the branch again from scratch. `land/overlap-recheck` spares it the full check only when the branch's own head was checked whole beforehand, and never on the readers' path.
- **Each waiting land runs its own check.** With two or three lands waiting, `main` took one land every 6 to 12 minutes, though a single check takes 2 to 4.5 minutes (`backlog/queued-lands-each-run-a-full-check`). The human watches lands wait and wants them batched (2026-11-13).

This is one of the root causes the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims` names, parallel land checks on one loaded machine, and the human broke the plan's freeze for it (2026-11-13).

## Outcome

Step one:
- A land runs its check once, and after that at most the tests its changes and `main`'s new commits can both break, however many lands the clone runs at once.
- A land with nothing to check still lands in seconds, never behind a full check.
- A land that waited for the land lock checks a merge with `main` as it is after the wait.

Step two:
- The lands waiting for a check when one takes its turn land together on one check, each as its own merge commit with its own `landed` record and `landed <sha> <kind>` line.
- When that check fails, the lands the failure can't be traced to are taken together again at once, and the rest land alone, or in halves when there are more than four. A branch is told it failed only by its own `clm land`, with the output it would have had landing alone.
- A land killed between its push and its records is finished by the next `clm land` of any branch that push held.
- Build sessions on other machines are untouched: the locks and queues are one clone's, so lands are taken together only within a clone, and a push from another clone costs a land the overlap re-check of step one.

## The two steps

`clm land` is the one door every session uses, the builds that would fix it included, so a defect in it stalls everything. The smaller step goes first and is complete and useful without the second.

| | Requirements | Count |
|---|---|---|
| Step one | `land/push-turn` (added); `land/land-target-lock`, `land/lock-queue` and `land/overlap-recheck` (modified) | `land`: 1 added, 3 modified |
| Step two | `land/batched-lands`, `land/failed-batch`, `land/unrecorded-land-finished` (added) | `land`: 3 added |

`land`'s Contract and Design are edited for both. `ledger` gets one sentence of its Contract reworded and no requirement, since no reader of a `landed` record's `code_key` assumes it is the landed commit's own. `gate`, `check.affected` and `verify` are unchanged: `clm affected --overlap` already prints what two changes can both break, readers included.

**How step two waits.** A `#### Needs` block can't order the steps: a need names a requirement of another component, and all seven are `land`'s. Instead the question item `backlog/whether-lands-still-wait-on-each-other-s-checks-is-unmeasured` blocks the three step-two requirements (`requirement-blocks`), so `land`'s build builds step one and leaves them pending until the human deletes the item.

**The measurement.** Step two is built after step one has run on this clone for build sessions holding at least 50 lands that ran a check. From their `landed` records, the *busy share* is the sum of their `check_seconds` over the wall time from the first to the last of them in each session. Under one half, a land waits less than half a check on average, about 90 seconds at today's check times, and batching saves little at that load; the human may still build it for more builds at once, where the wait grows fast (about 2.5 minutes at a busy share of 60%, about 15 at 90%). On 2026-11-13 the checks that won summed to 1,426 seconds over 79 minutes, a busy share of 30%, and the discarded ones, which step one removes, roughly doubled that.

## A batch that fails

The human's condition (2026-11-13): a failing batch must not bring a lot of waiting and bisecting. Two signals the holder already has decide what it does, with no extra check to find them:

1. **Which step failed.** A check that marks its steps says which was open when it failed, and each step names the selected paths it ran for. A land whose own part of the selection holds none of those paths isn't a suspect. The lands that aren't suspects are taken together again at once.
2. **How many suspects.** Four or fewer land alone, one after another, as they would today. More than four are halved. With one bad land among `N`, landing alone costs `N + 1` checks and halving about `1 + 2 log2 N`; they break even near four.

So with the two or three lands that wait together today, a failed batch costs one wasted check and then exactly what landing costs now. A check that marks no steps makes every land a suspect, and the size rule alone applies.

Left out, and recorded in `backlog/a-flaky-test-that-fails-lands-taken-together-looks-like-a-real-failure`: running a failed batch a second time to tell a flaky test from a real failure. It pays only when flaky failures are more than about one in `N` of batch failures, and nothing counts failed land checks today.

## Why each added requirement isn't a scenario on an existing one

- **`push-turn`.** No requirement governs a land that runs no check, or the time between another land's check and its push: `land-target-lock` says such a land takes no lock. The turn is a second rule with its own holder, order and waiting line.
- **`batched-lands`.** Every existing requirement has a `clm land` check and push its own branch. That one land's process lands others' branches is a new rule, not a case of `land-target-lock` or `lock-queue`, which only say who runs next.
- **`failed-batch`.** What a failing check does to the lands taken together has outcomes of its own (who is a suspect, who lands, who waits, who is told), and `batched-lands` is already as long as one requirement should be.
- **`unrecorded-land-finished`.** `land` says nothing today of a head `main` already holds: landing it again makes a second, empty merge and leaves the first unrecorded. It can't sit under `batched-lands`, since it also covers a land alone.

The waiting land that prepares again and the kept check are not new requirements: they are sentences and scenarios added to `land-target-lock` and `overlap-recheck`.

## Approaches

1. **A push turn and a kept check first; then the turn holder lands the waiting lands, each as its own merge commit (chosen).** As above.
2. **Every land takes a turn, with or without a check.** Simple, and nothing on the clone moves `main` under a running check. But a land with nothing to check, which lands in seconds today, would wait for a whole check. The human asked for a way to let those lands through without starving the rest.
3. **One merge commit for a batch, every branch a parent.** Every commit of `main` stays a checked tree and `land-locally` needs no edit. But `clm verify` could then blame only a batch, and one branch couldn't be reverted alone. The human gave up "every commit is a checked tree" instead, as moving slow tests out of the land already had (2026-11-13).
4. **The build loop lands for its runs.** `clm build` would take finished branches and batch them. Propose sessions, fix branches and other sessions still land with `clm land`, so two landing paths would have to agree on one `main`.
5. **Check several combined trees at once** (speculative, as Zuul does). Rejected on 2026-11-08: it needs as many builds at once as there are waiting lands, and six parallel builds had filled the disk.

## What a land's verdict rests on

A land's verdict is composed from more partial checks than before: its own check on an earlier `main`, the checks of what `main` gained, an overlap between them, and in step two a selection joined from several lands' parts. All of it rests on `land-checks-stay-fast`, a principle: no program checks that a test reads only what the spec graph or a `clm:reads` marker connects it to. When that fails, a composed verdict passes a tree a full check would have failed.

What catches it, and how soon, on this repository:

- **A build's own check, within one build.** Every build runs `clm land --check-only` before it lands, which runs every test on its branch merged with `main`. A broken `main` fails the next build's check, about 10 to 20 minutes later while builds run. It blames that build, not the land that broke `main`.
- **`[verify]`, within 20 lands or when the queue drains.** This repository sets `every = { lands = 20 }` and `triggers = ["queue-drained"]`. Its bisect names the land, each land having its own merge commit, and `verify/culprit-recheck` says when the culprit's verdict was composed.

That is not enough by itself to trust on a day when builds land all day: 20 lands is about two hours. Step one composes verdicts in one more case than today, so it adds exposure of a kind the repository already carries, not a new kind. Tightening `[verify]` is a settings change the human can make at any time, with no change to this design: `every = { lands = 5 }` would bound a wrong verdict to about half an hour at today's rate, for one more full check per five lands. The plan's own work on what a passing check proves is the real fix for the principle being unchecked.

## Human layer

- **`land-locally` is edited, so this branch needs the human's review.** It said `clm land` "runs the checks on the merged tree"; with lands taken together only the last merge commit of a push is a tree the check ran on. The entry now adds: "Lands waiting on one clone may share one run of the checks, on the tree that holds them all, so every commit a push leaves `main` at is a checked tree, though not every commit behind it (`batched-lands`)." The human accepted this on 2026-11-13.
- `approval-policy` and the `land-safely` capability hold as written: each branch keeps its own approval, merge commit and `landed` record, and `main` is never left at an unchecked tree.

## Constraints

- `git-is-truth`: a waiting land learns its outcome from the ledger's `landed` record and from `main`. The queues' tickets stay a clone's own scheduling state under `.clm/`.
- `humans-review-intent`: the gate runs on each land's own merge commit in a batch, since a land merged before it can change whether it needs review.
- `deterministic-enforcement`: `clm ledger verify` still finds every land recorded and checked, a land killed after its push included (`land/unrecorded-land-finished`).
- `three-kinds-of-rule`: every requirement here is checked, and proven by a test of `clm land`'s behaviour against a repository and a check script. None is satisfied by text in a prompt or a skill, and no prompt or skill changes.
- `upgrades-carry-migrations`: no record gains a field and no file `clm init` writes changes, so no migration is needed. An older `clm` reads the new `landed` records as any other, never reads a ticket's content, and takes no push turn: its lands can still move `main` under a turn, and the land that holds it then goes on as for a push from another clone.
- `method-in-the-plugin`, `plain-terms`: the guide's page on landing says what step one changes, and the glossary defines the push turn. What the guide says of lands taken together is added when step two is built; the question item says so.
- `yagni`: no setting for how many lands are taken together or for the threshold of four.
- No capability test observes one check per land. `land_safely.rs`'s `a_branch_that_breaks_main` and `slow-tests-after-land`'s land over its target each land alone and must still observe the check and the timing they assert; `overlap-recheck` and `batched-lands` name both.

## Decisions

- **`a-passed-check-is-kept`** (new): the push turn, the overlap rule applied to the check a land just ran, and why lands with nothing to check keep landing at once.
- **`batched-lands`** (new): the land-lock holder lands the waiting lands on one check; what a failed batch does; what `main` no longer promises; a killed holder; why it waits for step one.
- **`landed-record-names-its-code-key`** (amended): a land taken with others names the key its check ran on.

## Open questions

- **When to build step two** is the question item's, for the human.
- **`backlog/overlap-checks-queue-in-the-ordinary-lane`** stays open. Step one runs an overlap re-check inside the push turn, where it waits behind no full check, but a land whose first attempt runs an overlap still queues in the ordinary lane. Step two settles it in practice, since such a land joins the lands ahead of it; the item is left for then.
- **`land/land` is left as it is.** It says a land's `landed` record names its merged tree's code key, which `batched-lands` changes for a land taken with others and says so in its own text. Rewording `land/land` now would make step one's build re-prove it against a rule that isn't built.
- **`P1` and the push turn.** `ordered-lock-queue` puts `P1` first on the land and check locks. The push turn goes in arrival order, because a turn lasts seconds and a land that finished its check must not lose it again and again.
