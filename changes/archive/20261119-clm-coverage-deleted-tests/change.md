---
status: done
resolves: [clm-coverage-deleted-tests-names-base-files-from-the-repository]
---

## Intent

`clm coverage --deleted-tests <base>` reads the revision's files under the CLM root and names both sides from the CLM root, as `clm land` does, so the two give one verdict on whether a branch lost a citing test in a repository whose CLM root is a subdirectory of its git repository too (`backlog/clm-coverage-deleted-tests-names-base-files-from-the-repository`).

## Problem

- `clm_check::lists::deleted_tests_lines` (`crates/clm-check/src/lists.rs`) reads the base with `clm_git::tree_blobs`, which lists with `git ls-tree --full-tree`, so its paths start at the repository root and take in files outside the CLM root; it reads the working tree with `git ls-files` run in the CLM root, whose paths start there.
- When the CLM root is a subdirectory, no base file's path matches a head file's. `deleted_tests` counts by bare name, so plain moves still balance, but its rename match looks in the head's text of the gone test's file, which never exists, so a test renamed in its own file citing every requirement it cited is listed as gone. A citing test in a file outside the CLM root is listed as gone too, since only the base side reads it. `clm land` (`land/deleted-tests-approved`) strips `rev-parse --show-prefix` and keeps only files under the CLM root, and lands such a branch.
- `check/coverage-lists` says only "the files of the revision `<base>` against the working tree's", which states neither which files nor how they are named, so no requirement as written states the behaviour the fix restores, and the fix is a spec change (`code-fix-needs-a-written-rule`). Nothing since 2026-11-12 fixed it: the item was filed today by `20261119-the-land-and-clm`, which names it as `check`'s bug and doesn't depend on it, and no code fix resolves it.

## Approaches

- **Taken: `check/coverage-lists` names the revision's files as the land does.** The revision's files are those under the CLM root, named from it, as the working tree's already are. The build filters and strips `git rev-parse --show-prefix` as `crates/clm-land/src/deleted_tests.rs` does; `deleted_tests` is unchanged.
- **Rejected: a helper in `clm-git` that lists a revision's blobs under the CLM root, named from it.** About a dozen crates strip the prefix by hand, and a shared call would be reuse, but it is `vcs`'s requirement and a sweep of every caller, a cleanup apart from this bug; the build here is a filter and a strip of a few lines.
- **Rejected: accepting it.** `clm coverage --deleted-tests` is how a session learns what the land will refuse; a list that names tests the land lands sends the session after a `deleted-tests.txt` it doesn't need, which needs a human's review.

## Outcome

- `check/coverage-lists` (modified): the revision's files are those under the CLM root, and both sides are named from the CLM root, as `clm land` names them; a scenario *a CLM root in a subdirectory* shows a test renamed in its own file and a citing test outside the CLM root both left off the list.

## Constraints

- `deleted_tests`' signature, rules and return stay as they are; `clm land` changes in nothing.
- Every scenario of `check/coverage-lists` as it stands keeps its outcome; in a repository whose CLM root is the top of its git repository the output is unchanged.

## Edge cases and workarounds

- The base side skips `.md` files while the working tree's side reads them; a Markdown file holds no test `deleted_tests` counts, so the difference changes no verdict, and this change leaves it.

## Open questions

None.
