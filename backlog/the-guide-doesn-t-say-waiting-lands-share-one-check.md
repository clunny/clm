---
title: "The guide doesn't say waiting lands share one check"
kind: deferred
components: [land]
---

`20261113-land-starvation`'s step two (`land/batched-lands`, `land/failed-batch`, `land/unrecorded-land-finished`) is built by `land`'s next build. Once those three are current, the guide's page `docs/guide/ci-and-deployment.md` must say that lands waiting on a clone share one run of the check, what a failed one does, and that a branch is told it failed only by its own `clm land`. The guide is a spec path, so the build can't carry it: it goes in a docs branch of its own, which deletes this item. It was a note in the question item that held step two back, deleted on 2026-11-15.

Once `land/batch-keeps-its-check` (`20261115-a-batch-whose-main`) is current too, the page's section on re-checking only the overlap must say that lands sharing a check keep it the same way when `main` moves, and that their `landed` records then say `recheck: none` or `overlap` though the shared check ran in full, its time on the first land's record.
