---
status: done
resolves: [upgrade-corpus-test-needs-both-allow-rule-migrations-declined]
---

## Intent

Name the upgrade-corpus test among the fixtures a new allow-rules note migration's build updates. `clm/upgrade-corpus` states that the test declines each pending note migration offering the allow rules `clm init` seeds, both `20261107-projects-set-up-before` and `20261112-a-draft-s-clm`, and that the build writing a further one adds its ID there. `upgrade/migration-setup-allow-rules` names the corpus test beside `project()`, `clm_repo` and *skipping releases*.

## Problem

`upgrade/migration-setup-allow-rules` lists the fixtures its build must update: `shipped_tests.rs`'s `project()`, `hooks.rs`'s `clm_repo` and the capability test's *skipping releases*. It leaves out `crates/clm/tests/upgrade.rs`'s corpus test. `clm/upgrade-corpus` names only `20261107-projects-set-up-before` as the migration to decline. When the new migration shipped, every corpus entry ended with it pending, so none was `up to date`. `init`'s build declined both IDs in the test as a forced fixture fix that no requirement stated. The code is already right (`ALLOW_RULES_IDS` holds both IDs). The rule text lags behind it, and the next allow-rules migration would hit the same gap.

## Approaches

1. **State both IDs, and the duty to extend the list, in `clm/upgrade-corpus`; name the corpus test in `upgrade/migration-setup-allow-rules`** (chosen). This brings the rules up to the code as built. It also tells the next change that adds such a migration where its ID goes. Both requirements go `behind`, and their builds re-cite their tests at the new versions. No code changes.
2. **Have the test decline every pending note migration whose `file` is `.claude/settings.json`, read from `clm-upgrade`'s list.** No list to maintain, but it would hide a future note on that file that the corpus should not decline. It also widens the test's reach into `upgrade`'s internals for a gap that has come up once (`yagni`). Rejected.
3. **Widen `template-change-ships-its-migration`'s exception in `ARCHITECTURE.md` to cover `clm`'s corpus test as well as `upgrade`'s tests.** That is a general rule for a narrow case, and it would edit `ARCHITECTURE.md` for a gap that has come up once. Rejected for now: see `## Open questions`.

## Outcome

- `clm/upgrade-corpus` names both allow-rules migrations, states that the test keeps their IDs in one list, and needs `upgrade/migration-setup-allow-rules`.
- `upgrade/migration-setup-allow-rules` names the corpus test's list among the fixtures its build updates.
- The builds of both requirements only re-cite: `crates/clm/tests/upgrade.rs` already declines both IDs and asserts neither is pending for a fresh entry.

## Edge cases and workarounds

None.

## Open questions

- **`path-ownership`** (architecture audit). Both requirements have `init`'s build edit `crates/clm/tests/upgrade.rs`, which is `clm`'s. That is like the `clm_repo` fixture in `crates/clm/tests/hooks.rs`, which `upgrade/migration-setup-allow-rules` already sends that build to on `main`. `template-change-ships-its-migration`'s exception names only `upgrade`'s tests. The decline-list edit is not plainly a forced test-fixture fix in the sense `forced-fixes-cover-additions` gives. The change has the build name both edits under `## Fixes in other components`, as every exception requires, and states the edit in the requirements. It leaves open whether `ARCHITECTURE.md`'s exception should widen to cover `clm`'s upgrade tests (rejected Approach 3). That hand edit needs review, and this change does not make it.
