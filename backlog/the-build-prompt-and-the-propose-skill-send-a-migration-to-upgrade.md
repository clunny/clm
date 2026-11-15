---
title: "The build prompt and the propose skill still place a new migration in upgrade"
kind: deferred
components: [orchestrator.run, plugin]
origin: 20261114-migrations-with-templates
---

Once `20261114-migrations-with-templates` is built, a migration written today is `init`'s (`migrations-live-with-init`), and two texts say otherwise:

- `clm run`'s prompt tells a build whose requirement changes a file `clm init` writes to leave it unbuilt with a question item. It should say that the migration is a requirement of `init`'s, built in the same branch in `init`'s own files, with its tests giving `upgrade`'s engine that migration alone.
- `/clm:propose`'s builder-check step says to add to the change "a requirement of `upgrade` for the migration". It should say a requirement of `init`.

Both are code-only work, each done under its own component's claim, the last deleting this item. It waits on `init/all-migrations` being current.
