---
component: plugin
---

## ADDED Requirements

### Requirement: upgrade-skill
The plugin SHALL ship `skills/upgrade/SKILL.md`, with a `description` in its frontmatter saying it brings a project up to date with the `clm` it runs. It SHALL tell Claude to:
- run `clm upgrade` and, when it prints `up to date`, say so and stop;
- cut a branch `upgrade/<YYYYMMDD>` from `origin/main` in a worktree of its own (`session-worktrees`), and work there;
- run `clm upgrade --apply`;
- for each `note <id>` it prints, read the note with `clm upgrade --note <id>`, show the human the edit it proposes beside the file's current text, and make it on their yes, or run `clm upgrade --decline <id>` on their no, an edit to `ARCHITECTURE.md` or `CAPABILITIES.md` being one the human then approves again with `clm approve`;
- when `clm upgrade` still lists a `required` migration after `--apply`, or a migration fails, report it with `clm defect`, naming `upgrade` with `--component`, and tell the human that `skip_migrations` in the clone's `clm.local.toml` lets this clone run meanwhile, never setting it without their yes;
- run `clm check`; commit the changes outside spec paths on the branch and land it when there are any, since mechanical migrations are code and a note's edit is spec, which `clm gate` refuses together as `mixed`; then commit the rest, with `clm.toml`'s `upgraded` and `declined_migrations` lines, on a branch `upgrade/<YYYYMMDD>-notes` cut from the new `origin/main`, and land it; landing each by the first line of `clm gate --base origin/main`, as `propose` lands a branch, asking for the human's `clm approve` when it prints `spec review`.

#### Scenario: the skill
- GIVEN the plugin's `skills/upgrade/SKILL.md`
- WHEN it is read
- THEN it names `clm upgrade --apply`, `clm upgrade --note`, `clm upgrade --decline`, `clm defect`, `skip_migrations`, `mixed` and `clm land`

#### Needs
- clm/upgrade-command
- gate/upgrade-stamp-left-out
- config/upgrade-settings

#### Serves
- capability/upgrade-clm

### Requirement: builder-check-asks-migrations
`skills/propose/SKILL.md` SHALL tell Claude, in the step that runs the builder check, to ask the subagent whether the change alters a file `clm init` writes, a settings key, or a format `clm` reads that projects set up by a published release already hold, and, when it does, to add to the change a requirement of `upgrade` for the migration: its level, its `done` check, and its `apply` or `note` (`upgrades-carry-migrations`). A `required` migration is always an `apply`.

#### Scenario: the question
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN its builder-check step is read
- THEN it names `clm init`, the migration and `upgrade`

#### Serves
- architecture/upgrades-carry-migrations
