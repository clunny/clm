---
component: config
---

## MODIFIED Requirements

### Requirement: host-setting <!-- base: 61abfd0c88e08e31 -->
Loading SHALL take `host` only from the repository's `clm.toml` or the clone's `clm.local.toml`, defaulting to `github`, SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `host` for a value other than `none`, `git`, `github` or `azure`, and SHALL fail naming the user's `config.toml` and the key when that file sets it. The `azure` value widens `Host`, which `clm-host`'s `adapter` matches exhaustively, so this requirement's build adds there the arm that gives the `Azure` adapter for it, already built (`host/azure-pull-requests`) and named for that value by `host/azure-selected`, as a forced fix (`forced-fixes-cover-additions`).

#### Scenario: default
- GIVEN a repository with no `clm.toml`
- WHEN `clm config get host` runs
- THEN it prints `github` and `default`

#### Scenario: from the repository
- GIVEN a repository `clm.toml` with `host = "git"`
- WHEN `clm config get host` runs
- THEN it prints `git` and the file's path

#### Scenario: azure
- GIVEN a repository `clm.toml` with `host = "azure"`
- WHEN `clm config get host` runs
- THEN it prints `azure` and the file's path

#### Scenario: unknown host
- GIVEN a repository `clm.toml` with `host = "gitlab"`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `host`

#### Scenario: unknown host set locally
- GIVEN a repository `clm.toml` with `host = "github"` and a `clm.local.toml` with `host = "gitlab"`
- WHEN the settings are loaded
- THEN it fails naming `clm.local.toml` and `host`

#### Scenario: from the user's file
- GIVEN a user `config.toml` setting `host = "none"`
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `host`

#### Serves
- capability/git-host
- architecture/host-adapters

#### Needs
- host/azure-pull-requests
- host/azure-other-operations
