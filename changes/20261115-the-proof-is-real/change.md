---
status: proposed
resolves: [the-proof-is-real-upgrade-note-says-a-foreign-change-needs-review]
---

## Intent

The upgrade note `20261114-proof-is-real`, not yet in a release, tells an adopter the wrong thing about `wiring`: that a branch changing files its component doesn't own "needs review". `clm land` refuses such a build branch, and no review lets it through (`land/build-holds-own-files`). The note also doesn't tell a project whose tests share files to give them to their components with `tests` globs before it names `wiring`. Fix the note before the release that first ships it, after which `upgrade/shipped-migrations-frozen` freezes it.
