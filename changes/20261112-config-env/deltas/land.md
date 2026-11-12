---
component: land
---

## ADDED Requirements

### Requirement: check-env
Every run of `[land] check`, by `clm land <branch>` or by `clm land --check-only`, SHALL set in the command's environment each variable of the `env` that the settings it takes `[land] check` from give (`config/env-setting`), replacing any value the caller's environment holds for that name. `CLM_SLOT` (`check-slot`), `CLM_SELECTION`, `CLM_LAND_DIR` (`land-dir-env`) and the removal of `CI` SHALL be as they are without it, and the check lock (`check-lock`) unchanged.

#### Scenario: the clone's value wins
- GIVEN a `[land] check` that writes `$RELAY_SERVER_IMAGE` to a file outside the repository, a `clm.toml` with `[env]` setting `RELAY_SERVER_IMAGE = "relay:amd64"`, a `clm.local.toml` with `[env]` setting `RELAY_SERVER_IMAGE = "relay:arm64"`, and `RELAY_SERVER_IMAGE=relay:shell` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds `relay:arm64`

#### Scenario: no table
- GIVEN the same check, no `[env]` in either file, and `RELAY_SERVER_IMAGE=relay:shell` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds `relay:shell`

#### Needs
- config/env-setting
