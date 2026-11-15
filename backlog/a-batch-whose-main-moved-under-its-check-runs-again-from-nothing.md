---
title: "A batch whose main moved under its check runs again from nothing"
kind: bug
components: [land]
---

`land`'s Design says lands taken together keep their check as a single land does (`overlap-recheck`, `a-passed-check-is-kept`): when `main` moved while the batch's check ran, only what the batch and `main`'s new commits can both break runs again, with `P` the last of the merge commits.

The build of `batched-lands` doesn't do this. When `main` has moved by the time the batch takes the push turn, it keeps the `checked` record for the batch's tip, pushes nothing, and lets each land go on as it did before the batch: the holder starts its attempt afresh, which keeps its own `checked` record only when the head still contains `main`, and the lands it took wait for their own turns. A batch that loses the race to a push from another clone so costs every land in it a check of its own.

Fixing it means rebuilding the chain on the new `main` inside the push turn, running `clm affected --overlap <tip> <main>` and the gate and `clm check` on the rebuilt tip, and pushing, as `land.rs` does for one land.
