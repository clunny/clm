---
title: "The land that takes the lock lands the waiting lands on one check, each as its own merge commit"
date: 2026-11-13
status: accepted
---

When several lands of a clone wait for a check, each runs its own, one after another (`backlog/queued-lands-each-run-a-full-check`, elaborated with the human on 2026-11-08 and proposed in `20261113-land-starvation`). Merge queues check one tree holding `main` and every waiting branch instead. The human watched lands wait through a build session on 2026-11-13 and chose to batch them.

**The holder lands for the others.** Each `clm land` is its own process. The one that takes the land lock merges every waiting land's branch after its own, each as its own merge commit, runs one check on the last, and pushes once. It appends each branch's records; each waiting `clm land` finds its `landed` record and prints its usual line. An approval's fingerprint is of the branch's own diff, so landing with others doesn't disturb it, and the gate runs on each branch's own merge commit, since a branch merged before it can change whether it needs review.

**What `main` no longer promises.** Only the last merge commit of a push is a tree a check ran on. `land-locally` says so: every commit a push leaves `main` at is a checked tree, though not every commit behind it. The human accepted this (2026-11-13), as moving slow tests out of the land had already given up a full check of each commit (`verify-after-land`). The alternative, one merge commit with every branch as a parent, keeps every commit checked but lets `clm verify` blame only a batch and makes one branch hard to revert.

**Why not let each waiting land push itself after a shared check.** `main` would be left at trees no check ran on, and other lands could push in between.

**Only within a clone.** The locks and queues are one clone's, so build sessions on other machines land as before. A push from another clone costs a batch what it costs any land: the overlap re-check of `a-passed-check-is-kept`.

**A batch that fails.** The human's condition was that a failure must not bring a lot of waiting and bisecting (2026-11-13). Two signals the holder already has decide what happens, with no check spent to find them:
- *Which step failed.* A check that marks its steps says which was open when it failed and which selected paths that step ran for (`check-step-markers`). A land whose own part of the selection holds none of them isn't a suspect, and the lands that aren't suspects are taken together again at once.
- *How many suspects.* Four or fewer land alone, as they would today; more than four are halved. With one bad land among `N`, landing alone costs `N + 1` checks and halving about `1 + 2 log2 N`, which break even near four.

A `clm land` is told it failed only by its own run on `main` and its branch alone, with the output, log and exit code it has today, so no split blames an innocent branch. With the two or three lands that waited together on 2026-11-13, a failed batch costs one wasted check and then what landing costs now. The threshold is a constant, not a setting (`yagni`).

**Left out: running a failed batch again.** It would tell a flaky test from a real failure, and pays when flaky failures are more than about one in `N` of batch failures. Nothing counts failed land checks, so the rate isn't known (`backlog/a-flaky-test-that-fails-lands-taken-together-looks-like-a-real-failure`).

**No new ledger field.** A land taken with others records, in the `code_key` every `landed` record has, the key its check ran on (`landed-record-names-its-code-key`). Which lands shared a check can be read from records sharing a key, and an older `clm` reads these records as it reads any. Only the holder's record carries the check's time and steps, so sums of check time count a shared check once.

**A killed holder.** A push and the records that follow it are two steps. Each merge commit of a push of several lands names in its message the key their check ran on, and the `checked` record for that key is appended before the push. So a `clm land` that finds its head on `main` with no `landed` record can prove the commit was checked, judge the approval and the gate on it as a land would have, and append the records for the whole push. A merge no land made has no such `checked` record, or fails that judgement, and is refused, so `clm ledger verify` still reports it. Before this rule, landing such a branch again made a second, empty merge and left the first unrecorded.

**Why it waits.** `clm land` is the one door every session uses, and this changes how every land of a clone reaches `main`. `a-passed-check-is-kept` is built first and removes the discarded checks, which were most of the waiting on 2026-11-13. A question item blocks these requirements until the human deletes it, with the measurement that says how much waiting is left.
