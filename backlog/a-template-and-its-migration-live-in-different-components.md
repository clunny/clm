---
title: "A template clm init writes and the upgrade migration that carries its change live in different components"
kind: idea
components: [init, upgrade]
---

## Intent

A requirement that changes a file `clm init` writes must ship the `upgrade` migration in the same land, because the upgrade corpus test fails a land of either half alone (`template-change-ships-its-migration`). The two halves live in different components, so `path-ownership` keeps a transitional exception that lets `init`'s build write into `upgrade`'s files (`coupled-changes-sequence`). Things that must land together belong in one component.

## Open questions

- Should each template's migrations live with the template, in `init`, with `upgrade` keeping only the machinery that applies them?
- Or should `upgrade` own the templates, with `clm init` writing the result of applying every migration to an empty project?
