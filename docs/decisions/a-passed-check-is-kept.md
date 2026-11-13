---
title: "A land keeps the check it ran when main moves, and every land pushes inside a short turn"
date: 2026-11-13
status: accepted
---

A `clm land` whose check passed used to throw it away when `main` had moved before its push. On 2026-11-13 one land ran its check 8 times over 80 minutes this way while a build loop landed other branches (`backlog/a-land-with-a-full-check-to-run-can-lose-every-attempt-to-lands`, resolved by `20261113-land-starvation`). Two rules end it.

**The check a land just ran counts.** `land/overlap-recheck` already let a land whose branch was checked whole beforehand run only the overlap of its branch and `main`'s new commits. The same reasoning holds for the merge commit a land has just checked: a test neither side can affect gives the same result on the new merge (`land-checks-stay-fast`). So a land whose check passed on an earlier `main` runs again only what `clm affected --overlap` prints for that merge commit and the new `main`, on every path, the readers of checked code included.

**Every land pushes inside the push turn.** The overlap rule alone still lets a stream of lands move `main` between a finished check and its push. So each land holds a short exclusive turn from the fetch its push relies on to the push. A land that ran a check takes the turn after the check, so it does at most one overlap re-check; a land with nothing to check takes only the turn.

**Why lands with nothing to check don't queue behind checks.** Making every land wait for the land lock would also stop `main` moving under a check, and is simpler. But a land with nothing to check lands in seconds today and would then wait minutes; the human asked for a way to let it through without starving the others (2026-11-13). With the overlap rule its push costs the land that is checking nothing, or a few tests.

**Why the turn is in arrival order.** A land that has finished its check must not lose the turn again and again to lands arriving after it, which an operating system's lock allows.

**A waiting land prepares again.** A land that waited for the land lock merges with `main` as it is once it holds the lock, on the readers' path too. It checked a stale merge there before, which is what made every attempt of the starved land lose.

**What it rests on.** A composed verdict is only as good as `land-checks-stay-fast`, which no program checks. A build's own `clm land --check-only` finds a broken `main` within one build, and `[verify]` names the land (`verify-after-land`).
