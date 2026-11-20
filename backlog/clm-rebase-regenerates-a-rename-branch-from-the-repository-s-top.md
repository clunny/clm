---
title: "clm rebase regenerates a rename branch from the repository's top, not the CLM root"
kind: bug
components: [sync]
origin: 20261119-rename-replay-reads-the
---

`crates/clm-sync/src/renames.rs` finds the rename branch's `change.md` as `HEAD:changes/<id>/change.md`, which git reads from the repository's top, lists revisions with `git ls-tree --full-tree` (`rev_ids`), and runs `clm rename` on the whole extracted tree (`replayed`, `extract_tree`). In a CLM repository that is a subdirectory `tool/` of its git repository, `clm rebase` then never finds the change, or, once `rename/replay` reads the CLM root's trees (`20261119-rename-replay-reads-the`), runs `clm rename` at the extraction's top, where there is no `specs/`, so it refuses or names the wrong paths as differing. A rename branch there can't be regenerated (`sync/regenerate-renames`); it fails closed. Found while designing `20261119-rename-replay-reads-the`.

Expected: `regenerate-renames` reads, extracts and compares the CLM root's trees with paths relative to it, as `rename/replay` does (`fingerprint-covers-the-clm-root`). No requirement of `sync` states this yet, so the fix needs a design.
