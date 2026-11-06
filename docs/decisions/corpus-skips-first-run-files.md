---
title: "The upgrade corpus test leaves out the files clm init writes on a first run only"
date: 2026-11-06
status: accepted
---

Refines `upgrade-migrations`' *The corpus test*: "`clm init`'s files equal what it writes today" holds for every file `clm init` writes but two, `.claude/settings.json` and the root `specs/SPEC.md` (`clm/upgrade-corpus`).

**Why these two.** `clm init` writes every other file whenever it is missing, from a fixed template. These two it writes only on a first run, before the repository has a `specs/SPEC.md`, and their text depends on the repository: the root spec's title and `outside` come from the directory and its code, and the allow rules are added to whatever settings Claude Code already wrote. A migration's `old` and `new` texts need a fixed template, so none fits them, and an apply migration would rewrite files the project has owned, and may have edited, since its first run (`land-locally`).

**What the corpus found.** Projects set up before `init-seeds-permissions` hold no `.claude/settings.json`, and their root's `paths` don't list it. That is right for them: the root lists the file because a fresh project has it. The allow rules such a project lacks are reported by `clm init --check-permissions`, which the `build` and `triage` skills run before the first unattended session.

**What it costs.** A change to the root spec template or to the seeded settings is not caught by the corpus test, so `upgrades-carry-migrations` rests on review for those two files. The human chose this split over a migration for each drift; `scripts/clm-check.sh`, a fixed template, got a note migration instead (`upgrade/migration-check-script-from-root`).
