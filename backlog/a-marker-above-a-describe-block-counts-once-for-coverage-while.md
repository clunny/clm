---
title: "A marker above a describe block counts once for coverage while uncited-test counts every test inside it"
kind: bug
components: [check.health, check.coverage]
origin: 20261124-wave-6-worker-by-the-rule
---

A `req:` marker placed above a TypeScript `describe` block is read two ways. Coverage counts it once, as one citation of the requirement by the block. `clm health`'s `uncited-test` row counts every test inside the block as citing nothing: for `web`, 112 of its 119 `uncited-test` rows are tests inside such a block, so the advice is false for them.

The land has the same blind spot from the other side: it sees one citation per block, so a single test inside a cited block can be deleted with no `deleted-tests.txt` line and no refusal, though it was the proof of the requirement the block cites.

Expected: coverage, `uncited-test` and the land's count of deleted citing tests read a block's marker by one rule, either as citing every test inside it (so each test's deletion is seen) or as citing nothing (so a marker goes on each test).

Found while drafting wave 6 (`20261124-wave-6-worker-by-the-rule` and `20261124-wave-6-app-by-the-rule`); inside the wave, the builds of `web`, `web.views` and `web.client-live` put a marker on each test that proves a requirement, which works around it.
