---
title: "overlapping_lands flakes with offline under load"
kind: bug
components: [land]
---

crates/clm/tests/land.rs overlapping_lands failed once in a full ci-check run (nextest, 3741 tests) with "clm land: offline: origin can't be reached" in the second land, and passed 3 of 3 alone: a fetch of the shared origin failing while two lands overlap under load. Relates to tests-run-concurrently.
