---
status: applied
priority: P2
resolves: [projects-upgrading-past-the-check-lock-lift-aren-t-told-their-tests-must-overlap]
---

# A migration telling older projects their tests must now overlap

## Intent

Tell a project upgrading `clm` past the lifted pre-land check lock that its tests must now run side by side. `upgrade/migration-tests-overlap` adds an advised note migration, `20261107-projects-upgrading-past-the`, that `clm upgrade`, the session-start reminder and `/clm:upgrade` show to every project set up before it. Its note says that builds' pre-land checks no longer wait for each other, that the project's tests must pass while other runs overlap (`tests-run-concurrently`) with the ports and paths they must know in advance keyed on `CLM_SLOT`, how to look for fixed ports, paths and shared servers and file each as a bug, and what to do meanwhile: replace the constraint when the project keeps its runs apart another way, or run one build at a time. The note asks for no file edit, so its `done` reads the upgrade stamp (decision `stamp-taken-notes`), and `upgrade/upgrade-apply` is modified to print a note before the stamp it writes takes it. `plugin/upgrade-skill-notes-without-edit` has `/clm:upgrade` show such a note, offer to do what it advises, and decline nothing; the glossary's *Migration* entry and decision `upgrade-migrations` say a note may ask for no edit. Resolves `backlog/projects-upgrading-past-the-check-lock-lift-aren-t-told-their-tests-must-overlap`.

## The human's answers

On backlog/projects-upgrading-past-the-check-lock-lift-aren-t-told-their-tests-must-overlap: asked whether to make a new clm release once the P1 pre-land check fix lands, the human asked "question, after this P1 fix lands, should we make a new clm release so that users can benefit from it now?"; told that lifting the pre-land check lock needs an upgrade note migration telling projects their tests must run side by side (`tests-run-concurrently`, `CLM_SLOT`), the human said:

> yes, draft the migration

## Problem

`20261107-builds-pre-land-checks` has a build's `clm land --check-only` in a build place stop taking the clone's check lock (`land/check-lock`). That lock was what kept a project's checks from overlapping when parallel builds ran them (`check-runs-one-at-a-time`). A project whose tests still bind fixed ports or paths will then fail intermittently between parallel builds, for reasons unrelated to any branch. `20261107-parallel-builds-test-runs` stated the rule and gave builds `CLM_SLOT`, but left a migration out until a project trips on it (`yagni`); lifting the lock is that moment, and an existing project learns of the rule only from `clm guide constraints` and its builds' context bundles.

## Approaches

- **A note migration taken by the stamp (chosen).** The note is the item's ask (`upgrades-carry-migrations`, `upgrade-note-and-decline`). What a project does about it lives in its tests, which no `done` check can read, so `done` is the stamp: a project `clm init` set up since is done, and an older one sees the note once, in `clm upgrade`, the session-start reminder and `--apply`'s output, until the upgrade branch lands its stamp. It needs `--apply` to choose its `note` lines before it writes the stamp, a one-line reorder.
- **A note whose `done` is never true, taken by declining.** Rejected: "decline" would mean "read", and the reminder would name it in every project, `clm init`'s own included, which `migration-list`'s *a fresh project is done* forbids.
- **A marker in `clm.toml` or the check script, written by `clm init` and by hand.** Rejected: a template change every older project must migrate, to record that someone read a note. `scripts/clm-check.sh` is in the corpus, which is why `20261107-parallel-builds-test-runs` dropped a comment there.
- **An advised apply migration that changes nothing,** with `/clm:upgrade` reacting to it as it does to `migration-offer-verify`. Rejected: an apply migration has no note `clm upgrade --note` prints.
- **Have `/clm:upgrade` always search the project and file the collisions as bugs, as `/clm:init` does (`plugin/init-skill-concurrent-tests`).** Not taken as a rule of its own: the skill's step for notes would otherwise propose an edit and a decline this note doesn't have (architecture audit), so `plugin/upgrade-skill-notes-without-edit` covers a note asking for no edit generally, offering to do what the note advises, the search and the bug items here, on the human's yes.

## Outcome

- `upgrade/migration-tests-overlap` (added): the note migration, with `file` `clm.toml`, no `old`/`new`, `done` reading the stamp, and the note's first line `Builds' checks now overlap: your tests must pass side by side`. It needs `land/check-lock`, `land/check-slot` and `orchestrator.run/prompt-slot`, so no release ships the note before the lift it describes and the `CLM_SLOT` it names. Its `done` is true too when `clm.toml` doesn't exist, as `migration-offer-verify`'s is, and `shipped_tests.rs`'s `project()` is stamped with `latest()`.
- `plugin/upgrade-skill-notes-without-edit` (added): `/clm:upgrade` shows a note that edits no file, offers to do what it advises, and runs no `--decline` for it.
- `upgrade/upgrade-apply` (modified): `note <id>` lines are judged after the rewrites and before the stamp is written.
- `upgrade`'s Design says how a stamp-taken note works.
- Decision `stamp-taken-notes`; `upgrade-migrations` amended to point at it; the glossary's *Migration* entry covers a note that asks for no edit.

## Constraints

- `upgrades-carry-migrations`: the change in what an existing project's tests must do ships its migration.
- `shipped-migrations-frozen`: the note's text is frozen once a release ships it.
- `tests-run-concurrently`, `carried-constraints`: the note points at the constraint and at replacing it, never copies its text into a project file.
- `yagni`: no setting or file marker.
- `method-in-the-plugin`: the skill and the glossary say how such a note is taken.

## Edge cases and workarounds

- A project with a `clm.toml` that sets no `upgraded`, or doesn't parse, has the note pending until `--apply` stamps it, as an apply migration would be.
- Existing notes' `done` read their files and are unchanged by `--apply`'s stamp, so printing their `note` lines before the stamp prints the same lines as before.
- `--decline` after `--apply` has stamped the project fails, since the note is no longer pending; nothing is left to decline.
- Fixtures standing for an up-to-date project: `crates/clm/tests/hooks.rs`'s `clm_repo(None)` writes no `clm.toml`, so the note is done there; `shipped_tests.rs`'s `project()` writes one with no `upgraded`, which the requirement has stamped with `latest()`. The corpus test and the capability test *skipping releases* run `--apply`, whose stamp takes the note (builder check).
- No workarounds are known.

## Open questions

None. The builder check found every requirement this change adds relies only on `land/check-lock`, `land/check-slot` and `orchestrator.run/prompt-slot` (pending, now in `#### Needs`) and, for the skill rule, `upgrade/migration-tests-overlap`; no capability test observes a difference, since each runs `--apply` before expecting `up to date`.
