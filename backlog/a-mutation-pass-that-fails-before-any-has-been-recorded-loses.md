---
title: "A mutation pass that fails before any has been recorded loses the requirements it was to judge"
kind: bug
components: [verify]
---

Step: `clm verify` running the mutation pass (`verify/mutation-pass`) for the first time in a clone, where the ledger holds no `mutated` record yet.

What happened: the pass's command failed, so the pass recorded nothing, as the rule says. The `verified` record for that commit had already been appended. The next verification, of a later commit, took its starting point from its own `verified` record's `since`, which was the commit of the failed pass. So every requirement whose test was added or changed before that commit was no longer due, and none of them was ever judged. The next pass then appended a `mutated` record, which fixed the starting point after them for good.

Expected: a pass that fails leaves its requirements due for the next pass, whether or not an earlier pass has been recorded. With no `mutated` record, the starting point should not move forward past a verification whose pass failed or did not run.

Once a `mutated` record exists the starting point is that record's commit, so a later failed pass loses nothing. The gap is only before the first record, which is exactly when a project has just switched the pass on and its command is least proven.

Rule involved: `verify/mutation-pass` ("With `P` the commit the newest `mutated` record on `C`'s first-parent chain names, or the `verified` record's `since` when there is none"), `mutation-after-land`. It needs a spec change: the requirement as written states the behaviour seen.
