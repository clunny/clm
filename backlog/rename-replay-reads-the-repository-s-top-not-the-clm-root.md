---
title: "rename/replay reads the repository's top, not the CLM root"
kind: bug
components: [rename]
origin: 20261119-the-gate-reads-changed
---

`crates/clm-rename/src/replay.rs` lists both trees with `git ls-tree --full-tree` and extracts the whole repository with `extract_tree`, so in a CLM repository that is a subdirectory `tool/` of its git repository, `added_changes` sees `tool/changes/<id>/change.md`, never `changes/<id>/change.md`, and `replays` is always false. A rename branch there is then classed `mixed` by the gate and can't land; it fails closed. Found by the builder check of `20261119-the-gate-reads-changed`.

Expected: `replays` reads the CLM root's trees with paths relative to it, as `gate.fingerprint/fingerprint` does (`fingerprint-covers-the-clm-root`), so a rename branch in a subdirectory root is `spec`. Which requirement states this needs a design.
