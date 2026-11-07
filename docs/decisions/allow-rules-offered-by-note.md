---
title: "Projects set up earlier are offered the unattended allow rules by an advised note migration, and a test ties the seeded list to a migration"
date: 2026-11-07
status: accepted
---

Refines `unattended-steps-run-clm` and `corpus-skips-first-run-files`.

**The gap.** `20261105-unattended-steps-denied-in` had `clm init` seed allow rules for the unattended `clm` commands into `.claude/settings.json` on a first run, and `20261107-slow-tests-after-land` added `clm verify`'s two. Neither shipped a migration, so `clm upgrade` and the session-start reminder never named the rules to a project set up earlier: it learnt of them only when `/clm:build` or `/clm:triage` ran `clm init --check-permissions` before a launch, and in a downstream project builds ran in auto mode without them. `unattended-steps-run-clm` weighed only rewriting the settings on every `init` run; it landed the same day as `upgrade-migrations` and never weighed a migration. The corpus test missed it because `corpus-skips-first-run-files` leaves `.claude/settings.json` out of its comparison.

**An advised note migration** (`upgrade/migration-unattended-allow-rules`). Its `done` is what `clm init --check-permissions` checks, over a copy of the rules frozen with the migration (`upgrade` doesn't depend on `init`). It carries no `old` or `new`, so `clm upgrade --apply` never writes the settings: they are the human's, a protected path, and a rule a project removed on purpose stays removed, the reason `unattended-steps-run-clm` rejected rewriting them. The human adds the rules by hand, or declines the note, and the session-start reminder names it until they do one or the other (`upgrade-migrations`). A project set up before `v0.19.0` has no settings file and a root whose `paths` don't list one, so the note and `/clm:upgrade` say to land that `paths` entry first.

**Alternatives.**
- *An apply migration adding the missing rules.* It would put back a rule a project removed on purpose, and write a file the decision keeps in the human's hands; for a project set up before `v0.19.0` it would also add a file no component owns, failing `clm check`.
- *A note with the `v0.19.0` seeded file as `old`.* It would rewrite only a settings file nobody had touched, which Claude Code's own `enabledPlugins` makes rare, and still leave every older project to the note (`yagni`).
- *Leaving it to `clm init --check-permissions`.* That is what failed: nothing runs it but the build and triage skills, so an attended project never hears of the rules.

**Keeping it honest.** The corpus test declines this migration, since taking it is a hand edit in a file it leaves out. In its place, `clm/upgrade-covers-allow-rules` removes each of `clm init`'s seeded rules in turn from a fresh project and requires `clm upgrade` to list a pending migration, so a change that adds a rule to the seeding fails a land until it ships a migration that offers it. A migration is frozen once shipped (`shipped-migrations-frozen`), so that later change ships a new note migration with the longer list, never an edit to this one.
