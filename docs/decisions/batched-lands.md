---
title: "The land that takes the lock lands the waiting lands on one check, once measurements say it is worth it"
date: 2026-11-13
status: accepted
---

When several lands of a clone wait for a check, each runs its own, one after another (`backlog/queued-lands-each-run-a-full-check`, elaborated with the human on 2026-11-08 and proposed in `20261113-land-starvation`). Merge queues check one tree holding `main` and every waiting branch instead.

**The holder lands for the others.** Each `clm land` is its own process. The one that takes the land lock merges every waiting land's branch after its own, each as its own merge commit, runs one check on the last, and pushes once, so `main` never points at a tree that wasn't checked. It appends each branch's records; each waiting `clm land` finds its `landed` record and prints its usual line. An approval's fingerprint is of the branch's own diff, so landing with others doesn't disturb it.

**Why not let each waiting land push itself after a shared check.** `main` would pass through trees no check ran on, and other lands could push in between.

**Halving, keeping the holder's half.** When the check fails, the holder tries again with the first half of the lands it took, until a check passes or it is alone. A land dropped stays in the queue. So a `clm land` is told it failed only by its own run on `main` and its branch alone, with the output, log and exit code it has today, and one bad branch among `N` costs about `log N` extra checks.

**No new ledger field.** A land taken with others records, in the `code_key` every `landed` record has, the key its check ran on. Which lands shared a check can be read from records sharing a key, so an older `clm` reads these records as it reads any.

**A killed holder.** A push and the records that follow it are two steps. Each merge commit of a push of several lands names in its message the key their check ran on, and the `checked` record for that key is appended before the push, so a `clm land` that finds its head on `main` with no `landed` record can prove the commit was checked and append the records for the whole push. A merge no land made has no such `checked` record and is refused, so `clm ledger verify` still reports it.

**Why it waits.** `clm land` is the one door every session uses, and this changes how every land of a clone reaches `main`. `a-passed-check-is-kept` removes the discarded checks, which were most of the waiting on 2026-11-13. A question item blocks these requirements until the busy share of the land lock is measured after that: under one half, batching saves less than half a check per land and isn't built.

**Left out.** No limit on how many lands are taken together, and no detection of flaky tests: a failure is reported only on a lone branch's own tree, so a flaky test costs what it costs today plus a halving.
