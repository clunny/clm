---
status: applied
resolves: [two-propose-requirements-name-the-test-file-their-tests-left]
---

# Two plugin.propose requirements stop naming the test file their tests left

## Intent

`plugin.propose/propose-adds-glossary-terms` and `plugin.propose/propose-adds-settings-sections` each say their test sits in `crates/clm/tests/plugin.rs`. Since `20261108-plugin-skill-children` split the plugin into children, `plugin.propose`'s Design (`tests-count-where-cited`) puts the component's tests in `crates/clm/tests/plugin_propose.rs`, and its build moved both tests there. This change drops the file path from both requirements, leaving the Design's Tests paragraph as the one place that names the file.

## Problem

- Both requirements name a file their tests no longer live in, contradicting the component's own Design, so a builder reading either is sent to the wrong file.

## Approaches

- **A. Name `crates/clm/tests/plugin_propose.rs` in each requirement.** Correct today, but repeats what the Design's Tests paragraph already says, and goes stale again if the file moves.
- **B. Drop the path (chosen).** The `req:` marker ties each test to its requirement, and the Design says where the component's tests sit; the requirement keeps only what the test must do: read the skill's text and fail naming the missing text.

## Outcome

- Each requirement says "A test, a function over the skill's text, SHALL read the skill and fail naming the missing text." The rest of each requirement, and every scenario, is unchanged.
- The build re-cites the two existing tests in `crates/clm/tests/plugin_propose.rs` at the new versions; no test's assertions change, and no capability test sees anything different.
- The `workaround` flag `backlog/workaround-plugin-propose-adds-glossary-terms-1.md` (two older skill tests exempt the glossary path) still applies to the reworded text, which asks the same of the skill, so it is re-recorded at the new version rather than deleted.

## Constraints

- `tests-count-where-cited`: the component's Design still places its tests in `plugin_propose.rs`.
- `versioned-markers`: the rewording changes both hashes, so both requirements go `behind` until their markers move.

## Edge cases and workarounds

None.
