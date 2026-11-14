---
status: proposed
resolves:
  - no-test-reads-what-clm-upgrade-lock-appends
---

## Intent

`clm upgrade --lock` has no test that reads the line it appends, and `clm_upgrade::check_lock` is public only for its own crate's tests, kept by an allowance in `clm.toml` whose condition is met. This change settles both.
