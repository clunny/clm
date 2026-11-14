---
title: "A land with a full check to run can lose every attempt to lands that need none"
kind: bug
components: [land]
---

Step: `clm land` on a branch whose merged tree is unchecked, while a build loop lands other branches.

What happened: each attempt merged with the current `main`, waited for the land lock, ran the full check (about 3 minutes) and then found `main` had moved, so the attempt was thrown away and the next began. Over about an hour the land ran the full check 8 times without landing, while `main` took a merge roughly every 7 minutes. At least one of those merges landed while a different land held the lock and this one waited in the queue: a land takes the lock only when it has a check to run, so a land whose tree is already checked pushes `main` without queueing. The queue's lanes and streak order only the lands that wait for the lock, so they give this land no protection. The land gives up after 10 attempts with "`main` keeps moving", and the checks it ran are wasted.

Expected: a land that has waited or been beaten gets a turn in which no other land moves `main` before its push, so the number of full checks it runs is bounded and small, however busy the loop is. Lands that need no check should queue behind it, or the loop should hold its other lands while one has lost an attempt.

Rule involved: `land/lock-queue` and `land/check-lock`; the comment on the push says the lock covers the gate, `clm check` and the push so no other land gets in first, which doesn't hold for lands that take no lock.
