---
title: "Stopped to raise a spec change: the list today needs init's first migration, which breaks upgrade's fresh-project scenario"
kind: workaround
components: [init]
requirement: init/all-migrations@da1da9d3
---

Same cause as init/migration-group-markers: all-migrations' scenario the list today names init's first migration, 20261124-a-marker-above-a, whose existence breaks upgrade's crates/clm-upgrade/tests/corpus.rs::a_fresh_project_has_every_migration_done (upgrade/migration-list, a fresh project is done). Its tests stay behind until the spec change backlog/init-s-first-migration-breaks-upgrade-s-fresh-project-scenario asks for.
