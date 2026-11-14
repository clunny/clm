---
component: upgrade
---

## MODIFIED Requirements

### Requirement: migration-offer-verify <!-- base: 3243c239fd12f8e5 -->
An `advised` apply migration SHALL bring a project set up before `clm verify` the commented-out `[verify]` section `clm init` writes (`init/init-verify-comment`), appending it to `clm.toml`, after one blank line, when it holds no line that is `[verify]` or `# [verify]` ignoring surrounding whitespace. Its `done` SHALL be true when `clm.toml` holds such a line or doesn't exist. Its description's first line SHALL read `Slow tests can run after the land: offer [verify]`, so `/clm:upgrade`, seeing it applied, asks the human whether to set it. It changes no setting: the section stays commented until the human sets it.

#### Scenario: offered to an older project
- GIVEN a `clm.toml` with no `[verify]` line
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists the migration as `advised apply Slow tests can run after the land: offer [verify]`, the second prints `applied` with its ID, and `clm.toml` then ends with the commented section and loads with no `verify` setting

#### Scenario: done once set or commented
- GIVEN a `clm.toml` holding the line `# [verify]`
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Needs
- clm/upgrade-command
- init/init-verify-comment

#### Serves
- capability/slow-tests-after-land

### Requirement: migration-agents-md <!-- base: 41da745b3d009ed4 -->
An `advised` `apply` migration SHALL move a project's instructions to `AGENTS.md`, which every supported agent harness reads (`platform`). Its `done` SHALL be true when `CLAUDE.md` doesn't exist or holds the line `@AGENTS.md`. Its `apply` SHALL, when `AGENTS.md` is missing, write to `AGENTS.md` the text the current `clm init` writes there when `CLAUDE.md`'s text equals one a released `clm init` wrote, and `CLAUDE.md`'s text unchanged otherwise, and replace `CLAUDE.md` with the one line `@AGENTS.md`, so an unedited project ends as `clm init` writes it today (`clm/upgrade-corpus`) and an edited one keeps its edits; and, when both files exist, add the line `@AGENTS.md` as `CLAUDE.md`'s first line, followed by a blank line, changing nothing else, so no instruction is lost or moved under the human's feet. It SHALL supersede `20261105-clm-upgrade-2`, whose `done` reads `CLAUDE.md` for `/clm:help`, which the moved text no longer holds there.

#### Scenario: CLAUDE.md alone
- GIVEN a project whose edited `CLAUDE.md` holds the instructions and no `AGENTS.md`
- WHEN `clm upgrade --apply` runs
- THEN `AGENTS.md` holds that text and `CLAUDE.md` is `@AGENTS.md`

#### Scenario: an unedited CLAUDE.md
- GIVEN a project whose `CLAUDE.md` equals what `clm init` `v0.16.0` wrote, and no `AGENTS.md`
- WHEN `clm upgrade --apply` runs
- THEN `AGENTS.md` and `CLAUDE.md` equal what the current `clm init` writes

#### Scenario: both files
- GIVEN a project with its own `AGENTS.md` and a `CLAUDE.md` that doesn't import it
- WHEN `clm upgrade --apply` runs
- THEN `AGENTS.md` is unchanged and `CLAUDE.md` starts with `@AGENTS.md` followed by its old text

#### Scenario: already moved
- GIVEN a project whose `CLAUDE.md` is `@AGENTS.md`
- WHEN `clm upgrade` runs
- THEN the migration isn't pending

#### Needs
- clm/upgrade-command
- init/init-files

#### Serves
- capability/upgrade-clm
- capability/work-from-any-agent-harness
