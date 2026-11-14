---
title: "No test reads what clm upgrade --lock appends, and check_lock is public for its own tests only"
kind: bug
components: [upgrade, clm]
origin: 20261113-phase-2-delete
---

One thing is still owed, and it is `clm`'s. `upgrade/shipped-migrations-frozen`'s scenario "locking a release" runs `clm upgrade --lock`, and the test of the command in `crates/clm/tests/upgrade.rs` (`lock_runs_on_the_upgrade_crate_and_refuses_a_too_new_project`) asserts only that it exits 0 and the lock file exists, not the lines it appended: one per migration the lock doesn't list, naming the version, with every existing line unchanged. The fix is code-only, a stronger test in `crates/clm/tests/upgrade.rs`.

The other half is done: `check_lock` is no longer public and its allowance in `clm.toml` is gone (commit `3b11efe0`, on `main`).
