---
title: "`clm coverage --deleted-tests` leaves out a base file named `.md` that `scan` counts as a citing test"
kind: bug
components: [check]
---

Step: building `check.coverage/markers-in-tests` (build 2 of wave 2). The code review found this. It was there before this build.

What happens: `--deleted-tests` in `crates/clm-check/src/lists.rs` reads the base revision's files and drops Markdown with `!p.ends_with(".md")`. `scan_text` in `clm-coverage` drops only a file whose extension is exactly `md` (`Path::extension() == "md"`), as `markers-in-tests` states. A file named `.md` (or `dir/.md`) has no extension, so `scan` reads its markers, but the base filter leaves it out.

What it costs: a deleted `.md` file holding a counting marker is never listed as a lost citing test, so `land/deleted-tests-approved` can miss a test lost without a yes (`removal-needs-a-yes`).

Fix: drop the suffix pre-filter and let `markers_in` decide, or use the same `Path::new(p).extension().is_some_and(|e| e == "md")` test. `check`'s `coverage-lists` build, which removes the copy in `lists.rs`, is the natural place.
