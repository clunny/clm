---
component: clm
---

## ADDED Requirements
### Requirement: affected-why-command
`clm affected` SHALL take `--why <path>`, passing it with `--base` and `--root` to `clm-affected`'s `why` and printing its lines, and SHALL exit 1 naming both flags, running nothing, when `--readers` is given too.

#### Scenario: the chain
- GIVEN a repository with components `a`, owning `a/src/lib.rs`, and `b`, owning `b/src/lib.rs`, `a` depending on `b`, on a branch that changes only `b/src/lib.rs`
- WHEN `clm affected --why a/src/lib.rs` runs
- THEN stdout is `a/src/lib.rs owned by a`, `a depends on b` and `b owns b/src/lib.rs (changed)`, and it exits 0

#### Needs
- check.affected/why-chain

#### Serves
- capability/slow-tests-after-land

### Requirement: health-land-target
`clm health` SHALL set `clm_health::Settings`' `land_check_target_seconds` from the loaded settings' `[land] check_target_seconds`, its default when unset.

#### Scenario: the target reaches health
- GIVEN a repository `clm.toml` with `[land]`, `check = "true"` and `check_target_seconds = 45`, and a ledger with one timed land of 50 seconds
- WHEN `clm health` runs
- THEN its `land-time` summary is `1 of 1 timed lands over 45s`

#### Needs
- config/land-check-setting
- check.health/land-time-signal

#### Serves
- capability/know-health
