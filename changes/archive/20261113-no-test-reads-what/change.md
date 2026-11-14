---
status: done
---

## Intent

This change edits the backlog only and writes no delta. It narrows the item `no-test-reads-what-clm-upgrade-lock-appends` to what is still owed, and files one CLM defect met while drafting, `a-resumed-draft-s-first-command-fails-when-the-clone-still-holds`.

The item asked for two things. One is done on `main`: commit `3b11efe0` made `clm_upgrade::check_lock` test-only and deleted its allowance from `clm.toml`. The other, a test of `clm upgrade --lock` in `crates/clm/tests/upgrade.rs` that reads the lines the command appends, is a code-only fix in `clm`, since `upgrade/shipped-migrations-frozen` already states the behaviour. It is not fixed here: `clm claim clm --fix` was refused because `impl/clm` is claimed by a build, so the item stays in the backlog, out of this change's `resolves`, for the next triage.
