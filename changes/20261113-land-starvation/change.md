---
status: proposed
priority: P1
from: [queued-lands-each-run-a-full-check]
resolves: [a-land-with-a-full-check-to-run-can-lose-every-attempt-to-lands, overlap-checks-queue-in-the-ordinary-lane]
---

## Intent

Landing is what holds a build session up: every land that needs a check runs its own, one after another, and a land whose check finishes after `main` has moved throws the check away and starts again. This change makes the lands waiting on a clone share one check, lets a land with nothing to check keep landing at once without costing anyone else a check, and keeps every check that passed, so no land can be starved however busy the clone is.

## Problem

On 2026-11-13 one build loop ran four builds and drafts at a time on one clone for about 100 minutes.

- **A land lost 8 times in a row.** The draft `20261113-a-spec-branch-can` was ready at 13:15 and landed at 14:35. Its land ran its check at least 8 times (156 to 213 seconds each) and threw each away, because `main` had moved by the time it pushed. The ledger records 181 seconds for it, the attempt that won. The `model` build's land took 53 minutes the same way.
- **A waiting land checks a stale merge.** A land whose code key counts as checked but whose readers select a test builds its merge commit first and takes the land lock afterwards. When it had to wait, the land it waited for has usually moved `main`, and it checks the merge it built before waiting, so its push is refused. A land of unchecked code prepares again after waiting; this one doesn't, and no requirement states either.
- **Lands that take no lock move `main` under the one that holds it.** A land with nothing to run takes no lock (`land/land-target-lock`), so it pushes `main` while another land is in the middle of its check. The comment on the push says the lock keeps other lands out until the push; it keeps out only the lands that wait for it.
- **A check that passed is forgotten.** After a refused push the next attempt merges the branch again from scratch. `land/overlap-recheck` spares it the full check only when the branch's own head was checked whole beforehand, and never on the readers' path.
- **Each waiting land runs its own check.** With two or three lands waiting, `main` took one land every 6 to 12 minutes, though a single check takes 2 to 4.5 minutes (`backlog/queued-lands-each-run-a-full-check`).

## Outcome

- However many lands a clone runs at once, a land or a batch of lands runs its check once, and after that at most the tests its changes and `main`'s new commits can both break.
- The lands waiting for a check when one takes its turn land together on one check.
- A land with nothing to check still lands in seconds, never behind a full check.
- Each branch still lands as its own merge commit with its own `landed` record, pull request and `landed <sha> <kind>` line, and a branch that fails is told so by its own `clm land`, with the output it would have had landing alone.

## Approaches

1. **The turn holder lands the batch, and a passed check is kept (chosen).** Below.
2. **Every land takes a turn, with or without a check.** Simple, and nothing on the clone moves `main` under a running check. But a land with nothing to check, which lands in seconds today, would wait for a whole check. The human asked for a way to let those lands through without starving the rest, which approach 1's overlap rule and push lock give.
3. **The build loop lands for its runs.** `clm build` would take finished branches and batch them. Propose sessions, fix branches and other sessions still land with `clm land`, so two landing paths would have to agree on one `main`.
4. **Check several combined trees at once** (speculative, as Zuul does). Rejected on 2026-11-08: it needs as many builds at once as there are waiting lands, and six parallel builds had filled the disk.

## Design

### Two locks, two jobs

- **The land lock** (`.clm/land.lock`, as now) is the turn to run a check in the land tree. Only a land that will run `[land] check` takes it, in the order `land/lock-queue` sets.
- **The push lock** (new, `.clm/push.lock`, with a queue of its own in arrival order) is held by every land of the clone from its last fetch of `main` until its push has gone through: the fetch, the merge, the gate on the merge commit, `clm check` and the push. A land with nothing to check takes only this lock, for the seconds its attempt takes. A land that ran a check takes it after the check passes, while still holding the land lock.

A land takes the land lock first and the push lock second, never the other way, and the check lock (`.clm/check.lock`) last, as now. Nothing on the clone moves `main` while a land holds the push lock, so a land inside it loses only to a push from another clone.

### A batch

When a land takes the land lock it fetches `main` then, never before, and becomes the *holder*. It takes into its batch every land still waiting for the land lock, in turn order after its own:

- A waiting land's ticket names its branch. The holder reads the branch from the clone's own refs, where every worktree of the clone sees it.
- For each one, the holder does what that land's own attempt would: it reads the approval of its fingerprint under the policy at the base (the fingerprint is of the branch's own diff, so a batch doesn't change it), merges the branch onto the chain so far as its own merge commit, `Merge <branch> into main`, and runs the gate on that commit.
- A branch that isn't approved, conflicts with the chain, fails its gate or whose check environment differs from the holder's (`land/check-env`) is left out and stays in the queue. Its own `clm land` reports the reason when its own turn comes, as it does today.

