---
title: "A scenario whose outcome is reversed under its name lands on checks, so a draft can drop a refusal with no yes"
kind: bug
components: [gate]
origin: 20261118-renaming-a-citing-test
---

Drafting `20261118-renaming-a-citing-test`, the change reverses the outcome of two scenarios named *a renamed test* (`check.coverage/test-lists` from "returns `tests/t.rs::two`" to "returns nothing", `land/deleted-tests-approved` from "exits 1" to "lands"): the land stops refusing a behaviour it refused. No scenario name is gone and no `SHALL` sentence is dropped, so `clm gate` prints `spec` and lists no cut, and the draft prompt says to land a `spec` change on checks. `removal-needs-a-yes` says dropped behaviour needs the human's yes, and `cuts-are-what-a-program-lists` leaves "a reword that keeps the count of sentences" to the reader, but an unattended draft has no reader and no path in its prompt to ask for review when the gate says `spec`. The draft left the change for review against the gate's line. Either the gate lists a scenario whose THEN changes as a cut, or the draft prompt gives a draft a way to ask for review of a relaxation the gate can't see.
