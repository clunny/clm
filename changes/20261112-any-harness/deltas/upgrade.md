---
component: upgrade
---

## ADDED Requirements

### Requirement: superseded-notes
A migration SHALL be able to name the earlier note migrations it supersedes, and `pending(root)` SHALL leave out a note migration that a migration whose `done` is true supersedes, since its edit is to a text the later migration has moved. A shipped migration's own files stay as they shipped (`shipped-migrations-frozen`).

#### Scenario: a superseded note
- GIVEN note migration `n` not done, and a later migration `m` that supersedes `n` and is done
- WHEN `pending` runs
- THEN it doesn't return `n`

#### Scenario: not yet superseded
- GIVEN the same, with `m` not done
- WHEN `pending` runs
- THEN it returns `n`


### Requirement: migration-agents-md
An `advised` `apply` migration SHALL move a project's instructions to `AGENTS.md`, which every supported harness reads (`platform`). Its `done` SHALL be true when `CLAUDE.md` doesn't exist or holds the line `@AGENTS.md`. Its `apply` SHALL, when `AGENTS.md` is missing, write to `AGENTS.md` the text the current `clm init` writes there when `CLAUDE.md`'s text equals one a released `clm init` wrote, and `CLAUDE.md`'s text unchanged otherwise, and replace `CLAUDE.md` with the one line `@AGENTS.md`, so an unedited project ends as `clm init` writes it today (`clm/upgrade-corpus`) and an edited one keeps its edits; and, when both files exist, add the line `@AGENTS.md` as `CLAUDE.md`'s first line, followed by a blank line, changing nothing else, so no instruction is lost or moved under the human's feet. It SHALL supersede `20261105-clm-upgrade-2`, whose `done` reads `CLAUDE.md` for `/clm:help`, which the moved text no longer holds there.

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
- init/claude-md-names-the-skills

#### Serves
- capability/upgrade-clm
- capability/work-from-any-harness
