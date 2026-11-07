---
component: gate
---

## ADDED Requirements

### Requirement: settings-unchanged
`clm-gate` SHALL offer `settings_unchanged(before, after)`, of `clm.toml`'s bytes on each side of a diff as `upgrade_stamp_only` takes them (`None` where the file doesn't exist), true exactly when both sides are UTF-8 and parse as TOML and their tables are equal once the top-level `upgraded` and `declined_migrations` are removed, a missing file counting as an empty table. So a change to comments, formatting or those two keys alone, or no change, gives true. `upgrade_stamp_only` SHALL return `settings_unchanged` and a difference between the parsed tables, so the two read settings alike. The function SHALL run no process and SHALL build for `wasm32-unknown-unknown`.

#### Scenario: the stamp only
- GIVEN a `clm.toml` and the same file with `upgraded` changed
- WHEN `settings_unchanged` runs on the two
- THEN it returns true, as `upgrade_stamp_only` does

#### Scenario: a comment only
- GIVEN a `clm.toml` and the same file with a commented-out table appended
- WHEN `settings_unchanged` and `upgrade_stamp_only` run on the two
- THEN `settings_unchanged` returns true and `upgrade_stamp_only` false

#### Scenario: a setting
- GIVEN a `clm.toml` and the same file with `[land] check` changed
- WHEN `settings_unchanged` runs on the two
- THEN it returns false

#### Scenario: unreadable
- GIVEN a `clm.toml` and a side that doesn't parse as TOML
- WHEN `settings_unchanged` runs on the two
- THEN it returns false

#### Serves
- capability/upgrade-clm
