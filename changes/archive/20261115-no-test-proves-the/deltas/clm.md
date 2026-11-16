---
component: clm
---

## ADDED Requirements

### Requirement: rebase-settings
For `clm rebase`, the root SHALL hand `clm-sync` the settings `config` loads from the CLM root `clm-sync` gives it, `health` among them, so the repository's `[health] reconsider` is the threshold at which `clm rebase` files a reconsider item (`sync/file-reconsider-item`). When they don't load, the root SHALL hand it the error as `clm check` prints a violation of `check.health/health-report`: `<source>: check.health/health-report: <error>`, the source being the file that failed to load, or `settings` when the error names none (`sync/judged-by-check`). Each scenario SHALL be proven by running the `clm` binary, with no user `config.toml` and no `clm.local.toml` for it to load.

#### Scenario: settings that can't be loaded
- GIVEN a fixture repository on branch `spec/x`, up to date with its `origin/main`, whose committed `clm.toml` sets `[component_size] spec_tokens = 0`
- WHEN `clm rebase` runs as a process there
- THEN its first line of standard output starts with the path of `clm.toml` in the repository, then `: check.health/health-report: `, and names `component_size`, with no part of the line under the temporary directory

#### Scenario: the repository's reconsider threshold
- GIVEN a fixture repository on branch `impl/a`, up to date with its `origin/main`, whose committed `clm.toml` sets `[health] reconsider = 2`, and requirement `a/x` with two committed `workaround` items naming it at its current version
- WHEN `clm rebase` runs as a process there
- THEN it prints `filed backlog/reconsider-a-x.md`, which the default threshold of 3 wouldn't

#### Serves
- capability/prove-by-tests
- capability/configure-clm
