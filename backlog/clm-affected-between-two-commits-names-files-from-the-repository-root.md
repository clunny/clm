---
title: "`clm affected` between two commits names files from the repository root, not the CLM root"
kind: bug
components: [check.affected]
---

Found by the code review of `impl/check` building `check/coverage-lists` at 963db740, which fixed the same mismatch in `clm coverage --deleted-tests`.

What happens: in a repository whose CLM root is a subdirectory (say `sub/`), `affected_between` in `crates/clm-affected/src/lib.rs` (used by `affected_overlap`) runs `git diff --name-only` without `--relative`, keeps the paths `clm_git::tree_blobs` gives from the top of the repository without filtering or stripping the `git rev-parse --show-prefix` prefix, and loads the graph from the root of the extracted tree, where `specs/` doesn't exist.

Effect: the changed paths and scanned files are named `sub/...` while the component `paths` globs are relative to the CLM root, and the graph load fails or finds nothing, so the selection between two commits is wrong or refused there.

Fix: read and name both trees under the CLM root as `crates/clm-land/src/deleted_tests.rs` and `crates/clm-check/src/lists.rs` (`deleted_tests_lines`) do, add `--relative` to the diff, and load the graph from the prefix directory of the extracted tree. Check first whether a requirement of `check.affected` states that the selection is read from the CLM root; if none does, this needs a spec change (`code-fix-needs-a-written-rule`).
