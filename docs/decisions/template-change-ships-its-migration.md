---
title: "A change to a file clm init writes lands in one build with the migration that carries it"
date: 2026-11-09
status: accepted
---

When a change alters the text of a file `clm init` writes and projects already hold, the build of `init`'s requirement for the new text also writes the `upgrade` migration that brings the old text to it, in the same branch, and brings `upgrade`'s existing tests that the new migration makes fail (one expecting an earlier migration's rewrite of the same file to be the last, or the file to be up to date after it) to what `upgrade`'s requirements as applied state, naming each under `## Fixes in other components` in its pull request body. The migration's own requirement, `upgrade`'s, needs `init`'s, so `upgrade`'s build then adds the tests that prove the migration, and fixes it where they show it wrong.

**Why.** `clm/upgrade-corpus` requires every corpus project, upgraded, to hold exactly what `clm init` writes today, and `clm`'s tests run it on any land that changes `init` or `upgrade`, since `clm` depends on both. A template change alone fails it (no migration rewrites the old text), and a migration alone fails it too (it rewrites the old text to one `clm init` doesn't write yet). Neither order of two builds lands, so the two edits must reach `main` together. `upgrades-carry-migrations` already says the change that alters the file ships the migration; this says which build does. `20261109-land-still-sets-cargo`'s starter check script was the first change to meet it: earlier template changes either changed a file the comparison reads as parsed TOML (a comment in `clm.toml`) or added the migration for a template that had already changed.

**Why `init`'s build.** The template change is the cause, and `init` already depends on `upgrade` (it stamps `upgraded` with `latest()`), so its build reads `upgrade`'s list of migrations anyway. The migration's text is fixed by the requirement as written (its `old` the released script, its `new` the one `init`'s requirement states), so the build decides nothing of `upgrade`'s behaviour that the spec leaves open.

This is an exception of `path-ownership`, beside the re-cited markers and the capability tests, and refines `path-ownership-by-prompt`: the build prompt states it, and the build's review and spec audit read the migration under `## Fixes in other components`.

**Rejected.**
- *Leave the template file out of the corpus comparison while the two builds land.* Weakens the check `upgrades-carry-migrations` rests on, for a window nobody would remember to close.
- *Put the migration in `init`.* `upgrade` holds every migration so the lock and the stamp see them all; `upgrade` can't depend on `init` (a cycle), which is why a migration embeds its texts.
