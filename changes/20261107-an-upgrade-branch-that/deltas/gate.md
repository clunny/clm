---
component: gate
base: { contract: "2ffdc2ed31fcfb07" }
---

## MODIFIED Contract
`clm gate --base <ref> [--approved]` decides whether `HEAD` may merge into `<ref>`. Its first line is the branch's kind, `spec review` for a spec branch that needs a human's review, `spec` for one that doesn't, `code` or `mixed`, then any violations in `check`'s format, and it exits 0 to allow or 1 to block. `--approved` says the ledger holds an approval of this diff that the repository's policy accepts; `clm land` computes it from the fold (`land-locally`), and the gate itself reads no ledger and no host.

`clm fingerprint`, the fingerprint an approval is pinned to, is the child `gate.fingerprint`'s (crate `clm-fingerprint`). `clm-gate` re-exports its `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint`, so `land`, `ledger`, `web` and `crates/clm` call them as `clm_gate::...` (`fingerprint-reexported`).

`clm-gate` also offers `settings_unchanged(before, after)`: given `clm.toml`'s bytes on each side of a diff (`Option<&[u8]>`, `None` where the file doesn't exist), whether its settings are the same once the upgrade notes `upgraded` and `declined_migrations`, comments and formatting are set aside, false when either side isn't UTF-8 TOML. `land` reads it to count a land whose `clm.toml` changed only so as checked (`land/settings-unchanged-is-checked`).

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
