---
title: "A note that only asks to be read is taken by the upgrade stamp"
date: 2026-11-07
status: accepted
---

`upgrade-migrations` gives each note migration a `done` check over the project's files: the note is pending until the file is right or the project declines it. Some changes tell a project something without asking it to edit any file CLM can check. The first is `20261107-builds-pre-land-checks`: lifting the check lock for builds' pre-land checks means a project's tests must now overlap (`tests-run-concurrently`), and what a project does about that lives in its tests, which CLM can't judge.

**Decision.** Such a note's `done` reads `clm.toml`'s `upgraded`: true once the stamp names the note or a migration after it. A project `clm init` set up since the note is stamped past it and never sees it; an older project sees it in `clm upgrade`, the session-start reminder and `clm upgrade --apply`'s `note <id>` line, and `--apply`'s stamp marks it taken. So `--apply` judges which notes to print before it writes the stamp (`upgrade/upgrade-apply`). Declining still works, and hides it the same way.

**Rejected.**
- *A `done` that is never true, taken only by declining.* "Decline" would mean "read", and the session-start reminder would name the note in every project until someone declined it.
- *A marker the project writes,* such as a comment in `clm.toml` or the check script. It makes the human edit a file to say they read something, and needs `clm init` to write the marker so a fresh project is done, a template change every older project must then migrate (`upgrades-carry-migrations`).
- *An advised apply migration that changes nothing,* as `migration-offer-verify` is advised and has a skill react to it. An apply migration has no note for `clm upgrade --note` to print, and the text is the point.

This refines `upgrade-migrations`.
