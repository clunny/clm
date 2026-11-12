---
component: orchestrator.run
---

## MODIFIED Requirements

### Requirement: setup-command <!-- base: 44d5d2929c4a38f2 -->
`clm setup [--root <dir>]` SHALL load the settings of the CLM root, `--root` defaulting to the one found from the current directory as every command's does, and, when `clm.toml` or `clm.local.toml` sets `[build] setup`, run that command through `sh -c` with that root as its working directory and its standard input closed, passing its standard output and standard error through, taking no lock and setting no variable but each variable of the `env` the settings give (`config/env-setting`), which replaces any value the caller's environment holds for that name; it SHALL exit 0 when the command exits 0, and otherwise exit 1 with a line on stderr saying `setup failed` and naming the command and its exit status, or `killed by signal <n>` when it has none. When no `[build] setup` is set, it SHALL print `no [build] setup` and exit 0, running nothing. When the settings can't be loaded, it SHALL exit 1 with the reason, running nothing. It SHALL fetch nothing, commit nothing and write nothing itself.

#### Scenario: setup runs in the root
- GIVEN a CLM repository whose `clm.toml` sets `[build] setup = "touch ready"`, and a subdirectory `src/` of it
- WHEN `clm setup` runs in `src/`
- THEN it exits 0, and the file `ready` exists at the repository's root and not in `src/`

#### Scenario: setup fails
- GIVEN a CLM repository whose `clm.toml` sets `[build] setup = "exit 3"`
- WHEN `clm setup` runs in it
- THEN it exits 1, and stderr says `setup failed` and names `exit 3` and the status 3

#### Scenario: nothing declared
- GIVEN a CLM repository whose `clm.toml` has no `[build]` table
- WHEN `clm setup` runs in it
- THEN it prints `no [build] setup` and exits 0

#### Scenario: unreadable settings
- GIVEN a CLM repository whose `clm.toml` isn't valid TOML
- WHEN `clm setup` runs in it
- THEN it exits 1 naming `clm.toml`

#### Scenario: setup gets the project's variables
- GIVEN a CLM repository whose `clm.toml` sets `[build] setup = "echo $RELAY_SERVER_IMAGE > seen"` and `[env]` with `RELAY_SERVER_IMAGE = "relay:amd64"`, a `clm.local.toml` with `[env]` setting `RELAY_SERVER_IMAGE = "relay:arm64"`, and `RELAY_SERVER_IMAGE=relay:shell` in the caller's environment
- WHEN `clm setup` runs in it
- THEN it exits 0, and `seen` holds `relay:arm64`

#### Needs
- config/build-setup-setting
- config/env-setting

#### Serves
- architecture/tests-run-in-their-own-tree
