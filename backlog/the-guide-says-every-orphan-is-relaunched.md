---
title: "The guide says every orphan is relaunched"
kind: deferred
components: [orchestrator.loop]
origin: 20261115-the-build-loop-waits
---
Waits on `orchestrator.loop/loop-start` being current at the version `20261115-the-build-loop-waits` gave it.

The glossary's **Orphan** entry (`docs/guide/glossary.md`) says the session that next holds the loop relaunches an orphan on its branch. Once `loop-start` is built, an orphan whose relaunch is refused is ended instead, and the loop tells the human why once with `<name> wasn't relaunched: <reason>`. The entry then says so, and what the person does: mend what the message names and the loop builds the component again, or give the build up with `clm build release <component>` (`method-in-the-plugin`).

The edit changes only `docs/guide/glossary.md`, so it lands as a docs-only branch of its own (`small-change-no-claim`), which also deletes this item.
