---
title: "A spec branch's land skips the code tests that read spec text, so main can land failing"
kind: bug
components: [land]
origin: 20261104-found-by-search-and-agents
---

What happened: a spec branch whose prose under `changes/` held a retired term landed on `main` through `clm land`. Because the merged tree's code had already been checked, `clm land` ran only `clm check`, not the `[land] check` command. A test in the code suite that reads every file under `changes/` (the retired-terms test) therefore never ran on that tree, and `main` now fails `cargo test`. The next session to run the suite found the failure on a branch that hadn't caused it.

Expected: a land skips the code checks only when no file it changes is read by a test. A spec-only diff whose files a code test reads should run that test, or the full check, before it pushes `main`.

Rule involved: the land check's shortcut for trees whose code was already checked (CLAUDE.md's Gotchas: "A tree whose code was already checked runs only `clm check`"), and `land-locally`.
