---
status: applied
resolves: [clm-affected-between-two-commits-names-files-from-the-repository-root]
---

## Intent

`clm affected` reads a change from the CLM root it runs in, in every mode, so its selection is right in a repository whose CLM root is a subdirectory of its git repository: the paths it reads and prints are those under the CLM root, named from it as component `paths` are, and `--overlap` loads each commit's graph from the CLM root's place in that commit's tree (`backlog/clm-affected-between-two-commits-names-files-from-the-repository-root`).

## Problem

- `affected_between` (`crates/clm-affected/src/lib.rs`), which `--overlap` and `clm land`'s overlap recheck use, diffs the two commits without `--relative`, keeps `clm_git::tree_blobs`' paths, which start at the top of the repository, and loads the graph from the top of the extracted tree, where a subdirectory root's `specs/` isn't. With the CLM root in `sub/`, every path is `sub/...`, which no component's `paths` hold, and the graph load fails or finds nothing.
- The working-tree side has the same mismatch, which the item doesn't name: `affected`'s `git diff --name-only <merge base>` names paths from the top of the repository too, and lists paths outside the CLM root, while its `ls-files` calls name them from the CLM root. A branch changing `sub/crates/a/src/lib.rs` prints `all sub/crates/a/src/lib.rs` (or `all other/x` for a file outside `sub/`), and a reader of `ARCHITECTURE.md` is selected by a change to the top-level `ARCHITECTURE.md` that isn't the project's. Checked by hand in a scratch repository.
- No requirement of `check.affected` states which root the paths are read from; the Design says "a changed path is matched as git lists it, relative to the repository root", which is the bug. So the fix is a spec change (`code-fix-needs-a-written-rule`). Nothing since 2026-11-12 fixed it: the item was filed today by the review of `20261119-clm-coverage-deleted-tests`, which fixed the same mismatch in `check/coverage-lists`, and no code fix names it.

## Approaches

- **Taken: one added requirement, `clm-root-paths`, for every mode.** The paths `clm affected` reads and prints are under the CLM root and named from it, with `--base`, `--readers`, `--why` and `--overlap` alike; `--overlap` reads each commit's tree at the CLM root's place. The build adds `--relative` to both diffs, filters and strips `git rev-parse --show-prefix` from `tree_blobs`' paths as `crates/clm-land/src/deleted_tests.rs` does, and loads the graph from the prefix directory of the extracted tree. An added requirement, not a modified `affected-base` or `overlap-selection`, leaves those requirements' many citing tests current.
- **Rejected: fixing only `affected_between`, as the item names.** The working-tree side is wrong the same way in the same component and is what every land runs first; a rule for one mode would leave the other wrong and unstated (`correctness over convenience`).
- **Rejected: a `clm-git` helper listing a revision's blobs under the CLM root.** As `20261119-clm-coverage-deleted-tests` found, it is `vcs`'s requirement and a sweep of a dozen callers; here the strip is a few lines.

## Outcome

- `check.affected/clm-root-paths` (added): three scenarios, a branch, two commits and a reader, each in a repository whose CLM root is `sub/`, with a path outside `sub/` that must select nothing.
- The Contract's changed-files bullet names the rule; the Design says the diffs run with `--relative` and how `affected_between` strips the prefix and finds the graph.

## Constraints

- In a repository whose CLM root is the top of its git repository the output is unchanged; every existing requirement and scenario keeps its outcome.
- The library calls' signatures (`affected`, `affected_between`, `overlap`, `readers`, `why`) stay as they are.

## Edge cases and workarounds

- `exists` for a working-tree changed path is `root.join(path).is_file()`; with paths named from the CLM root this becomes right in a subdirectory root without a further change.

## Open questions

None.
