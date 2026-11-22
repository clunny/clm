---
title: "`clm coverage --deleted-tests` reads a linked test file in the working tree but not in the base"
kind: bug
components: [check]
---

Step: building `check/coverage-lists@9f4e5a9b` (wave 2, `check`'s third claim). The code review found this. It was there before this build.

What happens: the base side of `--deleted-tests` (`crates/clm-check/src/lists.rs`) reads the revision through `clm_git::tree_blobs`, which skips symbolic links (mode `120000`). The working tree side reads `clm_coverage::read_files`, which follows a link to a file and reads its target (`markers-from-files`). A test file that is a link therefore counts on one side only.

What it costs: a link's tests are seen in the working tree but never in the base, so `deleted_tests`' count by bare name across files can take a test lost elsewhere for one the link kept. A citing test lost without a yes can then go unlisted (`removal-needs-a-yes`, `land/deleted-tests-approved`).

Fix: make both sides agree. Either read a link to a file in the base as well, or leave links out of both. If `markers-from-files` must say which, that is a spec change.
