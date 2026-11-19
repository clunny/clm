---
title: "The review lists every line of a test list whose base text GitHub withholds"
kind: bug
components: [web.worker.review]
---

When a change's `deleted-tests.txt` is not UTF-8 (or binary) at the merge base but readable at the head, GitHub gives its base text as null, so `files::spec_files` leaves it out of the base's files and `review::test_lists` passes `base: None`. `clm_gate::cuts` then counts every line of the head's list as added, so the review's `cuts` list each of them, while `clm gate`, which reads the base text lossily (`text_if_any`), lists only the lines the branch adds. `review` is true either way, so no approval is lost, but the page shows cut lines `clm gate` doesn't print.

The Worker can't read the base text at all, so `pr-review` needs a rule for this side, as it has one for a list withheld at the head: for example, leave such a list's lines out of `cuts` and keep `review` true. Found by `/code-review` on `impl/web.worker.review` (pre-existing in `test_lists`).
