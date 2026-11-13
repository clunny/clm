---
component: init
---

## MODIFIED Requirements
### Requirement: init-seeds-permissions <!-- base: 649ed0563f92d080 -->
When the CLM repository has no `specs/SPEC.md` as it starts, `clm init` SHALL add to `permissions.allow` in `.claude/settings.json` at the CLM repository's root each of its unattended allow rules the array lacks, creating the file, `permissions` and `allow` when missing: for each of the `clm` subcommands `apply`, `branch`, `build`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `setup`, `verify` and `workaround`, the rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`. `setup` is among them because the build and draft prompts run `clm setup` before their first test run on a code branch (`tests-run-in-their-own-tree`), and auto mode's classifier denies it when no rule allows it, which leaves a draft's code-only item unfixed; this repository's own `.claude/settings.json` allows its two rules first (`clm/session-commands`), so the build that adds them to the list leaves that test passing, and that build writes the note migration that offers them to projects set up earlier, as `upgrade/migration-setup-allow-rules` states it (`template-change-ships-its-migration`, `clm/upgrade-covers-allow-rules`). `build` is among them because `/clm:build` runs every step of its loop through `clm build`, and the loop keeps running its steps while the human is away (`build-loop-in-clm`), and this repository's own `.claude/settings.json` allows its two rules first (`clm/session-commands`), so the build that adds them to the list leaves that test passing; that build writes the note migration that offers them to projects set up earlier, as `upgrade/migration-build-allow-rules` states it. `verify` is among them because `/clm:build`'s loop runs `clm verify` when the project's triggers say, `/clm:triage` runs it at its end, and a pipeline in `pipeline` mode writes the ledger through it (`verify-after-land`); the build that adds it also adds its two rules to this repository's own `.claude/settings.json`, which `clm/session-commands` keeps equal to the list. It SHALL keep every other key and rule of an existing file, SHALL add no rule naming `clm approve` or `git`. When the file isn't a JSON object, or its `permissions` or `permissions.allow` has another type, it SHALL leave the file unchanged and name `.claude/settings.json` and the rules in its next steps. When the CLM repository already has `specs/SPEC.md`, it SHALL leave `.claude/settings.json` unchanged, and when the file lacks one of the rules, its next steps SHALL name `.claude/settings.json` and each missing rule, as the last step before the push step.

#### Scenario: empty repository
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `.claude/settings.json` holds `permissions.allow` with exactly the twenty-six rules, `Bash(clm build)`, `Bash(clm build *)`, `Bash(clm land)`, `Bash(clm land *)`, `Bash(clm setup)`, `Bash(clm setup *)` and `Bash(clm verify *)` among them, none naming `approve` or `git`, and `clm check` exits 0

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

#### Needs
- clm/session-commands
