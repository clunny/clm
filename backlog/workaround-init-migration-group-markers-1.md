---
title: "Stopped to raise a spec change: upgrade's fresh-project scenario can't hold once init has a migration"
kind: workaround
components: [init]
requirement: init/migration-group-markers@bd95320b
---

Building the migration as stated breaks upgrade's crates/clm-upgrade/tests/corpus.rs::a_fresh_project_has_every_migration_done (upgrade/migration-list, scenario a fresh project is done): clm init stamps init's migration, which upgrade's own list lacks. The test is upgrade's file, so the build stopped and filed backlog/init-s-first-migration-breaks-upgrade-s-fresh-project-scenario for a spec change.
