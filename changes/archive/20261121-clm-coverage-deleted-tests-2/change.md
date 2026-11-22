---
status: done
resolves: [clm-coverage-deleted-tests-reads-a-linked-test-file-on-one-side-only]
---

## Intent

`clm coverage --deleted-tests <base>` reads a test file that is a symbolic link on both sides or on neither: it leaves links out of both, as `clm land` already does, so a link's tests can no longer hide a citing test lost elsewhere.

## Problem

The base side of `--deleted-tests` (`crates/clm-check/src/lists.rs`) reads the revision through `clm_git::tree_blobs`, which skips symbolic links (`vcs/tree-blobs`). The working tree side reads `clm_coverage::read_files`, which reads a link to a file through it (`check.coverage/markers-from-files`). A test in a linked file therefore counts in the working tree and never in the base. `deleted_tests` counts citing tests by bare name across files, so the link's copy at the head can stand in for a same-named test the branch deleted from another file, and that test goes unlisted: with `tests/t.rs` and `tests/u.rs` each holding a test `two` citing `a/x` and `tests/l.rs` a link to `tests/u.rs`, deleting `two` from `tests/t.rs` prints `# tests deleted since b: 0`. `clm land` (`land/deleted-tests-approved`) reads both sides through `tree_blobs`, so it skips links on both and lists the test; the two commands, which `check/coverage-lists` says give one verdict, disagree.

No requirement as written says which way a link is read on the base side, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: leave a link out of both sides of `--deleted-tests`.** The working tree side drops from `read_files`' list each path that is a symbolic link; the base side already skips them. This is what `clm land` does, so the two commands agree again (`reuse-first`: no new reading of links). A link's tests lose nothing that matters: when the link leads to a file of the tree, that file's tests are counted under its own path on both sides, and a link duplicates them.
- **Not taken: read a link to a file in the base as well.** It would resolve each link's target inside the revision's tree, a reading `tree_blobs` doesn't offer, and `clm land` would then disagree with `clm coverage` unless it changed too, which is a second component's work for no case anyone has met.
- **Not taken: leave links out of `read_files` itself.** `scan` and the coverage read links through it (`check.coverage/markers-from-files`), and a test reached only through a link still counts there; changing that changes what `clm coverage` reports for every user, not only the deleted-tests list.

## Outcome

`check/coverage-lists` gains the exception and a scenario, `a test file that is a link`; `check`'s Design says how the list drops links. The build changes `deleted_tests_lines` in `crates/clm-check/src/lists.rs` and adds the scenario's test.

## Edge cases and workarounds

- A link whose target lies outside the side's own files (outside the CLM root, or outside the repository) holds tests that `--deleted-tests` now never counts on either side, as `clm land` never did: the land's verdict is the one that gates, and the list now matches it.

## Constraints

- `--obsolete-tests` and the coverage keep reading links through, unchanged.
