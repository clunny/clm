---
status: done
resolves: [projects-set-up-before-the-unattended-allow-rules-get-no]
---

# A migration offering the unattended allow rules

## Intent

A project set up before `clm init` seeded the allow rules for unattended `clm` commands (`init-seeds-permissions`), or before `clm verify`'s two were added, is never told by `clm upgrade` or the session-start reminder that it lacks them. This change ships the migration `upgrades-carry-migrations` asks for: an advised note migration, `upgrade/migration-unattended-allow-rules`, pending while `.claude/settings.json` and `.claude/settings.local.json` together lack one of the rules, whose note lists them and which the human takes by hand or declines. `/clm:upgrade` learns not to edit the settings file itself (`plugin/upgrade-skill-settings-by-hand`). The corpus test, which leaves the settings file out and so missed the gap, declines the note, and a new test, `clm/upgrade-covers-allow-rules`, fails a land when a rule is added to `clm init`'s seeding with no migration that offers it.

## Problem

`20261105-unattended-steps-denied-in` had `clm init` seed the rules on a first run only and shipped no migration; `20261107-slow-tests-after-land` added `verify`'s rules the same way. An existing project learns of them only when `/clm:build` or `/clm:triage` runs `clm init --check-permissions` before a launch, or from an `init` rerun's next steps. In a downstream project builds ran in auto mode without them, so `backlog/check-subagent-honours-allow-rule` still can't be confirmed.

Why the corpus test didn't flag it, which the item left open: `corpus-skips-first-run-files` (change `20261106-corpus-entries-drift-from`, the human's choice) leaves `.claude/settings.json` out of the comparison, with "settings drift left to `clm init --check-permissions`". So `upgrades-carry-migrations` rested on review for this file, and the next rule added to the seeding would slip through the same way.

## Approaches

- **An advised note migration with no `old`/`new` (chosen).** The item's own proposal. Its `done` is `clm init --check-permissions` passing; `--apply` never writes the settings, so a rule a project removed on purpose doesn't come back (the reason `unattended-steps-run-clm` rejected rewriting the settings on every run), and the human can decline it. It keeps the human's answer in `20261106-corpus-entries-drift-from` that the settings file has no template to compare: this note checks rules, not text.
- **An apply migration adding the missing rules.** Rejected: puts back removed rules, writes the human's protected settings, and in a project set up before `v0.19.0` adds a file no component's `paths` hold, failing `clm check`.
- **A note whose `old` is the `v0.19.0` seeded file.** Rejected (`yagni`): Claude Code's own `enabledPlugins` makes an untouched settings file rare, and every older project still needs the note.

For the corpus gap:
- **A test that each seeded rule is offered by some migration (chosen):** remove each rule of `UNATTENDED_RULES` in turn from a fresh project and require `clm upgrade` to list a pending migration. It needs nothing new in `upgrade`'s Contract.
- **Bring `.claude/settings.json` back into the corpus comparison.** Rejected: the file has no fixed text (`corpus-skips-first-run-files`), and the human chose to leave it out.

## Outcome

- `upgrade/migration-unattended-allow-rules`: the note migration `20261107-projects-set-up-before`, holding its own copy of the twenty-two rules, `verify`'s included.
- `clm/upgrade-corpus`, modified: the corpus test declines that migration before checking `up to date`, and compares `clm.toml` with `declined_migrations` aside as well as `upgraded`.
- `clm/upgrade-covers-allow-rules`: the new enforcer.
- `plugin/upgrade-skill-settings-by-hand`: `/clm:upgrade` shows the missing rules, has the human add them, and lands a root `paths` entry first when the project's root lacks one.
- Decision `allow-rules-offered-by-note`.

## Constraints

- `upgrades-carry-migrations`: the drift in a file older projects hold gets a migration, and the next one is caught by a test.
- `shipped-migrations-frozen`: the rule list is frozen with the migration; a later rule ships a new migration.
- `unattended-steps-run-clm`: nothing writes `.claude/settings.json` but the human (and `clm init` on a first run).
- `land-locally`: the settings are the project's once written.

Known edge cases: a project set up before `v0.19.0` has no settings file and a root whose `paths` don't list one, so taking the note takes a spec change to the root's `paths` before the file is committed (the note and the skill say so). A project that keeps its rules only in `.claude/settings.local.json` counts as done, as `clm init --check-permissions` counts it. The capability test `capability/upgrade-clm`'s *skipping releases* must take this note's edit (write the rules, list the file in the root's `paths`) to reach `up to date`; the migration's requirement names that, and the unit fixtures in `crates/clm-upgrade/src/shipped_tests.rs` and `crates/clm/tests/hooks.rs`'s session-start fixture that stand for an up-to-date project gain a settings file holding the rules. Taking the note in a project before `v0.19.0` lands in three branches (code, notes with the root's `paths` change, then the settings file), which `plugin/upgrade-skill-settings-by-hand` states. No workarounds are known.

## Decisions

- `allow-rules-offered-by-note`: refines `unattended-steps-run-clm`, `corpus-skips-first-run-files` and `upgrade-migrations` (its `done` also reads the clone's `.claude/settings.local.json`, as the architecture audit asked it to say).

## Open questions

- **`upgrades-carry-migrations`' `#### Enforced by`** (architecture audit, `human-layer-proof`). `clm/upgrade-covers-allow-rules` enforces the constraint and lists it under `#### Serves`, but the change doesn't add it to the constraint's `#### Enforced by`, since editing `ARCHITECTURE.md` would make this change need review for a one-line listing. The reviewer may add it by hand later; until then the rule is traced only from its own side.
- **Build order** (builder check). `init/init-seeds-permissions` is `behind`: the code seeds twenty rules, not `verify`'s two. Until it is built a fresh project lacks two of the migration's rules, so the migration's `#### Needs` names it and `clm/upgrade-covers-allow-rules` needs it too. The order is `init/init-seeds-permissions`, then `upgrade/migration-unattended-allow-rules`, then the `clm` and `plugin` requirements.
- **A merge in `skills/upgrade/SKILL.md`** (builder check). `plugin/upgrade-skill-offers-verify`, also pending, edits the same file; the second build to land resolves it in `clm rebase`.
