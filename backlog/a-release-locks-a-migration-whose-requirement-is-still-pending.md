---
title: "A release locks a migration whose requirement is still pending"
kind: bug
components: [ci, upgrade]
origin: 20261120-wave-2-rest-by-the-rule
---

**The problem.** A release freezes every migration it ships: `.github/scripts/release-prepare.sh` runs `clm upgrade --lock`, and from then on `upgrade/shipped-migrations-frozen` and `init/own-migrations-frozen` refuse any edit to the migration's files. Nothing in the release looks at what is pending. So when a spec change alters what an unshipped migration must do or say, and a release is cut before that change's build lands, the migration is frozen as the old requirement had it and the new requirement can never be built: its build may not edit a shipped migration.

**The case that showed it.** `20261120-wave-2-rest-by-the-rule` adds a sentence to the note of `20261114-proof-is-real` (`upgrade/migration-markers-in-tests`), which no release has shipped. The change says in its Constraints that the requirement is built before the next release, which no build and no release script reads.

**Until it is mended, the human holds it:** no release while `clm pending upgrade` or `clm pending init` lists a requirement of an unshipped migration. Wave 3 holds `ci`'s pass; whether the release refuses, or the lock leaves out a migration whose requirement is pending, is a design for then.
