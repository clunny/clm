---
component: plugin
---

## ADDED Requirements

### Requirement: upgrade-skill
The plugin SHALL ship `skills/upgrade/SKILL.md`, with a `description` in its frontmatter saying it brings a project up to date with the `clm` it runs. It SHALL tell Claude to:
- run `clm upgrade` and, when it prints `up to date`, say so and stop;
- cut a branch `upgrade/<YYYYMMDD>` from `origin/main` in a worktree of its own (`session-worktrees`), and work there;
- run `clm upgrade --apply` and commit what it changed; when that touched files other than `clm.toml`'s `upgraded` line, land the branch then, before any note's edit, since a mechanical change is code and a note's edit is spec, and cut the next branch `upgrade/<YYYYMMDD>-notes` from the new `origin/main`;
- for each migration it stopped at, print the note with `clm upgrade --note <id>`, show the human the edit it proposes beside the file's current text, make it only on their yes, and run `clm upgrade --done <id>`, then `clm upgrade --apply` again, landing as above whenever it makes a mechanical change, until `clm upgrade` prints `up to date`;
- when a migration fails or its `done` stays false after the edit, report it with `clm defect`, naming `upgrade` with `--component`, and tell the human that `skip_migrations` in `clm.local.toml` lets this clone run meanwhile, never setting it without their yes;
- run `clm check`, commit, and land the branch by the first line of `clm gate --base origin/main`, as `propose` lands a branch, asking for the human's `clm approve` when it prints `spec review`.

#### Scenario: the skill
- GIVEN the plugin's `skills/upgrade/SKILL.md`
- WHEN it is read
- THEN it names `clm upgrade --apply`, `clm upgrade --note`, `clm upgrade --done`, `clm defect`, `skip_migrations` and `clm land`

#### Needs
- clm/upgrade-command
- gate/upgrade-stamp-is-spec

#### Serves
- capability/upgrade-clm

### Requirement: builder-check-asks-migrations
`skills/propose/SKILL.md` SHALL tell Claude, in the step that runs the builder check, to ask the subagent whether the change alters a file `clm init` writes, a settings key, or a format `clm` reads that projects set up by a published release already hold, and, when it does, to add to the change a requirement of `upgrade` for the migration, with its level, its `done` check and its `apply` or `note` (`upgrades-carry-migrations`).

#### Scenario: the question
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN its builder-check step is read
- THEN it names `clm init`, the migration and `upgrade`

#### Serves
- architecture/upgrades-carry-migrations
