---
status: applied
resolves: [clm-coverage-deleted-tests-leaves-out-a-file-named-md-that-scan-counts]
---

## Intent

Deletes `backlog/clm-coverage-deleted-tests-leaves-out-a-file-named-md-that-scan-counts` as accepted: the behaviour it names is never observable, so nothing is wrong to fix.

`--deleted-tests` lists only tests: `clm_coverage::deleted_tests` reads each file through `file_tests`, which takes its tests from `tests_in`, and `tests_in` finds tests only in a file whose extension is one of the languages `check.coverage/markers-in-tests` reads (`rs`, `ts`, `tsx`, `mts`, `cts`, `js`, `jsx`, `mjs`, `cjs`, `py`). Every path the base filter in `crates/clm-check/src/lists.rs` drops ends in `.md`, so its extension is `md` or, for a file named `.md`, none: neither is such a language, so the file holds no test `deleted_tests` could list, with or without the filter. A file named `.md` does count its markers in `scan`, but as markers outside any test, which `--deleted-tests` never lists, so `land/deleted-tests-approved` can miss nothing through it. The filter only spares reading Markdown blobs, and `check/coverage-lists` as written ("the revision's files being those under the CLM root", read by `check.coverage/test-lists`) holds with it. Whether to keep it is left to `check`'s pending `coverage-lists` build, which reworks that function.
