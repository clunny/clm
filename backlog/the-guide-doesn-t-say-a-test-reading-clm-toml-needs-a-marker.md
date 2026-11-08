---
title: "The guide doesn't say a test reading clm.toml needs a clm:reads marker"
kind: deferred
components: [init]
origin: 20261107-the-guide-never-says
---

`20261107-the-guide-never-says` added "Tests that read your specs and docs" to the guide page `ci-and-deployment`, covering the doc-only paths only. Once `land/settings-unchanged-is-checked` is built (it was `d27ea07f`, unbuilt, when this was filed), a land whose `clm.toml` changes only in comments or upgrade notes (`upgraded`, `declined_migrations`) counts as checked too, and runs only the readers. Then add `clm.toml` to that subsection: such a land, and a `clm:reads clm.toml` example for a test that reads the project's `clm.toml`. Saying it before the build would describe behaviour no released `clm` has (`guide-from-docs`).
