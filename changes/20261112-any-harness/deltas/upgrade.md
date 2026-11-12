---
component: upgrade
---

## ADDED Requirements

### Requirement: migration-agents-md
An `advised` `apply` migration SHALL move a project's instructions to `AGENTS.md`, which every supported harness reads (`platform`). Its `done` SHALL be true when `CLAUDE.md` doesn't exist or holds the line `@AGENTS.md`. Its `apply` SHALL, when `AGENTS.md` is missing, write `CLAUDE.md`'s text to `AGENTS.md` unchanged and replace `CLAUDE.md` with the one line `@AGENTS.md`; and, when both files exist, add the line `@AGENTS.md` as `CLAUDE.md`'s first line, followed by a blank line, changing nothing else, so no instruction is lost or moved under the human's feet.

#### Scenario: CLAUDE.md alone
- GIVEN a project whose `CLAUDE.md` holds the instructions and no `AGENTS.md`
- WHEN `clm upgrade --apply` runs
- THEN `AGENTS.md` holds that text and `CLAUDE.md` is `@AGENTS.md`

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

#### Serves
- capability/upgrade-clm
- capability/work-from-any-harness
