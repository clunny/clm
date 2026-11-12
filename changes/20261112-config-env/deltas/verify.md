---
component: verify
---

## ADDED Requirements

### Requirement: verify-env
Every run of `[verify] check`, a bisect probe's included, SHALL set in the command's environment each variable of the `env` (`config/env-setting`) of the commit it checks out (`origin/main`'s head, the commit `--commit` names, or the probe's land): the one `config`'s `committed_env` gives for the CLM root's `clm.toml` at that commit, with the clone's `clm.local.toml`, never the working tree's `clm.toml`, each replacing any value the caller's environment holds for that name. When there is a last green commit and the `env` of the commit it verifies differs from the one `committed_env` gives for the green commit's `clm.toml` with the same `clm.local.toml`, every run of the check for that verification, its probes included, SHALL have `CLM_SELECTION` unset, since a variable can change any test's result. `CLM_LAND_DIR` (`verify-build-dir`) SHALL be as it is without it.

#### Scenario: main's table and the clone's
- GIVEN a `[verify] check` that writes `$A` and `$B` to a file outside the repository, a checkout whose `clm.toml` sets `[env]` with `A = "checkout"`, an `origin/main` whose `clm.toml` sets `[verify]` with that check and `[env]` with `A = "main"` and `B = "main"`, a `clm.local.toml` with `[env]` setting `B = "local"`, `A=caller` in the caller's environment, and a run due
- WHEN `clm verify --due` runs in the checkout
- THEN the file holds `main` for `A` and `local` for `B`

#### Scenario: a probe takes its land's variables
- GIVEN a passing record, then a land setting `[env]` `A = "1"`, then a land setting `A = "2"`, and a `[verify] check` that fails when `$A` is `2`
- WHEN `clm verify --due` runs and bisects
- THEN the probe of the first land ran with `A` set to `1` and passed, and the record names the second land as the culprit

#### Scenario: a changed variable runs every test
- GIVEN a passing record, then one land that changes only `clm.toml`'s `[env]`, and a `[verify] check` that writes whether `CLM_SELECTION` is set
- WHEN `clm verify --due` runs
- THEN the check ran with `CLM_SELECTION` unset

#### Needs
- config/env-setting

## MODIFIED Requirements

### Requirement: settings-from-main <!-- base: 325c67ff384736f7 -->
Every `clm verify` SHALL take `host` from the settings of the CLM root it runs in, and, after fetching `origin`, on every host, every other setting it uses (`[verify]`, `[affected]`) but `[env]`, which `verify-env` takes from the commit it checks, from `config`'s `load_committed` with the text of the CLM root's `clm.toml` committed at `origin/main`, `None` when `origin/main` or that file doesn't exist, and `source` `origin/main:clm.toml`. It SHALL never take `[verify]` or `[affected]` from the working tree's `clm.toml`, and SHALL decide whether `[verify]` is set only after the fetch. When those settings set no `[verify]` it SHALL print `verify off: no [verify] in origin/main:clm.toml` and exit 0, running and appending nothing; when they fail to load it SHALL exit 1 with `config`'s message, which names `origin/main:clm.toml`. A fetch that fails SHALL exit 2, as git failures do.

#### Scenario: a checkout behind main
- GIVEN a clone whose checkout's `clm.toml` sets no `[verify]`, and an `origin/main` ahead of it whose `clm.toml` sets `[verify]` with `mode = "local"` and `triggers = ["work-done"]`, with one land since a passing `verified` record
- WHEN `clm verify --due --event work-done --dry-run` runs in the checkout
- THEN it prints `verify due work-done` and exits 0

#### Scenario: turned off on main
- GIVEN a checkout whose `clm.toml` sets `[verify]` with a `check` that writes a file, and an `origin/main` whose `clm.toml` sets none
- WHEN `clm verify --due --event work-done` runs in the checkout
- THEN it prints `verify off: no [verify] in origin/main:clm.toml`, exits 0, and the file isn't written

#### Scenario: an invalid setting on main
- GIVEN an `origin/main` whose `clm.toml` sets `[verify]` with no `check`
- WHEN `clm verify --due` runs
- THEN it exits 1 naming `origin/main:clm.toml` and `verify`, and appends nothing

#### Needs
- config/committed-settings
- config/verify-setting
