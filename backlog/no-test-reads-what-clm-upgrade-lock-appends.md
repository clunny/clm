---
title: "No test reads what clm upgrade --lock appends, and check_lock is public for its own tests only"
kind: bug
components: [upgrade, clm]
origin: 20261113-phase-2-delete
---

`clm.toml` allows `clm_upgrade::check_lock` in the `unreferenced-public-items` detector "until `clm upgrade --lock` calls `append_lock`". It does now, so the condition is met, but deleting the line alone fails the detector: only the crate's own unit tests call `check_lock`.

Two things are owed. `upgrade/shipped-migrations-frozen`'s scenario "locking a release" runs `clm upgrade --lock`, and the test of the command in `crates/clm/tests/upgrade.rs` asserts only that it exits 0 and the lock file exists, not the line it appended. And `check_lock` should stop being public, or be used from outside its library by that test, so the allowance can go in the same diff (`unreferenced-allow-then-narrow`).