The batch's selection is the union of what each member would run alone (its selection, its overlap with `main`, or its readers) and of what any two members can both break (`check.affected`'s `overlap`). The holder runs `[land] check` once, on the last merge commit of the chain, then `clm check`.

When it passes, the holder takes the push lock and pushes the last merge commit, which carries the chain, so `main` never points at a commit that wasn't the tip of a checked tree. It appends one `checked` record for the tip's code key and, for each member, the `effort` and `landed` records that member's land would have appended. A member's `landed` record names its own merge commit, the tip's code key as the key that was checked, and the tip commit as its `batch`.

Each waiting `clm land` prepares again when the holder's turn ends. One that finds a `landed` record for its head does its own clean-up and prints `landed <sha> <kind>` from the record, exit 0. One that doesn't is still in the queue and goes on as before.

### A batch that fails

When the batch's check or `clm check` fails, the holder halves the batch, keeping the half that holds its own branch, and tries again, until a batch passes or it is alone. Alone, it fails as a land fails today, with its own output, log and exit code. Branches dropped from a batch are still waiting; the next holder batches them. So a failure is only ever reported to a branch by its own `clm land`, on a tree holding `main` and that branch alone, and one bad branch in a batch of `N` costs about `log N` extra checks.

### A passed check is kept

When a holder, or a lone land, has a check that passed on base `B` and finds inside the push lock that `main` is now `M`:

- It asks `clm affected --overlap <checked tip> <M>`, readers included, what its changes and `main`'s new commits can both break. This needs `M`'s code key to be checked, as `land/overlap-recheck` requires today.
- **Nothing:** it rebuilds its merge commits on `M`, runs the gate and `clm check` on them, pushes, and appends a composed `checked` record whose basis names the checked tip's key and `M`'s (`ledger/composed-check`).
- **Something:** it runs `[land] check` with that selection on the rebuilt tip, still inside the push lock, then pushes, with the same composed record. A failure here is a batch that fails, as above.
- **A conflict while rebuilding, or `M` unchecked:** the attempt starts again, as a moved `main` does today.

The same rule covers a lone land on every path, the readers' path included, and replaces the special case that the branch's head must have been checked whole beforehand: what counts is the check the land itself just ran. Ten moved attempts still exit 1.

### What stays as it is

- `land/lock-queue`'s lanes and priorities still order who takes the land lock, and so who leads a batch and the order of its members, and still order the check lock among lands, `--check-only` runs outside a build place and `verify`.
- `clm land --check-only`, `verify`, approvals, signing and the gate are unchanged. One clone has one writer and one key, so the holder signs for the batch.
- A project's part is unchanged: a check that runs the tests it is given on the tree it is given (`land-checks-stay-fast`).

## Decisions

- **`batched-lands`** (new): the land-lock holder lands the waiting lands on one check; halves on failure, keeping its own half; a failure is reported only by a branch's own land, alone.
- **`a-passed-check-is-kept`** (new): the push lock, and the overlap rule applied to the check a land just ran.
- **`ordered-lock-queue`** (amended): the queue's order now also sets a batch's leader and member order. `backlog/overlap-checks-queue-in-the-ordinary-lane` asked whether a land that runs only an overlap should take the priority lane, so it doesn't wait behind full checks. It no longer waits behind them: it joins their batch. The item is resolved with no change of lane.

## Constraints

- `land-locally`: still only `clm land` pushes `main`, with the checks run on the merged tree on that machine.
- `git-is-truth`: a waiting land learns its outcome from the ledger's `landed` record and from `main`, not from a private channel. The queue's tickets stay what they are, a clone's own scheduling state under `.clm/`.
- `land-checks-stay-fast`: the overlap rule and the batch's selection rest on its guarantee, as `land/overlap-recheck` does already; `[verify]` after the land is the backstop, and `clm verify`'s bisect can name a member of a batch, since each has its own merge commit.
- `deterministic-enforcement`: `clm ledger verify` must still find every land checked, so a member's `landed` record names the key that was checked.
- `yagni`: no setting for a batch's size, and no special handling of flaky tests beyond what a lone land has today.

## Left to the builder

- The ticket's format, so that a `clm` without batching still reads the queue's order.
- Whether the holder or the waiting land appends a member's `effort` records, so long as they land in the ledger as today.
- What a waiting land does when it finds its head on `main` with no `landed` record, as after a holder that was killed between the push and the append.
- The capability tests under `land-safely` and `work-concurrently` that observe one check per land.

## Open questions

- **Halving, not one at a time.** `backlog/queued-lands-each-run-a-full-check` settled halving on 2026-11-08; this keeps it, in the form where the holder keeps its own half. With the two or three lands that waited at once today it costs the same as landing one at a time after a failure.
- **Flaky tests.** The idea item proposed detecting a test that fails and then passes on the same tree. Left out: a failure is only ever reported on a lone branch's own tree, as today, so a flaky test costs what it costs now.
