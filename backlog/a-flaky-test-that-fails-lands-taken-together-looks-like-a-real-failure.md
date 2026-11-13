---
title: "A flaky test that fails lands taken together looks like a real failure"
kind: idea
components: [land]
origin: 20261113-land-starvation
---

When the check of lands taken together fails (`land/failed-batch`), the holder can't tell a flaky test from a real failure, so a flaky test costs every land in the batch a wasted check where it costs one land today.

Settled with the human on 2026-11-08, in the idea this came from (`queued-lands-each-run-a-full-check`): CLM can detect a flaky test, as a test that fails and then passes on the same tree, and report it as a defect in the test against `land-checks-stay-fast`, without guessing its cause; and retrying a failed batch once before blaming still helps.

Left out of `20261113-land-starvation` (2026-11-13): running a failed batch a second time pays only when flaky failures are more than about one in `N` of the failures of batches of `N` lands, and nothing counts failed land checks today, so the rate isn't known. Deciding this needs that count first, which means recording a failed land check somewhere.
