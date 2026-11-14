---
title: "dangling-markers, the rename rewrite and test selection still read only the markers that count"
kind: question
components: [check.coverage, check, rename, check.affected]
blocks: [check.coverage/markers-in-tests]
---

`check.coverage/markers-in-tests` makes `markers_in` and `scan(root).req` hold only the markers inside tests, in the languages `tests_in` reads. Three requirements it needs say they read every marker, `scan`'s `outside` included, but their code still reads only `req`, and works today only because `req` still holds every marker:

- `check/dangling-markers`: `crates/clm-check/src/lib.rs` passes `markers.req` alone to `dangling::violations`; its test `dangling_marker_outside_a_test_is_reported_once` fails once `req` narrows.
- `rename/marker-rewrite`: `crates/clm-rename/src/build.rs` builds its file list and `holding` from `scan(root).req`; `a_marker_outside_a_test_is_rewritten` fails.
- `check.affected/affected-files`: `crates/clm-affected/src/lib.rs` uses `clm_check::markers_in`; `a_marker_outside_a_test_still_selects` in `crates/clm/tests/affected.rs` fails.

The `check.coverage` build did the change, saw those three tests fail, and reverted it, since a build may edit only its own component's files. `clm check`'s `cites` and `skipped` rules and `clm-verify`'s mutation pass also read `scan.req` and would change meaning silently.

**What a spec change needs to say.** Each of the three components moves first, in a requirement of its own, onto the union of `req` and `outside` (for `check`, `check.affected` and `rename`), as `markers-in-tests`'s `Needs` already names them, and the Needs must then wait on those requirements being built against the code (their markers are current already, so `clm next` offers `markers-in-tests` too early). Then `markers-in-tests` is built. The text of `test-spans` ("Neither changes what `markers_in` returns or what `scan`'s `req` holds") also contradicts `markers-in-tests` once built and wants rewording.
