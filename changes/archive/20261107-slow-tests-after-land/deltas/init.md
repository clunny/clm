---
component: init
---

## MODIFIED Requirements

### Requirement: init-seeds-permissions <!-- base: cd8215b5c59acb0b -->
When the CLM repository has no `specs/SPEC.md` as it starts, `clm init` SHALL add to `permissions.allow` in `.claude/settings.json` at the CLM repository's root each of its unattended allow rules the array lacks, creating the file, `permissions` and `allow` when missing: for each of the `clm` subcommands `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `verify` and `workaround`, the rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`. `verify` is among them because `/clm:build` and `/clm:triage` run `clm verify` at their end and a pipeline in `pipeline` mode writes the ledger through it (`verify-after-land`); the build that adds it also adds its two rules to this repository's own `.claude/settings.json`, which `clm/session-commands` keeps equal to the list. It SHALL keep every other key and rule of an existing file, SHALL add no rule naming `clm approve` or `git`. When the file isn't a JSON object, or its `permissions` or `permissions.allow` has another type, it SHALL leave the file unchanged and name `.claude/settings.json` and the rules in its next steps. When the CLM repository already has `specs/SPEC.md`, it SHALL leave `.claude/settings.json` unchanged, and when the file lacks one of the rules, its next steps SHALL name `.claude/settings.json` and each missing rule, as the last step before the push step.

#### Scenario: empty repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `.claude/settings.json` holds `permissions.allow` with exactly the twenty-two rules, `Bash(clm land)`, `Bash(clm land *)` and `Bash(clm verify *)` among them, none naming `approve` or `git`, and `clm check` exits 0

#### Scenario: plugins already enabled
- GIVEN a git repository with no `specs/`, holding an untracked `.claude/settings.json` that sets `enabledPlugins` and `permissions.allow` `["Bash(make *)", "Bash(clm land *)"]`
- WHEN `clm init` runs
- THEN `.claude/settings.json` still sets `enabledPlugins`, its `permissions.allow` starts with `Bash(make *)` and `Bash(clm land *)`, holds each rule once, and `clm check` exits 0

#### Scenario: settings that aren't JSON
- GIVEN a git repository with no `specs/` and a `.claude/settings.json` holding `{`
- WHEN `clm init` runs
- THEN it exits 0, `.claude/settings.json` is unchanged, and a next step names `.claude/settings.json` and `Bash(clm land *)`

#### Scenario: rerun leaves the settings alone
- GIVEN a repository initialised by an earlier `clm` with no `.claude/settings.json`, committed
- WHEN `clm init` runs
- THEN `git status` shows no change, and a next step, before the push step, names `.claude/settings.json`, `Bash(clm land *)` and `Bash(clm branch *)`

#### Scenario: rerun with the rules
- GIVEN a repository just initialised by `clm init`, committed
- WHEN `clm init` runs again
- THEN `git status` shows no change and no next step names `.claude/settings.json`

#### Serves
- capability/adopt-clm
- capability/build-from-spec
- capability/triage-unattended

## ADDED Requirements

### Requirement: init-verify-comment
The `clm.toml` that `clm init` writes SHALL end with a commented-out `[verify]` section, every line of it starting `# `, naming `check`, `mode` with its three values, `triggers` with its three values, `every` and `workflow`, and the guide page `ci-and-deployment`, so a project sees where slow tests after the land are set and `upgrade`'s offer of them (`upgrade/migration-offer-verify`) is done. Uncommented, its lines SHALL load as a valid `[verify]` table.

#### Scenario: written commented
- GIVEN an empty git repository
- WHEN `clm init --lang rust` runs
- THEN `clm.toml` holds the line `# [verify]` and no uncommented `[verify]`, loads with no `verify` setting, and names `ci-and-deployment`

#### Scenario: uncommented it loads
- GIVEN that `clm.toml` with the `[verify]` section's leading `# ` removed from each of its lines
- WHEN the settings are loaded
- THEN they hold a `verify` setting

#### Needs
- config/verify-setting

#### Serves
- capability/slow-tests-after-land
