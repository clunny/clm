---
title: "The first migration in init's own list fails the root's test of `clm upgrade --lock`"
kind: deferred
components: [clm, init]
origin: 20261120-wave-2-rest-by-the-rule
---

`migrations-live-with-init` has every migration written today live in `init`'s list, which is empty so far. The root's test `crates/clm/tests/upgrade.rs::lock_runs_on_the_upgrade_crate_and_refuses_a_too_new_project` copies only `clm-upgrade`'s migration directories into its fixture and expects `clm upgrade --lock 0.1.0` to exit 0. Once `init` holds a migration, the command also hashes `crates/clm-init/src/migrations/<id>/`, which the fixture lacks, and exits 1 (`crates/clm/src/upgrade_command.rs:45-50`).

So the first change that adds a migration to `init` can't be built: `init`'s build may not edit the root's test (`own-files-at-the-gate`), and nothing orders the root's mend first. Found on 2026-11-21 by the second builder check of phase 5's wave 2, which had drafted such a migration for a comment in `clm.toml` and withdrew it on the human's waiver.

The mend is the root's: the fixture copies `init`'s migrations too, or the test builds its own lists. It is due before the next migration is proposed, and at the latest in wave 7, the root's pass. A change that adds a migration to `init` before then needs this first, as a small branch that lands before it (`expand-contract`).

**Deferred** (2026-11-21, the human): taken up in wave 7 of phase 5, the root's pass (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`). Until then, `clm triage` lists it as `deferred` and the build loop drafts nothing for it.
