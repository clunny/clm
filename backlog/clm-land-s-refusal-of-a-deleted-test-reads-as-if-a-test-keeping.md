---
title: "clm land's refusal of a deleted test reads as if a test keeping its name must keep its citations"
kind: bug
components: [land]
origin: 20261124-wave-6-worker-by-the-rule
---

When `clm land` refuses a branch that loses a citing test, its message says to keep the test under its name, citing every requirement it cited. Read plainly, that says a test that keeps its name must also keep each of its citations, so moving a test's marker from one requirement to another would count as a deletion.

The land doesn't do that: the "citing every requirement it cited" condition applies only to a test that is renamed. A test that keeps its name and is re-cited to another requirement is not a lost test.

In wave 6, a drafter read the message as written and listed 7 re-cited tests for deletion in `deleted-tests.txt`, which would have asked the human to approve cuts that weren't cuts.

Expected: the refusal's text says what the land checks: a test kept under its name is not lost whatever it cites; a renamed test must cite every requirement the old name cited. `land/deleted-tests-approved`'s wording states the same.
