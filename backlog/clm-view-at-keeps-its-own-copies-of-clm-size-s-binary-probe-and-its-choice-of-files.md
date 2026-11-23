---
title: "`clm view --at` keeps its own copies of `clm size`'s binary probe and its choice of files"
kind: idea
components: [view.repo, check]
---

`clm-view-repo`'s `at.rs` decides that a blob is binary with its own probe, a NUL in the first 8000 bytes (`BINARY_PROBE`, `text`, `crates/clm-view-repo/src/at.rs:17,41-47`), a copy of the probe `clm-check`'s `size` uses (`crates/clm-check/src/size.rs`). `clm-replica` and `clm-init`'s survey hold copies too. `at.rs:111-123` also re-implements which files `clm size` counts: text files that `linguist_excluded` doesn't exclude, and each component's `SPEC.md` whatever its attributes say. If `check/node-size` changes either rule, `clm view spec --at` marks different components oversized from `clm view spec` at the same commit, and no test fails. One call in `clm-check` that sizes a set of blobs with their `.gitattributes`, built for wasm beside `size_of`, would hold both rules once, for `view.repo`, `replica` and `init`. Found in wave 4 of `a-passing-check-proves-less-than-the-spec-claims`, outside it (`check` is not in wave 4).
