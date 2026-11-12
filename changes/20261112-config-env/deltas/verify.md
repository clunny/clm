---
component: verify
---

## ADDED Requirements

### Requirement: verify-env
Every run of `[verify] check`, a bisect probe's included, SHALL set in the command's environment each variable of the `env` that the settings it follows give (`settings-from-main`): the `[env]` of the `clm.toml` committed at `origin/main`, merged with the clone's `clm.local.toml`'s as `config/env-setting` says, never the working tree's `clm.toml`'s, each replacing any value the caller's environment holds for that name. `CLM_SELECTION` and `CLM_LAND_DIR` (`verify-build-dir`) SHALL be as they are without it.

#### Scenario: main's table and the clone's
- GIVEN a `[verify] check` that writes `$A` and `$B` to a file outside the repository, a checkout whose `clm.toml` sets `[env]` with `A = "checkout"`, an `origin/main` whose `clm.toml` sets `[verify]` with that check and `[env]` with `A = "main"` and `B = "main"`, a `clm.local.toml` with `[env]` setting `B = "local"`, `A=caller` in the caller's environment, and a run due
- WHEN `clm verify --due` runs in the checkout
- THEN the file holds `main` for `A` and `local` for `B`

#### Needs
- config/env-setting
