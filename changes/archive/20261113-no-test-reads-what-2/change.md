---
status: done
---

## Intent

This change edits the backlog only and writes no delta. It files one CLM defect met while drafting, `a-draft-is-launched-for-a-code-only-bug-while-its-component-is`.

The item it was drafted for, `no-test-reads-what-clm-upgrade-lock-appends`, is untouched and out of this change's `resolves`. Its fix is code-only, a test in `crates/clm/tests/upgrade.rs` that reads the lines `clm upgrade --lock` appends, and `upgrade/shipped-migrations-frozen`'s scenario "locking a release" already states the behaviour. `clm claim clm --fix` was refused because `impl/clm` is claimed by a build, as it was for the draft `20261113-no-test-reads-what` earlier the same day, so the item stays in the backlog for a later triage. The defect filed here is that repeat: a draft spent on a bug whose component can't be claimed.
