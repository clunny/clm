---
title: "overlapping_lands fails under the full suite's load but passes alone"
kind: bug
components: [land]
origin: 20261108-land-specs-still-name
---

`clm land --check-only` on `spec/20261108-land-specs-still-name` (a branch changing only `specs/land`, a change and `docs/decisions/land-policy-from-base.md`) failed in `.github/scripts/ci-check.sh` on `clm::land overlapping_lands` (`crates/clm/tests/land.rs:231`): the second land printed `waiting for another land's check on <tmp>/work/.clm/land: 1 ahead` and then `clm land: offline: origin can't be reached; push and land when it can`. Run alone (`cargo test -p clm --test land overlapping_lands`) it passes. The test reads no file the branch changes, so a timing-dependent failure under load stops an unrelated land's checks.

Expected: `overlapping_lands` passes whatever the load, or its fixture's `origin` stays reachable while a land waits in the queue.

`clm land spec/20261108-land-check-timing` failed the same way on a sibling test, `clm::land held_through_the_gate_on_the_merge`, at the same line (`crates/clm/tests/land.rs:231`), with the same two lines; it passed alone and on the retried land. The fault is likely in the fixture they share, not in either test.
