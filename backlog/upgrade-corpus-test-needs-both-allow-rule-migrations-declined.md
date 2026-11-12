---
title: "upgrade-corpus test needs both allow-rule migrations declined"
kind: bug
components: [upgrade]
---

upgrade/migration-setup-allow-rules lists the fixtures its build must update (project(), clm_repo, skipping releases) but not crates/clm/tests/upgrade.rs's corpus test, which declined only 20261107-projects-set-up-before; adding the new migration made every corpus entry "not up to date". The init build declined both ids in that test as a forced fixture fix. The requirement and clm/upgrade-corpus should name it.
