---
component: clm
---

## MODIFIED Requirements
### Requirement: renamed-commands <!-- base: f600d2ffe6070005 -->
`clm rebase`, `clm wrap-up` and `clm pr` SHALL be the top-level commands that rebase a branch onto `origin/main` (`sync`), wrap up a session (`orchestrator.checkpoint`) and open or edit a branch's pull request (`host`'s `pr open` and `pr body`), each taking the arguments and subcommands, and printing the output and exiting with the codes, that its component states, and `clm --help` SHALL list them. Run as `clm rebase`, the outcome line `sync/outcome-line` states SHALL start with `rebase`: `rebase ok`, `rebase incomplete` or `rebase failed`. `clm wrap-up`'s report SHALL end with `wrap-up ok` or `wrap-up incomplete` (`orchestrator.checkpoint/checkpoint`). A message the root prints that names the command, such as the `<command>: <reason>` prefix of a failure on stderr, SHALL name it by its new name.

#### Scenario: rebase by its new name
- GIVEN a branch with one commit on top of `origin/main` that passes `clm check`
- WHEN `clm rebase` runs
- THEN the branch is rebased as `sync/rebase` states, its last line of stdout is `rebase ok`, and it exits 0

#### Scenario: wrap-up by its new name
- GIVEN a clone with no landed work and nothing that isn't safe on `origin`
- WHEN `clm wrap-up` runs
- THEN its last line is `wrap-up ok` and it exits 0

#### Scenario: pull request with no host
- GIVEN `host = "none"`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN it prints `none`, exits 0, and runs no `gh`

#### Needs
- orchestrator.checkpoint/checkpoint

#### Serves
- architecture/plain-terms

### Requirement: old-command-names-retired <!-- base: bb2bff6f2765b102 -->
The old names of the commands `renamed-commands` names, the subcommands `sync`, `checkpoint` and `page`, SHALL no longer run: `clm` given one of them SHALL fail as it fails for any unknown subcommand. `clm/retired-terms`'s list SHALL hold each old name written after `clm` and a space, and each outcome line that starts with an old name, so none comes back (`plain-terms`). No `clm:remove-by 20261101-rename-commands-contract` marker SHALL be left.

#### Scenario: an old name is refused
- GIVEN a branch with one commit on top of `origin/main`
- WHEN `clm` runs with the subcommand `sync`
- THEN it exits non-zero naming the unknown subcommand, and the branch is unchanged

#### Scenario: the old names retired
- GIVEN the repository
- WHEN the retired-terms test runs
- THEN its list holds the three old commands and the five old outcome lines, and it finds none of them

#### Needs
- orchestrator.checkpoint/checkpoint
- plugin/renamed-commands
- orchestrator.run/prompt-commands
- orchestrator.triage/draft
- orchestrator.triage/fold
- gate/up-to-date
- land/derived-specs
- land/land
