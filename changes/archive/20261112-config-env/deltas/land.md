---
component: land
---

## ADDED Requirements

### Requirement: check-env
Every run of `[land] check`, by `clm land <branch>` or by `clm land --check-only`, SHALL set in the command's environment each variable of the `env` (`config/env-setting`) of the tree it checks, the one `config`'s `committed_env` gives for the CLM root's `clm.toml` in that tree (the merge commit's for a land, the working tree's for `--check-only`) with the clone's `clm.local.toml`, replacing any value the caller's environment holds for that name. The command a `checked` record names, and that every lookup of a `checked` record with the same command compares, SHALL be `[land] check` preceded by `<NAME>=<value> ` for each of those variables in byte order of the names, or `[land] check` alone when there is none, so a record counts only for a check that ran with the same variables. When a land's `env` differs from the one `committed_env` gives for the fetched base's `clm.toml` with the same `clm.local.toml`, it SHALL run `[land] check` with `CLM_SELECTION` unset, since a variable can change any test's result. `CLM_SLOT` (`check-slot`), `CLM_LAND_DIR` (`land-dir-env`), the removal of `CI` and the check lock (`check-lock`) SHALL be as they are without it.

#### Scenario: the clone's value wins
- GIVEN a `[land] check` that writes `$RELAY_SERVER_IMAGE` to a file outside the repository, a `clm.toml` with `[env]` setting `RELAY_SERVER_IMAGE = "relay:amd64"`, a `clm.local.toml` with `[env]` setting `RELAY_SERVER_IMAGE = "relay:arm64"`, and `RELAY_SERVER_IMAGE=relay:shell` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds `relay:arm64`, and the `checked` record it appends names the command `RELAY_SERVER_IMAGE=relay:arm64 ` followed by the check

#### Scenario: no table
- GIVEN the same check, no `[env]` in either file, and `RELAY_SERVER_IMAGE=relay:shell` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds `relay:shell`, and the `checked` record names the check alone

#### Scenario: a record under other variables
- GIVEN a branch whose merged tree's code key has a `checked` record naming `[land] check` alone, and a `clm.local.toml` with `[env]` setting `A = "1"`
- WHEN `clm land` runs
- THEN it runs `[land] check` as for unchecked code, and appends a `checked` record naming `A=1 ` followed by the check

#### Scenario: a changed variable runs every test
- GIVEN a branch that changes only `clm.toml`'s `[env]`, and a `[land] check` that writes whether `CLM_SELECTION` is set to a file outside the repository
- WHEN `clm land` runs
- THEN the file says it is unset

#### Needs
- config/env-setting
