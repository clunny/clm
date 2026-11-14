---
status: applied
---

## Intent

`check.coverage/test-spans` stops saying what `markers_in` and `scan`'s `req` hold, which is `markers-in-tests`'s to say. Its last sentence, "Neither changes what `markers_in` returns or what `scan`'s `req` holds", was written to say that the two new calls narrow nothing by themselves. Two of its tests took it as "`req` holds every marker", which `markers-in-tests` ends, so that requirement's build would have had to break a current requirement's tests. Part of phase 3 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`.

## Approaches

- **Taken: reword the sentence.** `outside` holds the markers outside tests whether or not `req` holds them too.
- **Not taken: leave it and let the build re-read it.** The `check.coverage` build that narrowed `req` once already reverted on failing tests.

## Outcome

`test-spans` reads `behind` until `check.coverage`'s next build, which brings its two tests (`markers_in_is_unchanged`, `scan_holds_the_outside_markers_beside_req`) to the new text in the same build that makes `markers-in-tests`. No behaviour is removed: nothing but those tests relied on `req` holding the markers outside tests, once the three readers read `outside` (`9a547682`).

## Constraints

One sentence of one requirement.
