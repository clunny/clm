---
title: "A cut is what a program lists: names and sentences from the two specs, and tests from a list the change declares"
date: 2026-11-18
status: accepted
---

`removal-needs-a-yes` was kept by the session alone. Replayed over the last 150 spec lands before this decision, 26 removed a requirement, a scenario or a SHALL sentence, and nine of them were ordinary changes that dropped one scenario and landed on checks. Of the 163 `impl/*` lands since `8fe48394`, 14 lost a citing test by name, about half by a build renaming or replacing one.

**What the gate lists** (`gate/cuts`). From the requirements at a spec branch's merge base and at its head: a requirement that is gone, unless each of its scenarios is moved; a scenario whose name is gone from its requirement, unless it is moved; and a requirement whose statement has fewer sentences that hold `SHALL`. From the changes: each line the branch adds to a `changes/<id>/deleted-tests.txt`. A branch with any of these needs review, and the lines are in `clm gate`'s output, at the top of the pull request's body and in the web review.

**Moved, one for one and within the diff.** A gone scenario is moved when the head holds a scenario of that name under another requirement where the base held none, and each such place takes one. So a pass keeps a scenario's name when it rewords or moves it, a move across components sits in one branch, and of two same-named scenarios merged into one requirement, one is listed. Whether the merged scenario states what both did is the reader's question the line asks.

**A sentence that places a test isn't counted.** The human agreed on 2026-11-18 that a sentence saying where a test sits, where tests "sat" or that no test "is left" somewhere is cut under `requirements-state-what-the-contract-shows` with no separate yes: it states no behaviour of the product, and `check/tests-cite-their-owner` and `gate/own-files` check what it guarded. The gate leaves out a sentence holding `SHALL sit`, `sat in` or `is left in`. Each change still names every such sentence under "What the requirements no longer state".

**The tests a change deletes are declared.** `clm coverage --obsolete-tests` lists every test of a merged requirement, which a build re-cites, and misses a duplicate whose requirement stays. The change's list errs safely both ways: a line too many is read, and a line too few has the land refuse the build. An approved list is one on the base a land lands on, live or archived: it got there through the gate, which asked review for each line.

**What `clm land` refuses** (`land/deleted-tests-approved`). Any code branch, whatever its name, that leaves the tree with fewer citing tests of a name and no list naming the test. A moved test is not gone. A renamed test is: a build keeps a test's name.

**What stays a reader's.** A reword that keeps the count of sentences; a sentence of behaviour worded like one that places a test; whether a merged scenario states what two did; and whether a test asserts what its requirement says, which is the judge audit's.

Considered:
- **The gate reading names only.** Rejected: the first draft of `20261118-affected-by-the-rule` moved two library offerings out of their requirements with every scenario name kept.
- **Matching a name against the whole head.** Rejected: 15 requirements hold a scenario named "covered by the scan", so removing one would never be listed.
- **Excusing a rename whose scenario text is unchanged.** Rejected as not needed yet: one line on the list is its cost.
- **Computing the approved tests from the removed requirements.** Rejected: it would let a build delete the tests of a requirement that was merged, which the gate had read as moved and shown to nobody.
- **Holding only `impl/*` branches to the lists.** Rejected: a small code branch deleted a citing test in the replay.
