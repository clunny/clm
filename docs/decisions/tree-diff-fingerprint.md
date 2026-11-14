---
title: "A branch's fingerprint hashes the tree entries it changes, not a diff"
date: 2026-10-26
status: accepted
---

An approval is pinned to the branch's fingerprint (`per-writer-ledgers`), so every place that computes one must get the same answer for the same commits: each clone's `clm land`, `clm approve` and `clm status`, and the `web` Worker, which both shows approvals and writes them. The fingerprint was `git patch-id --verbatim` of `git diff <merge-base> HEAD`, and a diff is not a function of the commits alone. `core.abbrev` sets the length of the blob IDs a binary file's patch-id hashes; `diff.renames`, `diff.algorithm`, `diff.indentHeuristic` and `diff.context` change which lines are paired and which context is hashed; `diff.noprefix` and `diff.mnemonicPrefix` change hashed header lines. The Worker hashes GitHub's compare diff, made under GitHub's settings (renames on, abbreviated IDs), and reads it as UTF-8, so a file that isn't UTF-8 hashes differently there. An approval made on one side then doesn't show, or doesn't count, on the other.

The fingerprint is now the SHA-256 over the entries `git ls-tree -r` lists for each path that differs between the merge base and `HEAD`: its mode, type and full object ID on each side, by path. `ls-tree -z` prints full IDs and unquoted paths whatever the config, and GitHub's Trees API lists the same entries, so the Worker computes it from two tree listings through the same function `clm fingerprint` uses (`clm_gate`, which already builds for wasm), with no diff and no second implementation.

What it keeps: a rebase or a merge of `main` that touched only other files leaves it unchanged, since those paths then match on both sides; any change to a file the branch adds, modifies or deletes changes it.

What it gives up: patch-id ignored line numbers, so merging a `main` that changed *another part of a file the branch also changes* kept the approval. A tree fingerprint changes then, and the branch needs approving again. That is the case where the approved text now sits in a file a human hasn't seen in that form, so a fresh look is defensible, and on spec branches it mostly arises when two changes touch one node's `SPEC.md`, which `gate/up-to-date` already sends through `clm sync`.

Since `derived-specs-fingerprint`, that case no longer lapses an approval on a branch that adds a change: its `specs/` is derived, so the fingerprint skips it and `clm land` verifies the derivation instead.

Alternatives considered:
- **Pin the diff options** (`--no-ext-diff --no-textconv --no-renames --full-index --src-prefix=a/ --dst-prefix=b/`, a fixed algorithm, `-U3`). Makes clones agree, but the Worker can't ask GitHub's compare API for that diff, and rebuilding it in wasm would mean reproducing git's xdiff byte for byte. Clones and the Worker would still disagree.
- **Normalise GitHub's diff in the Worker** (re-expand renames from blobs, look up full blob IDs). Still depends on GitHub's algorithm choice matching git's, and adds a second diff pipeline to keep in step.
- **Pin to the head commit.** Every rebase costs a re-approval, which `approve-pins-the-review` rejects.

Changing the definition changed every fingerprint: `approved` records written before it match no branch, so a branch then awaiting landing on an approval needed approving again. Recorded `landed` records keep their stored values. No transition period accepts both kinds: a lapsed approval fails closed, and keeping patch-id alongside would keep the mismatch this removes.
